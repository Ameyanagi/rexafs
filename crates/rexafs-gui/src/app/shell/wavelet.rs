//! Wavelet workspace: immutable scientific maps, bounded current residency and
//! independent display controls. Original groups and pipeline settings stay intact.
use super::*;
use crate::{
    group_identity::GroupId,
    params::{PipelineParams, RequiredStage},
    wavelet_history::{
        self as history, WaveletArchive, WaveletReceipt, WaveletRecord, WaveletSavedRegion,
    },
    widgets::text_input::{InputEvent, TextInput},
};
use gpui::{AppContext, Entity};
use rexafs::{Wavelet, WaveletRegionValue};
use ruviz_gpui::{PlotPointerEvent, PlotPointerEventKind, RuvizPlot};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
mod controls;
mod plots;
pub(crate) const PLOT_WAVELET_K: usize = 500;
pub(crate) const PLOT_WAVELET_R: usize = 501;

#[derive(Default)]
pub(crate) struct WaveletState {
    pub open: bool,
    pub archive: WaveletArchive,
    group: Option<GroupId>,
    source_settings: Option<PipelineParams>,
    fields: Vec<Entity<TextInput>>,
    region_fields: Vec<Entity<TextInput>>,
    record: Option<Arc<WaveletRecord>>,
    receipt: Option<WaveletReceipt>,
    pub plot_k: Option<Entity<RuvizPlot>>,
    pub plot_r: Option<Entity<RuvizPlot>>,
    plot_map: Option<Entity<RuvizPlot>>,
    subscription: Option<gpui::Subscription>,
    stop: Option<Arc<AtomicBool>>,
    generation: u64,
    busy: bool,
    message: String,
    region_message: String,
    region: [f64; 4],
    region_value: Option<WaveletRegionValue>,
    cursor: [f64; 2],
    slices: bool,
    view: usize,
    palette: Option<crate::app::series_display::HeatmapPalette>,
    reversed: bool,
    colors_open: bool,
    history_open: bool,
    menu_position: gpui::Point<gpui::Pixels>,
    menu_focus: Option<gpui::FocusHandle>,
    lock_scale: Option<(f64, f64)>,
    lock_basis: Option<(PipelineParams, Wavelet)>,
    advanced: bool,
}
impl WaveletState {
    pub fn cancel(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        self.generation += 1;
        self.busy = false;
    }
}
fn prepare_wavelet_input(
    input: &crate::series_measurements::FrameInput,
    settings: &PipelineParams,
) -> Result<rexafs::Spectrum, String> {
    match &input.derived {
        Some(d) if d.quantity == crate::params::Quantity::ChiK && !d.quantity_unconfirmed => {
            d.for_display(settings)
        }
        Some(d) => d.prepare(settings, RequiredStage::Background),
        None => {
            let (e, m) = crate::params::load_raw(&input.path, settings)?;
            crate::params::prepare_arrays(e, m, settings, RequiredStage::Background)
        }
    }
}
impl StudioApp {
    pub(crate) fn wavelet_matches_current(&self) -> bool {
        self.wavelet.group == self.current_group_index().and_then(|i| self.group_id(i))
            && self.wavelet.source_settings.as_ref() == Some(self.ui_params())
    }
    pub(crate) fn open_wavelet(&mut self, cx: &mut Context<Self>) {
        self.wavelet.cancel();
        self.wavelet.open = true;
        self.wavelet.colors_open = false;
        self.wavelet.history_open = false;
        self.peaks.open = false;
        self.wavelet.group = self.current_group_index().and_then(|i| self.group_id(i));
        self.wavelet.source_settings = Some(self.ui_params().clone());
        self.wavelet.record = None;
        self.wavelet.receipt = None;
        self.wavelet.plot_map = None;
        self.wavelet.plot_k = None;
        self.wavelet.plot_r = None;
        self.wavelet.region_value = None;
        self.wavelet.message = "Choose the measured k range, then Calculate.".into();
        let recent = self
            .wavelet
            .archive
            .entries
            .iter()
            .rev()
            .find(|r| {
                Some(&r.group) == self.wavelet.group.as_ref() && r.settings == *self.ui_params()
            })
            .cloned();
        let available = self
            .spectrum
            .as_ref()
            .and_then(|s| s.k())
            .and_then(|k| k.last())
            .copied()
            .unwrap_or(12.);
        let definition = recent
            .as_ref()
            .map(|r| r.definition.clone())
            .unwrap_or_else(|| Wavelet::new(2. ..=(available.min(12.) * 100.).floor() / 100.));
        self.set_wavelet_fields(&definition, cx);
        if let Some(receipt) = recent {
            self.load_wavelet(receipt, cx);
        }
        cx.notify();
    }
    fn set_wavelet_fields(&mut self, definition: &Wavelet, cx: &mut Context<Self>) {
        self.wavelet.fields.clear();
        let taper = match definition.window {
            rexafs::WaveletWindow::None => 0.,
            rexafs::WaveletWindow::Cosine { width } => width,
        };
        let values = [
            format!("{:.2}", definition.k_range[0]),
            format!("{:.2}", definition.k_range[1]),
            definition.kweight.to_string(),
            format!("{:.2}", definition.rmax),
            definition.order.to_string(),
            definition.kstep.to_string(),
            definition.rstep.map(|v| v.to_string()).unwrap_or_default(),
            taper.to_string(),
        ];
        for (label, value) in [
            "Wavelet k from (Å⁻¹)",
            "Wavelet k to (Å⁻¹)",
            "Wavelet k weight",
            "Wavelet R maximum (Å)",
            "Cauchy order",
            "Wavelet k step (Å⁻¹)",
            "Wavelet R step (Å)",
            "Cosine taper width (Å⁻¹)",
        ]
        .into_iter()
        .zip(values)
        {
            let field = cx.new(|cx| {
                let mut f = TextInput::new("auto", value, self.theme, cx);
                f.set_accessible_name(label);
                f
            });
            cx.subscribe(&field, |app, _, event, cx| {
                if matches!(event, InputEvent::Edited(_)) {
                    app.wavelet.cancel();
                    app.wavelet.message = "Settings changed · Calculate to update the map".into();
                    cx.notify();
                }
            })
            .detach();
            self.wavelet.fields.push(field);
        }
    }
    fn wavelet_definition(&self, cx: &Context<Self>) -> Result<Wavelet, String> {
        if self.wavelet.fields.len() != 8 {
            return Err("Open Wavelet for a selected spectrum".into());
        }
        let value = |i: usize| self.wavelet.fields[i].read(cx).text().trim().to_owned();
        let number = |i: usize| {
            value(i)
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or("Enter finite wavelet settings".to_owned())
        };
        let weight = number(2)?;
        let order = number(4)?;
        if weight.fract() != 0.
            || !(0. ..=6.).contains(&weight)
            || order.fract() != 0.
            || !(1. ..=4096.).contains(&order)
        {
            return Err("Weight must be 0–6; order must be 1–4096 (whole numbers)".into());
        }
        let mut definition = Wavelet::new(number(0)?..=number(1)?)
            .kweight(weight as u8)
            .rmax(number(3)?)
            .order(order as usize)
            .kstep(number(5)?);
        if !value(6).is_empty() {
            definition = definition.rstep(number(6)?);
        }
        let taper = number(7)?;
        if taper < 0. {
            return Err("Taper width must be zero (none) or positive".into());
        }
        if taper > 0. {
            definition = definition.taper(taper);
        }
        Ok(definition)
    }
    fn calculate_wavelet(&mut self, cx: &mut Context<Self>) {
        if self.wavelet.busy {
            return;
        }
        let definition = match self.wavelet_definition(cx) {
            Ok(d) => d,
            Err(e) => {
                self.wavelet.message = e;
                cx.notify();
                return;
            }
        };
        let Some(ix) = self.current_group_index() else {
            return;
        };
        let Some(group) = self.group_id(ix) else {
            return;
        };
        let input = self.measurement_input(&group, self.entry_label(ix));
        let settings = self.ui_params().clone();
        let before = settings.clone();
        let project = self.project_generation;
        self.wavelet.cancel();
        let generation = self.wavelet.generation;
        let stop = Arc::new(AtomicBool::new(false));
        self.wavelet.stop = Some(stop.clone());
        self.wavelet.busy = true;
        self.wavelet.message = "Calculating full map…".into();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let mut sp = prepare_wavelet_input(&input,&settings)?;
                    let map = sp
                        .wavelet_with_cancel(&definition, || stop.load(Ordering::Relaxed))
                        .map_err(|e| e.to_string())?;
                    if map.r().len()<2 {return Err("The desktop map needs at least two R rows; increase R maximum or reduce R step".into());}
                    let mut fft_settings = settings.clone();
                    fft_settings.fft_kmin = Some(definition.k_range[0]);
                    fft_settings.fft_kmax = Some(definition.k_range[1]);
                    fft_settings.fft_kweight = Some(definition.kweight as f64);
                    let fourier_error = match crate::params::forward_settings(&fft_settings) {
                        Ok(f) => sp.set_fft(f).fft().err().map(|e| e.to_string()),
                        Err(e) => Some(e),
                    };
                    let fourier = if fourier_error.is_none() {
                        sp.xftf
                    } else {
                        None
                    };
                    let record = WaveletRecord {
                        schema: 1,
                        group: input.group.clone(),
                        label: input.label,
                        settings,
                        map,
                        fourier,
                        fourier_error,
                    };
                    if stop.load(Ordering::Relaxed) {
                        return Err("Wavelet calculation cancelled".into());
                    }
                    let receipt = history::retain(&history::root()?, &record)?;
                    Ok::<_, String>((record, receipt))
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != project || app.wavelet.generation != generation {
                    return;
                }
                app.wavelet.busy = false;
                app.wavelet.stop = None;
                if app
                    .current_group_index()
                    .and_then(|i| app.group_id(i))
                    .as_ref()
                    != Some(&group)
                    || app.ui_params() != &before
                {
                    app.wavelet.message = "Spectrum or processing changed; calculate again".into();
                    cx.notify();
                    return;
                }
                match result {
                    Err(e) => app.wavelet.message = e,
                    Ok((record, receipt)) => {
                        app.wavelet.archive.insert(receipt.clone());
                        app.record("Retained full wavelet map", None);
                        app.show_wavelet(record, receipt, cx);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    fn load_wavelet(&mut self, receipt: WaveletReceipt, cx: &mut Context<Self>) {
        let project = self.project_generation;
        let generation = self.wavelet.generation;
        self.wavelet.busy = true;
        cx.spawn(async move |this, cx| {
            let copy = receipt.clone();
            let result = cx
                .background_spawn(async move { history::read(&copy) })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != project || app.wavelet.generation != generation {
                    return;
                }
                app.wavelet.busy = false;
                match result {
                    Ok(record) => app.show_wavelet(record, receipt, cx),
                    Err(e) => app.wavelet.message = e,
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn show_wavelet(
        &mut self,
        record: WaveletRecord,
        receipt: WaveletReceipt,
        cx: &mut Context<Self>,
    ) {
        if record.map.r().len() < 2 {
            self.wavelet.message =
                "The desktop map needs at least two R rows; increase R maximum or reduce R step"
                    .into();
            return;
        }
        if self
            .wavelet
            .lock_basis
            .as_ref()
            .is_some_and(|(settings, definition)| {
                settings != &record.settings || definition != record.map.settings()
            })
        {
            self.wavelet.lock_scale = None;
            self.wavelet.lock_basis = None;
        }
        let map = &record.map;
        let k = map.settings().k_range;
        let r = [map.r()[0], *map.r().last().unwrap()];
        self.wavelet.region = [
            k[0] + 0.2 * (k[1] - k[0]),
            k[1] - 0.2 * (k[1] - k[0]),
            r[0] + 0.2 * (r[1] - r[0]),
            r[1] - 0.2 * (r[1] - r[0]),
        ];
        self.wavelet.cursor = [0.5 * (k[0] + k[1]), 0.5 * (r[0] + r[1])];
        self.wavelet.message = format!(
            "Retained map · {} · {} k × {} R · order {}",
            record.label,
            map.k().len(),
            map.r().len(),
            map.settings().order
        );
        self.wavelet.plot_map = None;
        self.wavelet.plot_k = None;
        self.wavelet.plot_r = None;
        self.wavelet.subscription = None;
        self.wavelet.record = Some(Arc::new(record));
        self.wavelet.receipt = Some(receipt);
        self.wavelet.region_fields.clear();
        for (label, value) in [
            "Region k from (Å⁻¹)",
            "Region k to (Å⁻¹)",
            "Region R from (Å)",
            "Region R to (Å)",
        ]
        .into_iter()
        .zip(self.wavelet.region)
        {
            let field = cx.new(|cx| {
                let mut f = TextInput::new("", format!("{value:.2}"), self.theme, cx);
                f.set_accessible_name(label);
                f
            });
            cx.subscribe(&field, |app, _, event, cx| {
                if matches!(event, InputEvent::Edited(_)) {
                    app.read_wavelet_region(cx);
                }
            })
            .detach();
            self.wavelet.region_fields.push(field);
        }
        // Use the exact visible rounded bounds, so field text and the scalar agree.
        self.read_wavelet_region(cx);
        self.rebuild_wavelet_plots(cx);
    }
    fn read_wavelet_region(&mut self, cx: &mut Context<Self>) {
        if self.wavelet.region_fields.len() != 4 {
            return;
        }
        let values: Result<Vec<_>, _> = self
            .wavelet
            .region_fields
            .iter()
            .map(|f| f.read(cx).text().trim().parse::<f64>())
            .collect();
        match values {
            Ok(v) if v.iter().all(|x| x.is_finite()) => {
                self.wavelet.region.copy_from_slice(&v);
                self.update_wavelet_region();
            }
            _ => {
                self.wavelet.region_value = None;
                self.wavelet.region_message = "Enter finite region bounds".into();
            }
        }
        cx.notify();
    }
    fn update_wavelet_region(&mut self) {
        let Some(record) = &self.wavelet.record else {
            return;
        };
        let [ka, kb, ra, rb] = self.wavelet.region;
        match record.map.integral(ka..=kb, ra..=rb) {
            Ok(value) => {
                self.wavelet.region_message =
                    format!("∫ |W| dk dR = {:.5} {}", value.value, value.unit);
                self.wavelet.region_value = Some(value);
            }
            Err(e) => {
                self.wavelet.region_message = e.to_string();
                self.wavelet.region_value = None;
            }
        }
    }
    fn save_wavelet_region(&mut self, cx: &mut Context<Self>) {
        if let (Some(receipt), Some(measurement)) =
            (&self.wavelet.receipt, &self.wavelet.region_value)
        {
            self.wavelet.archive.regions.push(WaveletSavedRegion {
                map_digest: receipt.digest.clone(),
                label: receipt.label.clone(),
                measurement: measurement.clone(),
            });
            self.record("Retained wavelet region measurement", None);
            self.wavelet.message =
                "Region saved with its full map and processing provenance".into();
            cx.notify();
        }
    }
    fn export_wavelet(&mut self, cx: &mut Context<Self>) {
        let Some(record) = self.wavelet.record.clone() else {
            return;
        };
        let regions = self
            .wavelet
            .archive
            .regions
            .iter()
            .filter(|r| {
                self.wavelet
                    .receipt
                    .as_ref()
                    .is_some_and(|p| p.digest == r.map_digest)
            })
            .cloned()
            .collect::<Vec<_>>();
        let request = cx.prompt_for_new_path(&std::env::temp_dir(), Some("wavelet.json"));
        let project = self.project_generation;
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = request.await else {
                return;
            };
            let result = cx
                .background_spawn(async move {
                    let mut f = tempfile::NamedTempFile::new_in(
                        path.parent().ok_or("Export directory unavailable")?,
                    )
                    .map_err(|e| e.to_string())?;
                    #[derive(serde::Serialize)]
                    struct Export<'a> {
                        record: &'a WaveletRecord,
                        regions: &'a [WaveletSavedRegion],
                    }
                    serde_json::to_writer(
                        &mut f,
                        &Export {
                            record: record.as_ref(),
                            regions: &regions,
                        },
                    )
                    .map_err(|e| e.to_string())?;
                    f.as_file().sync_all().map_err(|e| e.to_string())?;
                    f.persist(&path).map_err(|e| e.to_string())?;
                    Ok::<_, String>(())
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation == project {
                    app.wavelet.message = match result {
                        Ok(()) => "Exported full map, original χ and provenance".into(),
                        Err(e) => e,
                    };
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_chi_reuses_original_arrays_and_unknown_quantities_still_fail() {
        let k = (0..101).map(|i| i as f64 * 0.1).collect::<Vec<_>>();
        let chi = k.iter().map(|x| (4. * x).sin()).collect::<Vec<_>>();
        let group = crate::params::DerivedSpectrum {
            energy: k.clone(),
            mu: chi.clone(),
            quantity: crate::params::Quantity::ChiK,
            ..Default::default()
        };
        let mut input = crate::series_measurements::FrameInput {
            group: GroupId::new_result(),
            label: "Typed χ".into(),
            path: Default::default(),
            derived: Some(Arc::new(group.clone())),
            settings: Default::default(),
            recipe: None,
        };
        let sp = prepare_wavelet_input(&input, &input.settings).unwrap();
        assert!(sp.normalization.is_none());
        let map = sp.wavelet(&Wavelet::new(1. ..=9.)).unwrap();
        assert_eq!(map.input_k(), k);
        assert_eq!(map.input_chi(), chi);
        let mut unknown = group;
        unknown.quantity_unconfirmed = true;
        input.derived = Some(Arc::new(unknown));
        assert!(prepare_wavelet_input(&input, &input.settings).is_err());
        let zero = Wavelet::new(1. ..=9.)
            .calculate(&k, &vec![0.; k.len()])
            .unwrap();
        assert!(
            plots::texture(&zero, 3)
                .0
                .iter()
                .flatten()
                .all(|v| v.is_nan())
        );
        assert_eq!(plots::scale(&zero, 0), (0., 1.));
    }
}
