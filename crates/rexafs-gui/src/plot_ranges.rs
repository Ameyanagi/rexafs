//! Display-only axis limits shared by the desktop's two-dimensional plots.
//!
//! Each endpoint is automatic by default. Automatic endpoints use the plot's
//! natural bounds (including its padding and physical-domain defaults), and
//! are resolved again after data changes. These limits never trim source data
//! or change normalization, transform or fitting ranges.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use futures::{StreamExt, channel::mpsc};
use gpui::{
    App, AppContext, Bounds, Context, Entity, Global, IntoElement, KeyDownEvent, ParentElement,
    Render, Styled, TitlebarOptions, WeakEntity, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, size,
};
use ruviz::core::{InteractiveChangeRevision, InteractivePlotSession, ViewportPoint, ViewportRect};
use ruviz_gpui::{GpuiContextMenuConfig, GpuiContextMenuItem, IntoPlotSession, RuvizPlot};

use crate::{theme::Theme, widgets::text_input::TextInput};

/// Optional numeric endpoints, in the same units as the displayed axes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Limits {
    x: [Option<f64>; 2],
    y: [Option<f64>; 2],
}

impl Limits {
    fn parse(fields: [&str; 4]) -> Result<Self, String> {
        let mut values = [None; 4];
        for (i, text) in fields.iter().enumerate() {
            let text = text.trim();
            if text.is_empty() || text.eq_ignore_ascii_case("auto") {
                continue;
            }
            values[i] = Some(
                text.parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| {
                        format!(
                            "{} must be a finite number or Auto.",
                            ["X minimum", "X maximum", "Y minimum", "Y maximum"][i]
                        )
                    })?,
            );
        }
        let limits = Self {
            x: [values[0], values[1]],
            y: [values[2], values[3]],
        };
        for (name, range) in [("X", limits.x), ("Y", limits.y)] {
            if let [Some(min), Some(max)] = range
                && min >= max
            {
                return Err(format!("{name} minimum must be less than its maximum."));
            }
        }
        Ok(limits)
    }

    fn bounds(self, session: &InteractivePlotSession) -> Result<ViewportRect, String> {
        let base = session.view_bounds_snapshot();
        let x = resolve(self.x, [base.base_bounds.min.x, base.base_bounds.max.x]);
        let y = resolve(self.y, [base.base_bounds.min.y, base.base_bounds.max.y]);
        for (name, range) in [("X", x), ("Y", y)] {
            let span = (range[1] - range[0]).abs();
            if !range.iter().all(|v| v.is_finite()) || !span.is_finite() || span == 0. {
                return Err(format!("{name} range must have a finite, nonzero span."));
            }
        }
        base.x_scale
            .validate_range(x[0], x[1])
            .map_err(|e| format!("X axis: {e}"))?;
        base.y_scale
            .validate_range(y[0], y[1])
            .map_err(|e| format!("Y axis: {e}"))?;
        Ok(ViewportRect {
            min: ViewportPoint::new(x[0], y[0]),
            max: ViewportPoint::new(x[1], y[1]),
        })
    }
}

fn resolve(limits: [Option<f64>; 2], base: [f64; 2]) -> [f64; 2] {
    let reversed = base[0] > base[1];
    let natural = [base[0].min(base[1]), base[0].max(base[1])];
    let mut range = [
        limits[0].unwrap_or(natural[0]),
        limits[1].unwrap_or(natural[1]),
    ];
    // An explicit bound can exclude all current data. Keep it exact and give
    // the automatic side a nonzero span instead of reversing the axis.
    if range[0] >= range[1] {
        let span = (natural[1] - natural[0])
            .max(range[0].abs() * 0.05)
            .max(1e-12);
        if limits[1].is_none() {
            range[1] = range[0] + span;
        } else if limits[0].is_none() {
            range[0] = range[1] - span;
        }
    }
    if reversed {
        range.swap(0, 1);
    }
    range
}

#[derive(Default)]
struct RangeConfig {
    limits: Limits,
    views: Vec<(WeakEntity<RuvizPlot>, std::rc::Weak<RefCell<ViewState>>)>,
}

/// Apply to every still-visible version of this logical plot, including a
/// replacement that arrived while the editor was open.
fn apply_ranges(
    config: &Rc<RefCell<RangeConfig>>,
    limits: Limits,
    cx: &mut App,
) -> Result<(), String> {
    let views: Vec<_> = config
        .borrow()
        .views
        .iter()
        .filter_map(|(plot, state)| Some((plot.upgrade()?, state.upgrade()?)))
        .collect();
    if views.is_empty() {
        return Err("This plot has closed. Reopen Axis range on the current plot.".into());
    }
    for (_, state) in &views {
        constrained_plot(&state.borrow().natural, limits)?;
    }
    for (plot, state) in views {
        plot.update(cx, |plot, cx| {
            state.borrow_mut().refresh(plot, limits, true, cx)
        })?;
    }
    let mut config = config.borrow_mut();
    config.limits = limits;
    config
        .views
        .retain(|(plot, state)| plot.upgrade().is_some() && state.strong_count() > 0);
    Ok(())
}

