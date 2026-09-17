//! Transform-stage wavelet view, sharing the core transform with RMC.
use super::{Stage, TfView, chip};
use crate::{
    app::StudioApp,
    wavelet::{self, Component, Settings},
    widgets::numeric_field::{FieldEvent, FieldKind, NumericField},
};
use gpui::{Context, Entity, ParentElement, Styled, div, prelude::*, px};
use ruviz_gpui::{RuvizPlot, plot_builder};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

#[derive(Default)]
pub(crate) struct WaveletState {
    pub settings: Settings,
    key: Option<u64>,
    data: Option<Arc<wavelet::Map>>,
    plot: Option<Entity<RuvizPlot>>,
    fields: Vec<Entity<NumericField>>,
    error: Option<String>,
    loading: bool,
}
impl WaveletState {
    pub(crate) fn from_settings(settings: Settings) -> Self {
        Self {
            settings,
            ..Default::default()
        }
    }
}
impl StudioApp {
    fn ensure_wavelet(&mut self, cx: &mut Context<Self>) {
        let Some(spectrum) = self
            .spectrum
            .clone()
            .filter(|_| !self.load_running && self.stale_plots.is_none())
        else {
            self.wavelet.key = None;
            self.wavelet.plot = None;
            self.wavelet.data = None;
            self.wavelet.error = Some("Prepare the selected spectrum in Background first.".into());
            return;
        };
        let params = self.ui_params();
        let range = [
            params.fft_kmin.unwrap_or(2.),
            params.fft_kmax.unwrap_or(12.),
        ];
        let weight = params.fft_kweight.unwrap_or(2.);
        let mut hash = DefaultHasher::new();
        (Arc::as_ptr(&spectrum) as usize).hash(&mut hash);
        self.spectrum_fingerprint.hash(&mut hash);
        self.project_generation.hash(&mut hash);
        let mut numerical = self.wavelet.settings.clone();
        numerical.component = Component::Magnitude;
        serde_json::to_string(&numerical)
            .unwrap_or_default()
            .hash(&mut hash);
        range.map(f64::to_bits).hash(&mut hash);
        weight.to_bits().hash(&mut hash);
        format!("{:?}", self.theme.mode).hash(&mut hash);
        let key = hash.finish();
        if self.wavelet.key == Some(key) {
            return;
        }
        self.wavelet.key = Some(key);
        self.wavelet.loading = true;
        self.wavelet.error = None;
        self.wavelet.plot = None;
        let settings = self.wavelet.settings.clone();
        let task = cx
            .background_executor()
            .spawn(async move { wavelet::calculate(&spectrum, &settings, range, weight) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |app, cx| {
                if app.wavelet.key != Some(key) {
                    return;
                }
                app.wavelet.loading = false;
                match result {
                    Ok(map) => {
                        let plot = wavelet::plot(&map, app.wavelet.settings.component, app.theme);
                        app.wavelet.data = Some(Arc::new(map));
                        app.wavelet.plot = Some(plot_builder(plot).interactive().build(cx));
                    }
                    Err(e) => {
                        app.wavelet.error = Some(e);
                        app.wavelet.data = None;
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    pub(crate) fn wavelet_center(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        self.ensure_wavelet(cx);
        let t = self.theme;
        let mut body = div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .child(self.plot_bar(cx));
        let mut bar = div().flex().flex_wrap().gap_2().px_3().py_2();
        for (i, component) in Component::ALL.into_iter().enumerate() {
            bar = bar.child(
                chip(
                    &t,
                    ("wavelet-component", i),
                    component.label(),
                    self.wavelet.settings.component == component,
                )
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.wavelet.settings.component = component;
                    // Rendering changes do not recompute coefficients.
                    if let Some(data) = &app.wavelet.data {
                        let plot = wavelet::plot(data, component, app.theme);
                        app.wavelet.plot = Some(plot_builder(plot).interactive().build(cx));
                    }
                    // The cache key excludes this display-only component.
                    cx.notify();
                })),
            );
        }
        body=body.child(bar).child(div().px_3().text_size(px(11.5)).text_color(t.text_muted)
            .child("Morlet transform of processed χ(k) · R is Fourier distance · display does not change the fitting objective"));
        if let Some(error) = &self.wavelet.error {
            body = body.child(div().p_3().text_color(t.warn).child(error.clone()));
        }
        if self.wavelet.loading {
            body = body.child(div().p_3().child("Calculating wavelet map…"));
        }
        if let Some(plot) = &self.wavelet.plot {
            body = body.child(div().flex_1().min_h_0().min_w_0().p_3().child(plot.clone()));
        }
        if let Some(data) = &self.wavelet.data {
            body = body.child(div().px_3().pb_2().text_color(t.text_muted).child(format!(
                "{} × {} cells · {}",
                data.k.len(),
                data.r.len(),
                if data.uses_fft {
                    "FFT evaluation"
                } else {
                    "Direct evaluation"
                }
            )));
        }
        body
    }
    pub(crate) fn wavelet_controls(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        if self.wavelet.fields.is_empty() {
            let s = &self.wavelet.settings;
            let specs = [
                ("R min (Å)", s.rmin),
                ("R max (Å)", s.rmax),
                ("Morlet ω₀", s.omega0),
                ("k centers", s.centers as f64),
                ("R points", s.distances as f64),
            ];
            for (index, (name, value)) in specs.into_iter().enumerate() {
                let kind = if index >= 3 {
                    FieldKind::Integer { min: Some(2) }
                } else {
                    FieldKind::Float
                };
                let field = cx.new(|cx| {
                    NumericField::new(name, "required", Some(value), kind, self.theme, cx)
                });
                cx.subscribe(&field, move |app, field, event, cx| {
                    if let FieldEvent::Changed(value) = event {
                        let Some(value) = value else {
                            let s = &app.wavelet.settings;
                            let previous = [
                                s.rmin,
                                s.rmax,
                                s.omega0,
                                s.centers as f64,
                                s.distances as f64,
                            ][index];
                            field.update(cx, |f, cx| f.set_value(Some(previous), cx));
                            return;
                        };
                        match index {
                            0 => app.wavelet.settings.rmin = *value,
                            1 => app.wavelet.settings.rmax = *value,
                            2 => app.wavelet.settings.omega0 = *value,
                            3 => app.wavelet.settings.centers = *value as usize,
                            _ => app.wavelet.settings.distances = *value as usize,
                        }
                        app.wavelet.key = None;
                        cx.notify();
                    }
                })
                .detach();
                self.wavelet.fields.push(field);
            }
        }
        div().flex().flex_col().gap_2().p_3()
            .child(div().child("Wavelet · Morlet"))
            .children(self.wavelet.fields.clone())
            .child(div().text_size(px(11.5)).text_color(self.theme.text_muted).child("Uses the Transform k range and k weight. Larger ω₀ gives broader localization in k. Grid points: 2–128."))
            .child(super::button(&self.theme,"wavelet-fourier","Fourier controls",false).on_click(cx.listener(|app,_,_,cx| {
                app.stage_view.tf_view=TfView::Both;app.set_stage(Stage::Transform,cx);cx.notify();
            })))
    }
}
