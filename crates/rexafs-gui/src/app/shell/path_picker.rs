//! Path picker: FEFF paths grouped by coordination shell, single scattering
//! first, multiple scattering folded under each shell. Selection (the
//! checkbox) is what the fit uses; presets and a text filter make choosing
//! a handful of shells out of dozens of files a two-click job.

use gpui::{
    ClickEvent, Context, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px,
};
use rexafs::xafs::structure::{ShellInfo, select_by, shells_of};

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{MONO, button, chip, controls::icon, section_label};
use crate::app::StudioApp;
use crate::fitting::{FitPathSpec, PathMeta, path_meta, relink_paths};
use crate::icons::Icon;

/// Selection presets shown above the picker.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PathPreset {
    FirstShell,
    ToFitRmax,
    Important,
    All,
    None,
}

impl PathPreset {
    pub(crate) const ALL: [PathPreset; 5] = [
        PathPreset::FirstShell,
        PathPreset::ToFitRmax,
        PathPreset::Important,
        PathPreset::All,
        PathPreset::None,
    ];

    fn label(self) -> &'static str {
        match self {
            PathPreset::FirstShell => "First shell",
            PathPreset::ToFitRmax => "To fit R max",
            PathPreset::Important => "Importance ≥ 10 %",
            PathPreset::All => "All",
            PathPreset::None => "None",
        }
    }

    fn id(self) -> &'static str {
        match self {
            PathPreset::FirstShell => "pp-first",
            PathPreset::ToFitRmax => "pp-rmax",
            PathPreset::Important => "pp-imp",
            PathPreset::All => "pp-all",
            PathPreset::None => "pp-none",
        }
    }
}

/// Variable names an expression refers to (`reff` and numbers included; callers
/// only compare these against defined variables).
fn symbols(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|s| s.starts_with(|c: char| c.is_alphabetic() || c == '_'))
}

fn path_symbols(spec: &FitPathSpec) -> BTreeSet<String> {
    [
        &spec.s02,
        &spec.degen,
        &spec.e0,
        &spec.sigma2,
        &spec.deltar,
        &spec.ei,
        &spec.third,
        &spec.fourth,
    ]
    .into_iter()
    .flat_map(|text| symbols(text).map(str::to_owned))
    .collect()
}

impl StudioApp {
    /// A source whose FEFF files could not be read, e.g. a project saved with
    /// linked files after rexafs's FEFF workspace was cleaned.
    pub(crate) fn path_source_missing(&self, source: &Path) -> bool {
        self.fit_paths
            .iter()
            .any(|p| p.spec.file.parent() == Some(source) && p.meta.is_none())
    }

    /// Rows capture their index, so a changed list is rebuilt from its specs.
    fn rebuild_path_rows(&mut self, specs: Vec<FitPathSpec>, cx: &mut Context<Self>) {
        self.fit_paths.clear();
        for spec in specs {
            self.add_path_row(spec, cx);
        }
        self.refresh_path_infos();
        self.structure.selected = None;
        self.structure.multi.clear();
        self.structure.ms_open.clear();
        self.structure.path_leg = None;
        self.structure.paths.clear();
        self.refresh_structure(cx);
        self.fit_model_changed(cx);
    }