#[derive(Default)]
struct RangeRegistry(BTreeMap<String, Rc<RefCell<RangeConfig>>>);
impl Global for RangeRegistry {}

/// Builder that adds an Axis range context-menu command without changing plot
/// pointer events, export, zoom or pan. A range key retains limits when a Live
/// or Series plot entity is rebuilt; callers must include the plotted quantity
/// so unrelated parameters with different units never share limits.
pub(crate) struct Builder<P>(ruviz_gpui::RuvizPlotBuilder<P>, Option<String>);

pub(crate) fn plot_builder<P: IntoPlotSession + 'static>(plot: P) -> Builder<P> {
    Builder(ruviz_gpui::plot_builder(plot), None)
}

impl<P: IntoPlotSession + 'static> Builder<P> {
    pub(crate) fn interactive(mut self) -> Self {
        self.0 = self.0.interactive();
        self
    }
    pub(crate) fn interaction_options(mut self, options: ruviz_gpui::InteractionOptions) -> Self {
        self.0 = self.0.interaction_options(options);
        self
    }
    pub(crate) fn presentation(mut self, mode: ruviz_gpui::PresentationMode) -> Self {
        self.0 = self.0.presentation(mode);
        self
    }
    pub(crate) fn range_key(mut self, key: String) -> Self {
        self.1 = Some(key);
        self
    }

    pub(crate) fn build<V: 'static>(self, cx: &mut Context<V>) -> Entity<RuvizPlot> {
        let limits = if let Some(key) = self.1 {
            cx.default_global::<RangeRegistry>()
                .0
                .entry(key)
                .or_default()
                .clone()
        } else {
            Rc::default()
        };
        let (sender, mut receiver) = mpsc::unbounded();
        let entity = self
            .0
            .context_menu(GpuiContextMenuConfig {
                show_reset_view: false,
                // Keep the menu within a compact Live plot's height. The
                // library places this menu inside the plot's input region.
                show_set_home_view: false,
                show_go_to_home_view: false,
                show_copy_cursor_coordinates: false,
                show_copy_visible_bounds: false,
                custom_items: vec![
                    GpuiContextMenuItem::new("axis-range", "Axis range…"),
                    GpuiContextMenuItem::new("auto-range", "Reset axes to Auto"),
                ],
                ..Default::default()
            })
            .on_context_menu_action(move |action| {
                let _ = sender.unbounded_send(action.action_id);
                Ok(())
            })
            .build(cx);

        let state = Rc::new(RefCell::new(ViewState::new(
            entity.read(cx).interactive_session(),
        )));
        {
            let mut config = limits.borrow_mut();
            // Live replaces plot entities for each accepted update. Retain
            // only current views, so a long acquisition does not accumulate
            // weak references to every earlier frame.
            config
                .views
                .retain(|(plot, state)| plot.upgrade().is_some() && state.strong_count() > 0);
            config
                .views
                .push((entity.downgrade(), Rc::downgrade(&state)));
        }
        // Apply a remembered range before the first render of a replacement
        // plot, including changes to the selected error-bar display.
        entity.update(cx, |plot, cx| {
            let _ = state
                .borrow_mut()
                .refresh(plot, limits.borrow().limits, false, cx);
        });
        let watched = limits.clone();
        let view_state = state.clone();
        cx.observe(&entity, move |_, entity, cx| {
            entity.update(cx, |plot, cx| {
                let _ = view_state
                    .borrow_mut()
                    .refresh(plot, watched.borrow().limits, false, cx);
            });
        })
        .detach();

        entity.update(cx, |_, cx| {
            cx.spawn(async move |plot, cx| {
                while let Some(action) = receiver.next().await {
                    let Some(plot) = plot.upgrade() else {
                        break;
                    };
                    let limits = limits.clone();
                    cx.update(|cx| {
                        if action == "auto-range" {
                            let _ = apply_ranges(&limits, Limits::default(), cx);
                        } else if action == "axis-range" {
                            open_editor(plot, limits, cx);
                        }
                    });
                }
            })
            .detach();
        });
        entity
    }
}

/// Keep the unconstrained plot so Auto can recover after a manual range.
/// A replacement from the host becomes the new source. Shared observable arrays
/// are retained, and a changed source revision resolves the automatic endpoints
/// before constructing the next display plot. Pan and zoom do not rebuild it.
struct ViewState {
    natural: InteractivePlotSession,
    displayed: usize,
    revision: Option<InteractiveChangeRevision>,
    applied: Limits,
}

