//! Analysis intervals are separate from display-only energy zoom presets.

use super::*;

impl StudioApp {
    pub(crate) fn analysis_energy_interval(&self, cx: &gpui::App) -> Option<(f64, f64)> {
        let tool = self.tools.open.filter(|t| t.is_analysis())?;
        if self.tools.lcf_space == LcfSpaceChoice::Chi {
            return None;
        }
        let first = if tool == Tool::Lcf {
            self.spectrum.as_ref()
        } else {
            let marks = self.analysis_selection(tool);
            let ix = marked_group_indices(&marks).next()?;
            self.cache.peek(&(ix, self.effective_fingerprint(ix)))
        }?;
        let e0 = first.e0()?;
        let lo = self
            .tools
            .field_value(ToolField::RangeLo, cx)
            .unwrap_or(-20.);
        let hi = self
            .tools
            .field_value(ToolField::RangeHi, cx)
            .unwrap_or(30.);
        (lo.is_finite() && hi.is_finite() && lo < hi).then_some((e0 + lo, e0 + hi))
    }

    pub(super) fn set_analysis_range_preset(
        &mut self,
        tool: Tool,
        range: Option<(f64, f64)>,
        cx: &mut Context<Self>,
    ) {
        let outcome = (|| {
            if self.tools.lcf_space == LcfSpaceChoice::Chi {
                return Err("Energy presets apply to norm, flat or derivative spectra".to_owned());
            }
            let range = if let Some(range) = range {
                range
            } else {
                let mut marks = self.analysis_selection(tool);
                if tool == Tool::Lcf
                    && let Some(ix) = self.current_group_index()
                {
                    marks.insert(ix);
                }
                let indices: Vec<_> = marked_group_indices(&marks).collect();
                let mut spectra = Vec::new();
                for ix in indices {
                    spectra.push(self.cache.peek(&(ix,self.effective_fingerprint(ix)))
                        .ok_or("Wait for the selected spectra to load before choosing common full range")?);
                }
                let first = if tool == Tool::Lcf {
                    self.spectrum.as_ref()
                } else {
                    spectra.first().copied()
                }
                .ok_or("Select spectra first")?;
                let e0 = first.e0().ok_or("The reference spectrum needs E0")?;
                let mut lo = f64::NEG_INFINITY;
                let mut hi = f64::INFINITY;
                for spectrum in spectra {
                    let energy = spectrum
                        .energy
                        .as_ref()
                        .ok_or("Spectrum has no energy axis")?;
                    lo = lo.max(*energy.as_slice().first().ok_or("Empty energy axis")?);
                    hi = hi.min(*energy.as_slice().last().ok_or("Empty energy axis")?);
                }
                if !lo.is_finite() || !hi.is_finite() || lo >= hi {
                    return Err("Selected spectra have no common measured interval".into());
                }
                (lo - e0, hi - e0)
            };
            for (field, value) in [(ToolField::RangeLo, range.0), (ToolField::RangeHi, range.1)] {
                if let Some((_, entity)) = self.tools.fields.iter().find(|(f, _)| *f == field) {
                    entity.update(cx, |field, cx| field.set_value(Some(value), cx));
                }
            }
            self.tools.sync_range(cx)?;
            self.analysis.shown = None;
            self.analysis.plot = None;
            self.tools.message =
                "Review the dashed analysis boundaries, then run the calculation.".into();
            self.invalidate_explore_plots(cx);
            Ok(())
        })();
        if let Err(error) = outcome {
            self.tools.message = error.into();
        }
        cx.notify();
    }
}