    /// Remove a source's paths, and the variables only those paths used.
    pub(crate) fn remove_path_source(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        let number = self.path_sources().iter().position(|s| *s == source);
        let (removed, kept): (Vec<_>, Vec<_>) = self
            .fit_paths
            .iter()
            .map(|p| p.spec.clone())
            .partition(|spec| spec.file.parent() == Some(source.as_path()));
        for dataset in &mut self.joint.config.datasets {
            dataset
                .paths
                .retain(|p| p.parent() != Some(source.as_path()));
            dataset
                .expressions
                .retain(|p, _| p.parent() != Some(source.as_path()));
        }
        // Keep the variables the remaining model refers to, directly or
        // through a kept variable's expression; the rest no longer affect the fit.
        let mut used: BTreeSet<String> = kept.iter().flat_map(path_symbols).collect();
        used.extend(
            self.joint
                .config
                .datasets
                .iter()
                .flat_map(|d| d.expressions.values().flat_map(path_symbols)),
        );
        loop {
            let more: BTreeSet<String> = self
                .fit_vars
                .iter()
                .filter(|v| used.contains(&v.spec.name))
                .filter_map(|v| v.spec.expr.as_deref())
                .flat_map(symbols)
                .filter(|name| !used.contains(*name))
                .map(str::to_owned)
                .collect();
            if more.is_empty() {
                break;
            }
            used.extend(more);
        }
        self.fit_vars.retain(|v| used.contains(&v.spec.name));
        if self.structure.source_filter.as_ref() == Some(&source) {
            self.structure.source_filter = None;
        }
        self.structure.remove_source = None;
        let n = removed.len();
        self.rebuild_path_rows(kept, cx);
        self.structure.source_filter = self
            .structure
            .source_filter
            .clone()
            .or_else(|| self.path_sources().into_iter().next());
        self.status = format!(
            "Removed source {} · {}",
            number.map_or(String::new(), |i| (i + 1).to_string()),
            crate::text::plural(n, "path")
        )
        .into();
    }

    /// After a path calculation, re-link a source whose files are missing when
    /// the new calculation reproduces it. A calculation started by a source's
    /// **Recalculate** re-links only that source. Returns the number of
    /// re-linked paths.
    pub(crate) fn relink_missing_source(
        &mut self,
        new: &[PathBuf],
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        let target = self.structure.recalculate_source.take();
        let recorded: BTreeMap<PathBuf, PathMeta> = self
            .fit_history
            .iter()
            .flat_map(|h| &h.path_details)
            .filter_map(|d| {
                Some((
                    d.file.clone(),
                    PathMeta {
                        reff: d.reff?,
                        degen: d.degeneracy?,
                        nleg: d.nleg?,
                    },
                ))
            })
            .collect();
        let pairs = self
            .path_sources()
            .into_iter()
            .filter(|s| self.path_source_missing(s))
            .filter(|s| target.as_ref().is_none_or(|t| t == s))
            .find_map(|source| {
                let missing: Vec<_> = self
                    .fit_paths
                    .iter()
                    .filter(|p| p.spec.file.parent() == Some(source.as_path()))
                    .map(|p| (p.spec.file.clone(), recorded.get(&p.spec.file).copied()))
                    .collect();
                relink_paths(&missing, new, path_meta)
            })?;
        let map: BTreeMap<_, _> = pairs.into_iter().collect();
        let relink = |p: &PathBuf| map.get(p).cloned().unwrap_or_else(|| p.clone());
        let specs: Vec<_> = self
            .fit_paths
            .iter()
            .map(|p| FitPathSpec {
                file: relink(&p.spec.file),
                ..p.spec.clone()
            })
            .collect();
        for dataset in &mut self.joint.config.datasets {
            for p in &mut dataset.paths {
                *p = relink(p);
            }
            dataset.expressions = std::mem::take(&mut dataset.expressions)
                .into_iter()
                .map(|(p, spec)| {
                    let file = relink(&spec.file);
                    (relink(&p), FitPathSpec { file, ..spec })
                })
                .collect();
        }
        self.rebuild_path_rows(specs, cx);
        Some(map.len())
    }

    /// Run a missing source's own `feff.inp` again.
    pub(crate) fn recalculate_path_source(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if self.feff_running {
            self.status = "Wait for the path calculation to finish.".into();
            return;
        }
        if source.join("feff.inp").is_file() {
            self.set_feff_workspace(Some(source.clone()));
            self.run_feff10_now(cx);
            if self.feff_running {
                self.structure.recalculate_source = Some(source);
            }
            return;
        }
        self.status = "This calculation's input is missing too. Choose the structure and \
                       calculate again; matching paths replace the missing ones."
            .into();
        self.set_fit_step(super::fit_workspace::FitStep::Structure, cx);
    }