fn identity(session: &InteractivePlotSession) -> usize {
    session.prepared_plot() as *const _ as usize
}

impl ViewState {
    fn new(session: &InteractivePlotSession) -> Self {
        Self {
            natural: session.clone(),
            displayed: identity(session),
            revision: None,
            applied: Limits::default(),
        }
    }

    fn refresh(
        &mut self,
        view: &mut RuvizPlot,
        limits: Limits,
        force: bool,
        cx: &mut Context<RuvizPlot>,
    ) -> Result<(), String> {
        let replaced = identity(view.interactive_session()) != self.displayed;
        if replaced {
            self.natural = view.interactive_session().clone();
            self.displayed = identity(&self.natural);
            self.revision = None;
        }
        let revision = self.natural.change_revision();
        if !force && self.revision == Some(revision) && limits == self.applied {
            return Ok(());
        }
        self.revision = Some(revision);
        if limits == Limits::default() && self.applied == limits {
            if force {
                view.reset_view(cx);
            }
            return Ok(());
        }
        let plot = constrained_plot(&self.natural, limits)?;
        let visibility: Vec<_> = (0..view.interactive_session().series_count())
            .map(|i| view.interactive_session().series_visible(i))
            .collect();
        view.set_plot(plot, cx);
        for (i, visible) in visibility.into_iter().enumerate() {
            view.interactive_session().set_series_visible(i, visible);
        }
        self.displayed = identity(view.interactive_session());
        self.applied = limits;
        cx.emit(RangesChanged);
        Ok(())
    }
}

fn constrained_plot(
    source: &InteractivePlotSession,
    limits: Limits,
) -> Result<ruviz::prelude::Plot, String> {
    let mut plot = source.prepared_plot().plot().clone();
    // Constructing a session resolves current observable arrays without drawing
    // a second image. This also avoids inheriting an old pan/zoom viewport.
    let natural = plot.clone().into_plot_session();
    let bounds = limits.bounds(&natural)?;
    if limits.x != [None; 2] {
        plot = plot.xlim(bounds.min.x, bounds.max.x);
    }
    if limits.y != [None; 2] {
        plot = plot.ylim(bounds.min.y, bounds.max.y);
    }
    Ok(plot)
}

/// Plot-session replacement notification for host cursor overlays and linked axes.
pub(crate) struct RangesChanged;
impl gpui::EventEmitter<RangesChanged> for RuvizPlot {}

struct Editor {
    limits: Rc<RefCell<RangeConfig>>,
    fields: [Entity<TextInput>; 4],
    error: Option<String>,
    theme: Theme,
}

fn open_editor(plot: Entity<RuvizPlot>, limits: Rc<RefCell<RangeConfig>>, cx: &mut App) {
    let color = plot.read(cx).prepared_plot().plot().get_theme().background;
    let theme = if color.r as u32 + color.g as u32 + color.b as u32 > 384 {
        Theme::light()
    } else {
        Theme::dark()
    };
    let bounds = Bounds::centered(None, size(px(410.), px(268.)), cx);
    let opened = cx.open_window(
        WindowOptions {
            show: false,
            focus: false,
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Axis range".into()),
                ..Default::default()
            }),
            window_min_size: Some(size(px(410.), px(268.))),
            ..Default::default()
        },
        move |window, cx| {
            crate::accessibility::install(window, cx);
            cx.new(|cx| {
                let current = limits.borrow().limits;
                let values = [current.x[0], current.x[1], current.y[0], current.y[1]];
                let fields = std::array::from_fn(|i| {
                    cx.new(|cx| {
                        let mut field = TextInput::new(
                            "Auto",
                            values[i].map(|v| v.to_string()).unwrap_or_default(),
                            theme,
                            cx,
                        );
                        field.set_accessible_name(
                            ["X minimum", "X maximum", "Y minimum", "Y maximum"][i],
                        );
                        field
                    })
                });
                Editor {
                    limits,
                    fields,
                    error: None,
                    theme,
                }
            })
        },
    );
    if let Ok(handle) = opened {
        handle
            .update(cx, |_, window, _| crate::accessibility::show(window))
            .ok();
    }
}

impl Editor {
    fn apply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let result = Limits::parse(std::array::from_fn(|i| self.fields[i].read(cx).text()))
            .and_then(|limits| apply_ranges(&self.limits, limits, cx));
        match result {
            Ok(()) => window.remove_window(),
            Err(error) => {
                self.error = Some(error);
                cx.notify();
            }
        }
    }
}

