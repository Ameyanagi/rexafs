use super::structure_depth::{DepthAxis, DepthFrame, DepthOptions, FadeMode, SliceMode};
use super::{button, chip, section_label};
use crate::accessibility::Control;
use crate::app::StudioApp;
use accesskit::{Action, ActionData, Role};
use gpui::{
    Bounds, Context, IntoElement, MouseButton, Pixels, SharedString, Styled, canvas, div, point,
    prelude::*, px, size,
};

#[derive(Clone, Copy)]
pub(crate) enum DepthControl {
    Position,
    Thickness,
    Opacity,
    Strength,
    Radius,
    CenterFocus,
}
impl DepthControl {
    fn index(self) -> usize {
        self as usize
    }
    fn label(self) -> &'static str {
        match self {
            Self::Position => "Position",
            Self::Thickness => "Thickness",
            Self::Opacity => "Opacity",
            Self::Strength => "Fade strength",
            Self::Radius => "Clear radius",
            Self::CenterFocus => "Center focus",
        }
    }
    fn value(self, o: DepthOptions) -> f64 {
        match self {
            Self::Position => o.offset,
            Self::Thickness => o.thickness,
            Self::Opacity => o.opacity,
            Self::Strength => o.strength,
            Self::Radius => o.focus_radius,
            Self::CenterFocus => {
                if o.fade == FadeMode::Center {
                    o.strength
                } else {
                    0.
                }
            }
        }
    }
    fn range(self, extent: f64) -> [f64; 2] {
        match self {
            Self::Position => [-extent, extent],
            Self::Thickness => [0.5, extent * 2.],
            Self::Opacity => [0.1, 1.],
            Self::Strength | Self::CenterFocus => [0., 0.98],
            Self::Radius => [0.5, extent],
        }
    }
    fn percent(self) -> bool {
        matches!(self, Self::Opacity | Self::Strength | Self::CenterFocus)
    }
    fn step(self) -> f64 {
        if self.percent() { 0.05 } else { 0.1 }
    }
    fn set(self, o: &mut DepthOptions, value: f64) {
        match self {
            Self::Position => o.offset = value,
            Self::Thickness => o.thickness = value,
            Self::Opacity => o.opacity = value,
            Self::Strength => o.strength = value,
            Self::Radius => o.focus_radius = value,
            Self::CenterFocus => {
                o.fade = if value > 0. {
                    FadeMode::Center
                } else {
                    FadeMode::Off
                };
                o.strength = value;
            }
        }
    }
}
#[derive(Default)]
pub(crate) struct DepthControls {
    pub open: bool,
    pub options: DepthOptions,
    tracks: [Option<Bounds<Pixels>>; 6],
    drag: Option<DepthControl>,
}
impl StudioApp {
    pub(crate) fn structure_depth_frame(&self) -> Option<DepthFrame> {
        Some(DepthFrame::new(
            self.structure.depth.options,
            self.structure.scene.as_ref()?,
            self.structure.camera,
            self.structure.pick.as_ref().map(|p| p.atom),
        ))
    }
    fn depth_extent(&self) -> f64 {
        self.structure
            .scene
            .as_ref()
            .map(|s| s.extent)
            .unwrap_or(8.)
            .max(4.)
    }
    fn set_depth_value(&mut self, key: DepthControl, value: f64, cx: &mut Context<Self>) {
        if !value.is_finite() {
            return;
        }
        let [lo, hi] = key.range(self.depth_extent());
        let value = if key.percent() {
            (value * 100.).round() / 100.
        } else {
            (value * 10.).round() / 10.
        };
        key.set(&mut self.structure.depth.options, value.clamp(lo, hi));
        cx.notify();
    }
    fn move_depth_slider(&mut self, key: DepthControl, x: Pixels, cx: &mut Context<Self>) {
        if let Some(b) = self.structure.depth.tracks[key.index()] {
            let fraction = ((f32::from(x - b.left()) - 7.)
                / (f32::from(b.size.width) - 14.).max(1.))
            .clamp(0., 1.) as f64;
            let [lo, hi] = key.range(self.depth_extent());
            self.set_depth_value(key, lo + fraction * (hi - lo), cx);
        }
    }
    fn depth_slider_track(&self, key: DepthControl, cx: &mut Context<Self>) -> Control {
        let t = self.theme;
        let value = key.value(self.structure.depth.options);
        let [lo, hi] = key.range(self.depth_extent());
        let fraction = ((value - lo) / (hi - lo)).clamp(0., 1.) as f32;
        let weak = cx.entity().downgrade();
        let track = canvas(
            move |bounds, _, cx| {
                weak.update(cx, |this, _| {
                    this.structure.depth.tracks[key.index()] = Some(bounds)
                })
                .ok();
            },
            move |b, _, w, _| {
                let x = f32::from(b.left()) + 7.;
                let width = f32::from(b.size.width) - 14.;
                let y = f32::from(b.center().y);
                for (length, color) in [(width, t.border), (width * fraction, t.accent)] {
                    w.paint_quad(gpui::quad(
                        Bounds::new(point(px(x), px(y - 2.)), size(px(length.max(1.)), px(4.))),
                        gpui::Corners::all(px(2.)),
                        color,
                        gpui::Edges::all(px(0.)),
                        color,
                        gpui::BorderStyle::Solid,
                    ));
                }
                w.paint_quad(gpui::quad(
                    Bounds::new(
                        point(px(x + width * fraction - 5.), px(y - 5.)),
                        size(px(10.), px(10.)),
                    ),
                    gpui::Corners::all(px(5.)),
                    t.accent,
                    gpui::Edges::all(px(1.)),
                    t.text,
                    gpui::BorderStyle::Solid,
                ));
            },
        )
        .size_full();
        let weak = cx.entity().downgrade();
        Control::new(div().id(("depth-slider", key.index())), key.label(), Role::Slider)
            .numeric(value, lo, hi, key.step())
            .value(if key.percent() { format!("{:.0}%", value * 100.) } else { format!("{value:.1} Å") })
            .description(if matches!(key, DepthControl::CenterFocus) {
                "Fade the outer cluster around the inspection center, the absorber by default. Selected scattering paths stay clear."
            } else { key.label() })
            .flex_1()
            .min_w(px(72.))
            .h(px(24.))
            .rounded_sm()
            .border_1()
            .border_color(gpui::transparent_black())
            .focus(|d| d.border_color(t.accent))
            .cursor(gpui::CursorStyle::ResizeLeftRight)
            .child(track)
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev: &gpui::MouseDownEvent, _, cx| {
                this.structure.depth.drag = Some(key);
                this.move_depth_slider(key, ev.position.x, cx);
            }))
            .on_mouse_move(cx.listener(move |this, ev: &gpui::MouseMoveEvent, _, cx| {
                if ev.pressed_button == Some(MouseButton::Left)
                    && this.structure.depth.drag.is_some_and(|k| k.index() == key.index()) {
                    this.move_depth_slider(key, ev.position.x, cx);
                    cx.stop_propagation();
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.structure.depth.drag = None;
                cx.stop_propagation();
            }))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, _| this.structure.depth.drag = None))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                let current = key.value(this.structure.depth.options);
                let [lo, hi] = key.range(this.depth_extent());
                let value = match event.keystroke.key.as_str() {
                    "left" | "down" => current - key.step(),
                    "right" | "up" => current + key.step(),
                    "home" => lo,
                    "end" => hi,
                    _ => return,
                };
                this.set_depth_value(key, value, cx);
                cx.stop_propagation();
            }))
            .on_request(vec![Action::Increment, Action::Decrement, Action::SetValue], move |action, data, _, cx| {
                weak.update(cx, |this, cx| {
                    let current = key.value(this.structure.depth.options);
                    let value = match (action, data) {
                        (Action::Increment, _) => current + key.step(),
                        (Action::Decrement, _) => current - key.step(),
                        (Action::SetValue, Some(ActionData::NumericValue(value))) => value,
                        _ => return,
                    };
                    this.set_depth_value(key, value, cx);
                }).ok();
            })
    }
    pub(crate) fn center_focus_slider(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let value = DepthControl::CenterFocus.value(self.structure.depth.options);
        div()
            .flex()
            .items_center()
            .gap_2()
            .flex_1()
            .min_w(px(215.))
            .max_w(px(340.))
            .text_size(px(11.))
            .text_color(self.theme.text_muted)
            .child("Center focus")
            .child(self.depth_slider_track(DepthControl::CenterFocus, cx))
            .child(
                div()
                    .w(px(30.))
                    .text_right()
                    .child(format!("{:.0}%", value * 100.)),
            )
    }
    fn depth_slider(&self, key: DepthControl, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let value = key.value(self.structure.depth.options);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(key.label())
                    .child(if key.percent() {
                        format!("{:.0}%", value * 100.)
                    } else {
                        format!("{value:.1} Å")
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        button(
                            &t,
                            SharedString::from(format!("depth-minus-{}", key.index())),
                            "−",
                            false,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_depth_value(
                                key,
                                key.value(this.structure.depth.options) - key.step(),
                                cx,
                            )
                        })),
                    )
                    .child(self.depth_slider_track(key, cx))
                    .child(
                        button(
                            &t,
                            SharedString::from(format!("depth-plus-{}", key.index())),
                            "+",
                            false,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_depth_value(
                                key,
                                key.value(this.structure.depth.options) + key.step(),
                                cx,
                            )
                        })),
                    ),
            )
    }
    pub(crate) fn structure_depth_panel(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let o = self.structure.depth.options;
        let center = self
            .structure
            .scene
            .as_ref()
            .and_then(|s| {
                s.atoms
                    .iter()
                    .find(|a| {
                        a.index.is_some() && a.index == self.structure.pick.as_ref().map(|p| p.atom)
                    })
                    .or_else(|| s.atoms.iter().find(|a| a.absorber))
            })
            .map(|a| {
                format!(
                    "{} · atom {}{}",
                    crate::structure::element_symbol(a.z),
                    a.index.unwrap_or(0),
                    if a.absorber { " · absorber" } else { "" }
                )
            })
            .unwrap_or_else(|| "Select a structure".into());
        let mut mode = div().flex().flex_wrap().gap_1();
        for (value, label) in [
            (SliceMode::Off, "Off"),
            (SliceMode::Slab, "Slab"),
            (SliceMode::Cutaway, "Cutaway"),
        ] {
            mode = mode.child(
                chip(
                    &t,
                    SharedString::from(format!("slice-{label}")),
                    label,
                    o.slice == value,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.structure.depth.options.slice = value;
                    cx.notify();
                })),
            );
        }
        let mut axis = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child("Normal");
        for value in [DepthAxis::View, DepthAxis::X, DepthAxis::Y, DepthAxis::Z] {
            axis = axis.child(
                chip(
                    &t,
                    SharedString::from(format!("slice-axis-{}", value.label())),
                    value.label(),
                    o.axis == value,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.structure.depth.options.axis = value;
                    cx.notify();
                })),
            );
        }
        let mut fading = div().flex().flex_wrap().gap_1();
        for (value, label) in [
            (FadeMode::Off, "None"),
            (FadeMode::Depth, "Back → front"),
            (FadeMode::Center, "Around center"),
        ] {
            fading = fading.child(
                chip(
                    &t,
                    SharedString::from(format!("depth-fade-{label}")),
                    label,
                    o.fade == value,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.structure.depth.options.fade = value;
                    cx.notify();
                })),
            );
        }
        div().flex().flex_col().gap_3().p_3().text_size(px(12.))
            .child(div().flex().items_center().justify_between()
                .child(button(&t,"depth-close","← Back",false).on_click(cx.listener(|this,_,_,cx|{this.structure.depth.open=false;cx.notify();})))
                .child(button(&t,"depth-reset","Reset view effects",false).on_click(cx.listener(|this,_,_,cx|{this.structure.depth.options=DepthOptions::default();cx.notify();}))))
            .child(section_label(&t,"Coordination center"))
            .child(center)
            .child(div().text_color(t.text_muted).child("Click a visible atom to move the inspection center. The calculation absorber stays unchanged."))
            .child(div().flex().flex_wrap().gap_1()
                .child(button(&t,"slice-recenter","Through center",false).on_click(cx.listener(|this,_,_,cx|{this.structure.depth.options.offset=0.;cx.notify();})))
                .child(button(&t,"slice-absorber","Use absorber",false).on_click(cx.listener(|this,_,_,cx|{this.structure.pick=None;this.structure.depth.options.offset=0.;this.rebuild_structure_plot(cx);cx.notify();}))))
            .child(section_label(&t,"Slice"))
            .child(mode)
            .when(o.slice!=SliceMode::Off,|d|d.child(axis)
                .child(div().text_color(t.text_muted).child(if o.axis==DepthAxis::View { "View follows rotation: − farther · + nearer. Cutaway removes the nearer side." } else { "X/Y/Z are fixed Cartesian normals in Å, not fractional crystal axes. Cutaway removes the + side." }))
                .child(self.depth_slider(DepthControl::Position,cx))
                .when(o.slice==SliceMode::Slab,|d|d.child(self.depth_slider(DepthControl::Thickness,cx)))
                .child(chip(&t,"slice-ghost","Faint outside context",o.ghost).on_click(cx.listener(|this,_,_,cx|{this.structure.depth.options.ghost=!this.structure.depth.options.ghost;cx.notify();}))))
            .child(section_label(&t,"Transparency"))
            .child(self.depth_slider(DepthControl::Opacity,cx))
            .child(fading)
            .when(o.fade!=FadeMode::Off,|d| d.child(self.depth_slider(DepthControl::Strength,cx)))
            .when(o.fade==FadeMode::Center,|d|d.child(self.depth_slider(DepthControl::Radius,cx)).child(div().text_color(t.text_muted).child("The center and neighbours within this radius stay clear. More distant atoms fade smoothly.")))
            .when(o.fade==FadeMode::Depth,|d|d.child(div().text_color(t.text_muted).child("Farther atoms fade; nearer atoms stay opaque. This cue follows the camera.")))
            .child(div().border_t_1().border_color(t.border).pt_2().text_color(t.text_muted).child("View only · FEFF cluster, scattering paths and fit remain unchanged. Slices select atom centers and clip bonds/faces."))
    }
}