    fn path_in_visible_source(&self, i: usize) -> bool {
        self.structure.source_filter.as_ref().is_none_or(|source| {
            self.fit_paths
                .get(i)
                .is_some_and(|p| p.spec.file.parent() == Some(source.as_path()))
        })
    }
    /// Apply a preset to the visible calculation, preserving other sources.
    pub(crate) fn apply_path_preset(&mut self, preset: PathPreset, cx: &mut Context<Self>) {
        let infos: Vec<_> = self
            .fit_path_infos
            .iter()
            .filter(|p| self.path_in_visible_source(p.index))
            .cloned()
            .collect();
        let selected: Vec<usize> = match preset {
            PathPreset::FirstShell => shells_of(&infos)
                .first()
                .map(|s| s.paths.clone())
                .unwrap_or_default(),
            PathPreset::ToFitRmax => select_by(&infos, self.fit_ranges.rmax + 0.3, 0.0, true),
            PathPreset::Important => select_by(&infos, f64::MAX, 10.0, false),
            PathPreset::All => infos.iter().map(|p| p.index).collect(),
            PathPreset::None => Vec::new(),
        };
        for p in &infos {
            self.fit_paths[p.index].spec.enabled = selected.contains(&p.index);
        }
        self.paths_selection_changed(cx);
    }

    /// Toggle one path or a whole shell.
    pub(crate) fn set_paths_selected(
        &mut self,
        indices: &[usize],
        on: bool,
        cx: &mut Context<Self>,
    ) {
        for &i in indices {
            if let Some(row) = self.fit_paths.get_mut(i) {
                row.spec.enabled = on;
            }
        }
        self.paths_selection_changed(cx);
    }

    /// Indices whose label matches the picker's text filter.
    fn picker_visible(&self, filter: &str) -> Vec<usize> {
        let f = filter.trim().to_ascii_lowercase();
        self.fit_path_infos
            .iter()
            .filter(|p| {
                self.path_in_visible_source(p.index)
                    && (f.is_empty()
                        || p.label.to_ascii_lowercase().contains(&f)
                        || p.filename.contains(&f)
                        || format!("{}", p.index + 1) == f)
            })
            .map(|p| p.index)
            .collect()
    }