fn button(
    theme: Theme,
    id: &'static str,
    label: &'static str,
    primary: bool,
) -> crate::accessibility::Control {
    crate::accessibility::Control::new(div().id(id), label, accesskit::Role::Button)
        .tab_index(0)
        .key_context("Control")
        .px_3()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .cursor_pointer()
        .bg(if primary { theme.accent } else { theme.raised })
        .text_color(if primary { theme.bg } else { theme.text })
        .child(label)
}

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.theme;
        let content =
            div()
                .size_full()
                .bg(t.surface)
                .text_color(t.text)
                .text_size(px(12.))
                .p_4()
                .flex()
                .flex_col()
                .gap_3()
                .on_key_down(cx.listener(|editor, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        window.remove_window();
                    } else if event.keystroke.key == "enter" {
                        editor.apply(window, cx);
                    }
                }))
                .child(
                    div()
                        .text_color(t.text_muted)
                        .child("Blank or Auto follows the data. Use the plot’s units."),
                )
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(div().w(px(24.)))
                        .child(div().flex_1().child("Minimum"))
                        .child(div().flex_1().child("Maximum")),
                )
                .children([("X", 0), ("Y", 2)].map(|(label, i)| {
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(div().w(px(24.)).child(label))
                        .child(div().flex_1().min_w_0().child(self.fields[i].clone()))
                        .child(div().flex_1().min_w_0().child(self.fields[i + 1].clone()))
                }))
                .child(
                    div()
                        .h(px(32.))
                        .text_size(px(11.))
                        .text_color(t.error)
                        .child(self.error.clone().unwrap_or_default()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(button(t, "auto", "All Auto", false).on_click(cx.listener(
                            |editor, _, _, cx| {
                                for field in &editor.fields {
                                    field.update(cx, |field, cx| field.set_text("", cx));
                                }
                                editor.error = None;
                                cx.notify();
                            },
                        )))
                        .child(div().flex_1())
                        .child(
                            button(t, "cancel", "Cancel", false)
                                .on_click(|_, window, _| window.remove_window()),
                        )
                        .child(button(t, "apply", "Apply", true).on_click(
                            cx.listener(|editor, _, window, cx| editor.apply(window, cx)),
                        )),
                );
        crate::accessibility::root(content, "Axis range")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviz::prelude::Plot;

    #[test]
    fn mixed_limits_follow_new_data_and_preserve_zero() {
        let limits = Limits::parse(["auto", "", "0", "Auto"]).unwrap();
        for top in [2., 8.] {
            let session = Plot::new().line(&[1., 2.], &[1., top]).into_plot_session();
            let bounds = limits.bounds(&session).unwrap();
            assert_eq!(bounds.min.y, 0.);
            assert!(bounds.max.y >= top);
            assert_eq!(
                bounds.max.y,
                session.view_bounds_snapshot().base_bounds.max.y
            );
            let displayed = constrained_plot(&session, limits)
                .unwrap()
                .into_plot_session();
            assert_eq!(displayed.view_bounds_snapshot().visible_bounds.min.y, 0.);
        }
        assert_eq!(resolve([Some(10.), None], [1., 2.]), [10., 11.]);
        assert_eq!(resolve([None, Some(-1.)], [1., 2.]), [-2., -1.]);
        assert_eq!(resolve([Some(0.), Some(5.)], [10., 1.]), [5., 0.]);
    }

    #[test]
    fn explicit_zero_is_not_limited_by_interactive_zoom_and_auto_recovers() {
        let data = ruviz::data::Observable::new(vec![2.541, 2.542]);
        let source = Plot::new()
            .line_source(&[1., 2.], data.clone())
            .into_plot_session();
        let limits = Limits::parse(["", "", "0", ""]).unwrap();
        let bounded = constrained_plot(&source, limits)
            .unwrap()
            .into_plot_session();
        let bounds = bounded.view_bounds_snapshot().base_bounds;
        assert_eq!(bounds.min.y, 0.);
        assert!(bounds.max.y >= 2.542);
        data.set(vec![2.541, 4.]);
        let updated = constrained_plot(&source, limits)
            .unwrap()
            .into_plot_session();
        assert_eq!(updated.view_bounds_snapshot().base_bounds.min.y, 0.);
        assert!(updated.view_bounds_snapshot().base_bounds.max.y >= 4.);
        let auto = constrained_plot(&source, Limits::default())
            .unwrap()
            .into_plot_session();
        assert!(auto.view_bounds_snapshot().base_bounds.min.y > 2.);
    }

    #[test]
    fn rejects_invalid_limits_before_changing_the_plot() {
        for fields in [
            ["NaN", "", "", ""],
            ["", "inf", "", ""],
            ["2", "1", "", ""],
            ["", "", "0", "0"],
        ] {
            assert!(Limits::parse(fields).is_err());
        }
        let session = Plot::new()
            .line(&[1., 2.], &[1., 2.])
            .xscale(ruviz::axes::AxisScale::Log)
            .into_plot_session();
        assert!(
            Limits::parse(["0", "", "", ""])
                .unwrap()
                .bounds(&session)
                .is_err()
        );
    }
}
