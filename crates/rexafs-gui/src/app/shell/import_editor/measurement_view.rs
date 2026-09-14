//! Compact, preview-first presentation for all universal reader sources.
use super::*;
use rexafs::io::{EnergyConversion, SignalCandidate, SignalConversion};

fn signal_label(signal: &SignalCandidate) -> String {
    if signal.name == "reference" {
        return "Reference".into();
    }
    if signal.name == "transmission" {
        return "Transmission".into();
    }
    if signal.name == "fluorescence" {
        return "Fluorescence".into();
    }
    let mode = match signal.mapping.signal {
        SignalConversion::Direct { .. } => "Stored signal",
        SignalConversion::Transmission { .. } => "Transmission",
        SignalConversion::Ratio { .. } => "Fluorescence / yield",
    };
    format!("{mode} · {}", signal.name)
}

impl ImportEditor {
    pub(super) fn activate_measurement(&mut self, action: Action, cx: &mut Context<Self>) -> bool {
        if matches!(
            action,
            Action::Signal(_) | Action::Scan(_) | Action::Mode(_)
        ) {
            self.save_measurement_draft();
        }
        let Some(source) = &mut self.measurement else {
            return false;
        };
        match action {
            Action::DatasetView => {
                self.choose_datasets = true;
                self.selector = None;
            }
            Action::Selector(which) => {
                self.selector = (self.selector != Some(which)).then_some(which)
            }
            Action::Columns => self.show_columns = !self.show_columns,
            Action::Scan(index) => {
                self.choose_datasets = false;
                source.select_scan(index);
                self.params.import = source.config();
                self.selector = None;
                self.show_columns = !source.confirmed;
                self.reload(cx);
            }
            Action::IncludeSignal(index) => {
                if source.included[index] || source.candidate_errors[index].is_none() {
                    source.included[index] = !source.included[index];
                }
            }
            Action::Signal(index) => {
                source.signal = Some(index);
                source.confirmed = true;
                self.params.import = source.config();
                self.selector = None;
                self.show_columns = false;
                self.reload(cx);
            }
            Action::Mode(index) => {
                source.signal = None;
                source.confirmed = true;
                let mut config = self
                    .draft
                    .as_ref()
                    .map(|d| d.config().clone())
                    .unwrap_or_else(|| source.config());
                config.mode = REVIEW_CHANNELS[index as usize];
                config.mu_col = config.mu_col.or(Some(1));
                config.i0_col = config.i0_col.or(Some(1));
                config.it_col = config.it_col.or(Some(2));
                config.ir_col = config.ir_col.or(Some(3));
                if config.fluor_cols.as_ref().is_none_or(Vec::is_empty) {
                    config.fluor_cols = Some(vec![2]);
                }
                self.params.import = config;
                self.selector = None;
                self.show_columns = true;
                self.reload(cx);
            }
            Action::Dataset(index) => {
                let path = source.document.datasets[index].path.clone();
                if source.dataset_paths.contains(&path) {
                    source.dataset_paths.retain(|p| p != &path);
                } else {
                    source.dataset_paths.push(path);
                }
            }
            Action::UseDatasets => match source.select_datasets() {
                Ok(()) => {
                    self.choose_datasets = false;
                    self.params.import = source.config();
                    self.show_columns = true;
                    self.reload(cx);
                }
                Err(e) => self.error = Some(e),
            },
            Action::Reset => {
                self.params.import = source.original_config();
                source.remember_config(&self.params.import);
                self.reload(cx);
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    pub(super) fn render_measurement(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let source = self.measurement.as_ref().unwrap().clone();
        let t = self.theme;
        let scan = source.document.scans.get(source.scan);
        let points = scan
            .and_then(|s| s.columns.first())
            .map_or(0, |c| c.values.len());
        let mut popup = None;
        let mut body = div()
            .id("measurement-preview-body")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3();
        let mut selectors = div().flex().gap_2().items_center();
        if source.document.scans.len() > 1 || !source.document.datasets.is_empty() {
            selectors = selectors.child(self.button(
                Action::Selector(0),
                format!("Scan: {} ▾", scan.map_or("Choose…", |s| s.label.as_str())),
                true,
                cx,
            ));
        }
        if scan.is_some() {
            let label: String = if source.confirmed {
                if source.signal.is_some() {
                    "Custom mapping…".into()
                } else {
                    "Custom signal".into()
                }
            } else {
                "Choose signal…".into()
            };
            selectors =
                selectors.child(self.button(Action::Selector(1), format!("{label} ▾"), true, cx));
            selectors = selectors.child(div().flex_1()).child(
                div()
                    .text_color(t.text_muted)
                    .child(format!("{} · {points} points", source.document.format)),
            );
        }
        body = body.child(selectors);
        if let Some(scan) = scan.filter(|_| !self.choose_datasets && source.signal.is_some()) {
            let mut outputs = div()
                .id("measurement-output-choices")
                .flex()
                .flex_col()
                .gap_1()
                .max_h(px(180.))
                .overflow_y_scroll();
            outputs = outputs.child(div().text_color(t.text_muted).child(
                "Select spectra to import. Preview a signal to inspect or edit its columns.",
            ));
            for (index, signal) in scan.signals.iter().enumerate() {
                let name = signal_label(signal);
                let enabled = source.included[index] || source.candidate_errors[index].is_none();
                let mut selected_source = source.clone();
                selected_source.signal = Some(index);
                let formula = selected_source
                    .mapping(&source.configs[index])
                    .map(|m| {
                        let col = |i: usize| {
                            scan.columns
                                .get(i)
                                .map(|c| c.name.clone())
                                .unwrap_or_else(|| format!("column {i}"))
                        };
                        match m.signal {
                            SignalConversion::Direct { column } => col(column),
                            SignalConversion::Transmission {
                                incident,
                                transmitted,
                            } => format!("ln({} / {})", col(incident), col(transmitted)),
                            SignalConversion::Ratio {
                                detectors,
                                incident,
                            } => format!(
                                "({}) / {}",
                                detectors
                                    .iter()
                                    .map(|&i| col(i))
                                    .collect::<Vec<_>>()
                                    .join(" + "),
                                col(incident)
                            ),
                        }
                    })
                    .unwrap_or_else(|e| e);
                let row = div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_1()
                    .rounded_md()
                    .bg(if source.signal == Some(index) {
                        t.surface
                    } else {
                        t.bg
                    })
                    .child(div().w(px(205.)).child(self.button(
                        Action::IncludeSignal(index),
                        format!("{} {name}", if source.included[index] { "☑" } else { "☐" }),
                        enabled,
                        cx,
                    )))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(t.text_muted)
                            .child(formula),
                    )
                    .child(self.button(
                        Action::Signal(index),
                        if source.signal == Some(index) {
                            format!("Previewing {name}")
                        } else {
                            format!("Preview {name}")
                        },
                        true,
                        cx,
                    ));
                outputs = outputs.child(row);
                if let Some(error) = &source.candidate_errors[index] {
                    outputs = outputs.child(
                        div()
                            .text_size(px(11.))
                            .text_color(t.warn)
                            .child(error.clone()),
                    );
                }
            }
            body = body.child(outputs);
        }
        if source
            .document
            .warnings
            .iter()
            .any(|w| w.contains("cannot enumerate") || w.contains("unsupported"))
        {
            body = body.child(
                div()
                    .text_color(t.warn)
                    .text_size(px(12.))
                    .child("Some source data could not be read. See Source details."),
            );
        }
        if let Some(which @ (0 | 1)) = self.selector {
            let labels: Vec<_> = if which == 0 {
                source
                    .document
                    .scans
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (Action::Scan(i), s.label.clone()))
                    .collect()
            } else {
                scan.into_iter()
                    .flat_map(|s| {
                        s.signals.iter().enumerate().map(|(i, s)| {
                            (Action::Signal(i), format!("Detected {}", signal_label(s)))
                        })
                    })
                    .collect()
            };
            let mut choices = div()
                .id("measurement-source-choices")
                .max_h(px(200.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_1()
                .p_2()
                .rounded_md()
                .bg(t.surface);
            for (action, label) in labels {
                choices = choices.child(self.button(action, label, true, cx));
            }
            if which == 1 {
                for (index, mode) in REVIEW_CHANNELS.iter().enumerate() {
                    choices = choices.child(self.button(
                        Action::Mode(index as u8),
                        format!("Custom {}…", mode.label()),
                        true,
                        cx,
                    ));
                }
            }
            if which == 0 && !source.document.datasets.is_empty() {
                choices =
                    choices.child(self.button(Action::DatasetView, "Select datasets…", true, cx));
            }
            popup = Some(
                choices
                    .absolute()
                    .top(px(85.))
                    .left(px(16.))
                    .w(px(400.))
                    .occlude()
                    .border_1()
                    .border_color(t.border)
                    .shadow_lg(),
            );
        }
        if let Some(draft) = self.draft.clone().filter(|_| !self.choose_datasets) {
            let config = draft.config();
            let resolved_energy = source.mapping(config).ok().map(|mapping| mapping.energy);
            let axis_label = match config.axis {
                AxisConversion::Auto => match resolved_energy {
                    Some(EnergyConversion::Bragg { .. }) => "Detected angle → eV",
                    Some(EnergyConversion::Kev) => "Detected keV → eV",
                    Some(EnergyConversion::OffsetEv { .. }) => "Relative energy → eV",
                    Some(EnergyConversion::Ev) => "Detected energy",
                    None => "Choose units…",
                },
                AxisConversion::EnergyEv => "Energy · eV",
                AxisConversion::EnergyKev => "Energy · keV",
                AxisConversion::AngleDegrees { .. } => "Angle · degrees",
                AxisConversion::AngleRadians { .. } => "Angle · radians",
            };
            let mut settings = div()
                .w(px(280.))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_color(t.text_muted).child("Energy axis"))
                .child(self.button(Action::Selector(2), format!("{axis_label} ▾"), true, cx));
            if self.selector == Some(2) {
                for (i, label) in [
                    "Detected",
                    "eV",
                    "keV",
                    "Angle · degrees",
                    "Angle · radians",
                ]
                .iter()
                .enumerate()
                {
                    settings = settings.child(self.button(Action::Axis(i as u8), *label, true, cx));
                }
            }
            match config.axis {
                AxisConversion::AngleDegrees { .. } | AxisConversion::AngleRadians { .. } => {
                    self.visible_focus
                        .push(self.spacing.read(cx).focus_handle(cx));
                    settings = settings.child(
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child("d (Å)")
                            .child(div().w(px(130.)).child(self.spacing.clone())),
                    );
                }
                AxisConversion::Auto => match source.selected_mapping().energy {
                    EnergyConversion::Bragg {
                        d_spacing,
                        degrees_per_unit,
                    } => {
                        settings = settings.child(
                            div()
                                .text_color(t.text_muted)
                                .text_size(px(12.))
                                .child(format!("d = {d_spacing} Å · θ × {degrees_per_unit}")),
                        );
                    }
                    EnergyConversion::OffsetEv { offset_ev } => {
                        settings = settings.child(
                            div()
                                .text_color(t.text_muted)
                                .text_size(px(12.))
                                .child(format!("Origin: {offset_ev} eV")),
                        );
                    }
                    _ => {}
                },
                _ => {}
            }
            settings = settings.child(self.button(
                Action::Columns,
                if self.show_columns {
                    "Columns ▴"
                } else {
                    "Columns ▾"
                },
                true,
                cx,
            ));
            if self.show_columns {
                for (role, col) in draft.roles().into_iter().filter(|(r, _)| r != "ROI") {
                    let key = match role.as_str() {
                        "Energy" => ColumnRole::Energy,
                        "I0" => ColumnRole::I0,
                        "It" => ColumnRole::It,
                        "Ir" => ColumnRole::Ir,
                        _ => ColumnRole::Mu,
                    };
                    settings = settings.child(self.button(
                        Action::Role(key),
                        format!(
                                "{role}: {} ▾",
                                col.map(|i| draft.column_label(i))
                                    .unwrap_or("Choose…".into())
                            ),
                        true,
                        cx,
                    ));
                }
                if self.open_role.is_some() || config.mode == DetectionMode::Fluorescence {
                    let mut columns = div()
                        .id("measurement-column-choices")
                        .max_h(px(170.))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_1();
                    for i in 0..draft.column_count {
                        let is_roi = self.open_role.is_none();
                        let label = if is_roi {
                            format!(
                                "{} {}",
                                if config.fluor_cols.as_ref().is_some_and(|c| c.contains(&i)) {
                                    "☑"
                                } else {
                                    "☐"
                                },
                                draft.column_label(i)
                            )
                        } else {
                            draft.column_label(i)
                        };
                        columns = columns.child(self.button(
                            if is_roi {
                                Action::Roi(i)
                            } else {
                                Action::Pick(i)
                            },
                            label,
                            true,
                            cx,
                        ));
                    }
                    settings = settings.child(columns);
                }
            }
            let mut graph = div()
                .flex_1()
                .min_w_0()
                .h(px(340.))
                .rounded_md()
                .bg(t.surface);
            if let Some(plot) = &self.plot {
                graph = graph.child(plot.clone());
            } else {
                graph =
                    graph
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if self.error.is_some() {
                            "Choose valid columns and units to preview"
                        } else {
                            "Updating preview…"
                        });
            }
            body = body.child(div().flex().gap_4().child(graph).child(settings));
            body = body.child(
                div()
                    .text_color(t.text_muted)
                    .text_size(px(12.))
                    .child(draft.formula()),
            );
        } else if scan.is_none() || self.choose_datasets {
            body = body.child("Select an axis and signal from matching one-dimensional datasets.");
            let mut datasets = div()
                .id("measurement-datasets")
                .max_h(px(280.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_1();
            for (i, ds) in source.document.datasets.iter().enumerate() {
                let enabled = ds.shape.len() == 1 && ds.imaginary.is_none();
                let label = format!(
                    "{} {} · {:?}",
                    if source.dataset_paths.contains(&ds.path) {
                        "☑"
                    } else {
                        "☐"
                    },
                    ds.path,
                    ds.shape
                );
                datasets = datasets.child(self.button(Action::Dataset(i), label, enabled, cx));
            }
            body = body.child(datasets).child(self.button(
                Action::UseDatasets,
                "Preview selected datasets",
                source.dataset_paths.len() >= 2,
                cx,
            ));
        } else {
            body = body.child("Reading source…");
        }
        if let Some(error) = &self.error {
            body = body.child(div().text_color(t.warn).child(error.clone()));
        }
        body = body.child(self.button(
            Action::Details,
            if self.show_details {
                "Source details ▴"
            } else {
                "Source details ▾"
            },
            true,
            cx,
        ));
        if self.show_details {
            let mut details = div()
                .id("measurement-source-details")
                .max_h(px(220.))
                .overflow_y_scroll()
                .p_3()
                .rounded_md()
                .bg(t.surface)
                .font_family(super::super::MONO)
                .text_size(px(11.))
                .flex()
                .flex_col()
                .gap_1();
            for line in source.source_details().lines() {
                details = details.child(line.to_owned());
            }
            if let Some(table) = &self.table {
                details = details.child("First source rows (unchanged):");
                if let Some(names) = &table.names {
                    details = details.child(names.join("   "));
                }
                for row in &table.rows {
                    details = details.child(
                        row.iter()
                            .map(|v| format!("{v:.6}"))
                            .collect::<Vec<_>>()
                            .join("   "),
                    );
                }
            }
            body = body.child(details);
        }
        let ready = !self.choose_datasets
            && source.confirmed
            && source.included_valid()
            && (!source.preview_included()
                || (self.error.is_none()
                    && self
                        .draft
                        .as_ref()
                        .and_then(|d| self.key(d.revision))
                        .is_some_and(|k| self.preview.ready(&k))));
        let panel = div()
            .id("measurement-import-dialog")
            .relative()
            .w_full()
            .max_w(px(1040.))
            .max_h_full()
            .min_h_0()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(t.border)
            .bg(t.bg)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_size(px(18.))
                            .child(format!("Import · {}", self.target.label)),
                    )
                    .child(self.button(Action::Close, "Close", true, cx)),
            )
            .child(body)
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pt_3()
                    .border_t_1()
                    .border_color(t.border)
                    .child(self.button(Action::Reset, "Reset mapping", self.draft.is_some(), cx))
                    .child(div().flex_1())
                    .child(self.button(Action::Cancel, "Cancel", true, cx))
                    .child(self.button(
                        Action::Apply,
                        format!(
                            "Import {} {}",
                            source.included_count(),
                            if source.included_count() == 1 {
                                "spectrum"
                            } else {
                                "spectra"
                            }
                        ),
                        ready,
                        cx,
                    )),
            );
        self.modal(panel.children(popup), cx).into_any_element()
    }
}