    /// The picker element. `docked` = the wide table beside the 3D view;
    /// otherwise the compact inspector version.
    pub(crate) fn path_picker(
        &self,
        docked: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let t = self.theme;
        let filter_text = self.structure.path_filter.read(cx).text();
        let visible = self.picker_visible(&filter_text);
        let infos: Vec<_> = self
            .fit_path_infos
            .iter()
            .filter(|p| self.path_in_visible_source(p.index))
            .cloned()
            .collect();
        let shells: Vec<ShellInfo> = shells_of(&infos);
        let n_sel = self.fit_paths.iter().filter(|r| r.spec.enabled).count();

        // ---- presets + filter ----
        let mut presets = div()
            .px_2()
            .py_1()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1();
        for preset in PathPreset::ALL {
            presets = presets.child(chip(&t, preset.id(), preset.label(), false).on_click(
                cx.listener(move |this, _: &ClickEvent, _w, cx| this.apply_path_preset(preset, cx)),
            ));
        }
        presets = presets.child(
            div()
                .w(px(if docked { 140. } else { 110. }))
                .child(self.structure.path_filter.clone()),
        );
        if !self.structure.multi.is_empty() {
            let sel: Vec<usize> = self.structure.multi.iter().copied().collect();
            let sel2 = sel.clone();
            let n = sel.len();
            presets = presets
                .child(
                    button(&t, "pp-enable-sel", format!("select {n}"), true).on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| this.set_paths_selected(&sel, true, cx),
                    )),
                )
                .child(
                    button(&t, "pp-disable-sel", "deselect", false).on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            this.set_paths_selected(&sel2, false, cx)
                        },
                    )),
                );
        }

        let header = div()
            .px_2()
            .py_0p5()
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(11.))
            .text_color(t.text_muted)
            .child(div().w(px(16.)))
            .child(div().flex_1().child("Path"))
            .child(div().w(px(52.)).child("R_eff"))
            .child(div().w(px(26.)).child("N"))
            .child(div().w(px(30.)).child("Legs"))
            .child(div().w(px(56.)).child("Amp"));

        let mut list = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .id(if docked {
                "pp-list-docked"
            } else {
                "pp-list-insp"
            })
            .overflow_y_scroll();
        if infos.is_empty() {
            list = list.child(
                div()
                    .p_2()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child("No paths yet. Calculate them from a structure."),
            );
        }

        let mut placed: Vec<bool> = vec![false; self.fit_paths.len()];
        for shell in &shells {
            // SS rows of this shell.
            let ss: Vec<usize> = shell
                .paths
                .iter()
                .copied()
                .filter(|i| visible.contains(i))
                .collect();
            // MS rows whose outermost shell is this one.
            let ms: Vec<usize> = infos
                .iter()
                .filter(|p| !p.is_single_scattering && p.shell == shell.number)
                .map(|p| p.index)
                .filter(|i| visible.contains(i))
                .collect();
            for &i in ss.iter().chain(ms.iter()) {
                placed[i] = true;
            }
            if ss.is_empty() && ms.is_empty() {
                continue;
            }
            let all_on = !shell.paths.is_empty()
                && shell
                    .paths
                    .iter()
                    .all(|&i| self.fit_paths.get(i).is_some_and(|r| r.spec.enabled));
            let shell_paths = shell.paths.clone();
            let n_sel_shell = shell
                .paths
                .iter()
                .filter(|&&i| self.fit_paths.get(i).is_some_and(|r| r.spec.enabled))
                .count();
            list = list.child(
                div()
                    .id(("pp-shell", shell.number))
                    .h(px(24.))
                    .px_2()
                    .mt_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .bg(t.surface)
                    .border_b_1()
                    .border_color(t.border)
                    .text_size(px(11.))
                    .child(self.picker_checkbox(
                        ("pp-shell-en", shell.number),
                        all_on,
                        n_sel_shell > 0 && !all_on,
                        move |this, cx| {
                            let on = !this
                                .fit_paths
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| shell_paths.contains(i))
                                .all(|(_, r)| r.spec.enabled);
                            this.set_paths_selected(&shell_paths, on, cx);
                        },
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(SharedString::from(format!(
                                "shell {} · {}–{}",
                                shell.number, shell.absorber, shell.symbol
                            ))),
                    )
                    .child(mono(&t, format!("{:.3}", shell.reff), 52.))
                    .child(mono(&t, format!("{:.0}", shell.degeneracy), 26.))
                    .child(div().w(px(30.)))
                    .child(
                        div()
                            .w(px(56.))
                            .text_size(px(11.))
                            .text_color(t.text_muted)
                            .child(SharedString::from(format!(
                                "{} SS · {} MS",
                                shell.paths.len(),
                                infos
                                    .iter()
                                    .filter(|p| !p.is_single_scattering && p.shell == shell.number)
                                    .count()
                            ))),
                    ),
            );
            for i in ss {
                list = list.child(self.picker_row(i, docked, cx));
            }
            if !ms.is_empty() {
                let open = self.structure.ms_open.contains(&shell.number);
                let n_ms = ms.len();
                let n_ms_sel = ms
                    .iter()
                    .filter(|&&i| self.fit_paths.get(i).is_some_and(|r| r.spec.enabled))
                    .count();
                let number = shell.number;
                list = list.child(
                    div()
                        .id(("pp-ms", number))
                        .h(px(22.))
                        .pl_6()
                        .pr_2()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_size(px(11.))
                        .text_color(t.text_muted)
                        .cursor_pointer()
                        .hover(|d| d.bg(t.raised))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            if !this.structure.ms_open.remove(&number) {
                                this.structure.ms_open.insert(number);
                            }
                            cx.notify();
                        }))
                        .child(if open { "▾" } else { "▸" })
                        .child(SharedString::from(format!(
                            "multiple scattering ({n_ms}{})",
                            if n_ms_sel > 0 {
                                format!(", {n_ms_sel} selected")
                            } else {
                                String::new()
                            }
                        ))),
                );
                if open {
                    for i in ms {
                        list = list.child(self.picker_row(i, docked, cx));
                    }
                }
            }
        }
        // Paths that belong to no shell (no geometry, or MS legs outside any
        // SS shell).
        let rest: Vec<usize> = infos
            .iter()
            .map(|p| p.index)
            .filter(|&i| !placed[i] && visible.contains(&i))
            .collect();
        if !rest.is_empty() {
            list = list.child(
                div()
                    .h(px(22.))
                    .px_2()
                    .mt_1()
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .bg(t.surface)
                    .border_b_1()
                    .border_color(t.border)
                    .child(SharedString::from(format!("other paths ({})", rest.len()))),
            );
            for i in rest {
                list = list.child(self.picker_row(i, docked, cx));
            }
        }

        let mut col = div().flex().flex_col().min_h_0().min_w_0();
        if docked {
            col = col.child(
                div()
                    .px_2()
                    .pt_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(section_label(&t, "Paths"))
                    .child(div().text_size(px(11.)).text_color(t.text_muted).child(
                        SharedString::from(format!(
                            "{n_sel} selected across {} · Click a row to inspect",
                            crate::text::plural(self.path_sources().len(), "source")
                        )),
                    )),
            );
        }
        let mut sources = div()
            .id("path-source-list")
            .max_h(px(168.))
            .overflow_y_scroll()
            .px_2()
            .py_1()
            .flex()
            .flex_col()
            .gap_1();
        for (n, source) in self.path_sources().into_iter().enumerate() {
            let count = self
                .fit_paths
                .iter()
                .filter(|p| p.spec.file.parent() == Some(source.as_path()) && p.spec.enabled)
                .count();
            let label = format!(
                "{} · {} · {count} selected",
                n + 1,
                self.source_label(&source)
            );
            let active = self.structure.source_filter.as_ref() == Some(&source);
            let missing = self.path_source_missing(&source);
            let confirming = self.structure.remove_source.as_ref() == Some(&source);
            let total = self
                .fit_paths
                .iter()
                .filter(|p| p.spec.file.parent() == Some(source.as_path()))
                .count();
            let pick = source.clone();
            let line = div()
                .id(("path-source", n))
                .flex_1()
                .min_w_0()
                .text_ellipsis()
                .overflow_hidden()
                .whitespace_nowrap()
                .cursor_pointer()
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    let source = pick.clone();
                    this.structure.source_filter = Some(source.clone());
                    this.structure.selected = this
                        .fit_paths
                        .iter()
                        .position(|p| p.spec.file.parent() == Some(source.as_path()));
                    this.structure.multi.clear();
                    this.structure.ms_open.clear();
                    this.structure.path_leg = None;
                    this.structure.pick = None;
                    this.rebuild_structure_plot(cx);
                    cx.notify();
                }));
            let ask = source.clone();
            let mut entry = div()
                .px_2()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(if missing {
                    t.warn
                } else if active {
                    t.accent
                } else {
                    t.border
                })
                .text_size(px(11.))
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(line)
                        .when(missing, |d| {
                            d.child(
                                div()
                                    .flex_none()
                                    .text_color(t.warn)
                                    .child("FEFF files missing"),
                            )
                        })
                        .child(
                            div()
                                .id(("path-source-remove", n))
                                .flex_none()
                                .px_1()
                                .cursor_pointer()
                                .text_color(t.text_muted)
                                .hover(|d| d.text_color(t.error))
                                .child("×")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.structure.remove_source = Some(ask.clone());
                                    cx.notify();
                                })),
                        ),
                );
            if missing && !confirming {
                let recalc = source.clone();
                entry = entry.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .text_color(t.text_muted)
                                .child("Recalculate to re-link these paths, or remove them."),
                        )
                        .child(
                            button(&t, ("path-source-recalc", n), "Recalculate", false).on_click(
                                cx.listener(move |this, _, _, cx| {
                                    this.recalculate_path_source(recalc.clone(), cx);
                                }),
                            ),
                        ),
                );
            }
            if confirming {
                let remove = source.clone();
                entry = entry
                    .child(div().w_full().child(SharedString::from(format!(
                        "Remove source {} and its {}? Variables no other path uses are removed too.",
                        n + 1,
                        crate::text::plural(total, "path")
                    ))))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                button(&t, ("path-source-remove-ok", n), "Remove", false).on_click(
                                    cx.listener(move |this, _, _, cx| {
                                        this.remove_path_source(remove.clone(), cx);
                                    }),
                                ),
                            )
                            .child(
                                button(&t, ("path-source-remove-no", n), "Cancel", false).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.structure.remove_source = None;
                                        cx.notify();
                                    }),
                                ),
                            ),
                    );
            }
            sources = sources.child(entry);
        }
        col = col.child(sources);
        if self.path_sources().len() > 1 {
            col = col.child(
                div()
                    .px_2()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child("Selected paths from all sources contribute to the fit."),
            );
            if let Some(source) = self.structure.source_filter.clone() {
                col = col.child(
                    button(
                        &t,
                        "pp-exclude-other-sources",
                        "Deselect other sources",
                        false,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        for path in &mut this.fit_paths {
                            if path.spec.file.parent() != Some(source.as_path()) {
                                path.spec.enabled = false;
                            }
                        }
                        this.paths_selection_changed(cx);
                    })),
                );
            }
        }
        col.child(
            button(&t, "another-path-source", "+ Add another structure", false).on_click(
                cx.listener(|this, _, _, cx| {
                    this.set_fit_step(super::fit_workspace::FitStep::Structure, cx);
                }),
            ),
        )
        .child(presets)
        .child(header)
        .child(list)
        .flex_1()
    }

    fn picker_checkbox<F: Fn(&mut Self, &mut Context<Self>) + 'static>(
        &self,
        id: (&'static str, usize),
        on: bool,
        partial: bool,
        f: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<F> {
        let t = self.theme;
        div()
            .id(id)
            .flex_none()
            .w(px(12.))
            .h(px(12.))
            .rounded_sm()
            .border_1()
            .flex()
            .items_center()
            .justify_center()
            .when(on, |d| d.bg(t.accent).border_color(t.accent))
            .when(!on && partial, |d| d.bg(t.raised).border_color(t.accent))
            .when(!on && !partial, |d| d.bg(t.raised).border_color(t.border))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                cx.stop_propagation();
                f(this, cx);
            }))
            .children(on.then(|| icon(&t, Icon::Check).size(px(11.)).text_color(t.bg)))
            .children(
                (!on && partial).then(|| div().text_size(px(11.)).text_color(t.accent).child("–")),
            )
    }

    fn picker_row(
        &self,
        i: usize,
        docked: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let t = self.theme;
        let info = &self.fit_path_infos[i];
        let enabled = self.fit_paths.get(i).is_some_and(|r| r.spec.enabled);
        let selected = self.structure.multi.contains(&i);
        let focused = self.structure.selected == Some(i);
        let label: SharedString = format!("{} {}", i + 1, info.label).into();
        let bar_w = (info.importance * 0.4).round().max(1.0) as f32;
        div()
            .id(("pp-row", i))
            .h(px(if docked { 24. } else { 22. }))
            .pl(px(if info.is_single_scattering { 14. } else { 26. }))
            .pr_2()
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(11.5))
            .cursor_pointer()
            .when(selected, |d| d.bg(t.raised))
            .when(focused, |d| d.border_l_2().border_color(t.accent))
            .hover(|d| d.bg(t.raised))
            .on_hover(cx.listener(move |this, hovered: &bool, _w, cx| {
                this.hover_structure_path(if *hovered { Some(i) } else { None }, cx);
            }))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                this.select_structure_path(i, ev.modifiers(), cx);
            }))
            .child(self.picker_checkbox(
                ("pp-row-en", i),
                enabled,
                false,
                move |this, cx| {
                    let now = this.fit_paths.get(i).is_some_and(|r| r.spec.enabled);
                    this.set_paths_selected(&[i], !now, cx);
                },
                cx,
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .when(!enabled, |d| d.text_color(t.text_muted))
                    .child(label),
            )
            .child(mono(&t, format!("{:.3}", info.reff), 52.))
            .child(mono(&t, format!("{:.0}", info.degen), 26.))
            .child(mono(&t, format!("{}", info.nleg), 30.))
            .child(
                div()
                    .w(px(56.))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(div().w(px(bar_w)).h(px(6.)).rounded_sm().bg(if enabled {
                        t.accent
                    } else {
                        t.border
                    }))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(t.text_muted)
                            .child(SharedString::from(format!("{:.0}", info.importance))),
                    ),
            )
    }
}

fn mono(t: &crate::theme::Theme, text: String, w: f32) -> gpui::Div {
    div()
        .w(px(w))
        .flex_none()
        .font_family(MONO)
        .text_size(px(11.))
        .text_color(t.text_muted)
        .child(SharedString::from(text))
}
