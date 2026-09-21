//! Analysis intervals are separate from display-only energy zoom presets.

use super::*;

/// Full measured energy overlap, in eV; never extrapolate a shorter spectrum.
pub(super) fn common_energy_interval<'a>(
    spectra: impl IntoIterator<Item = &'a XASSpectrum>,
) -> Result<(f64, f64), String> {
    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::INFINITY;
    for spectrum in spectra {
        let energy = spectrum
            .energy
            .as_ref()
            .ok_or("Spectrum has no energy axis")?;
        let start = *energy.as_slice().first().ok_or("Empty energy axis")?;
        let end = *energy.as_slice().last().ok_or("Empty energy axis")?;
        if !start.is_finite() || !end.is_finite() || start >= end {
            return Err("Spectrum needs a finite, increasing energy interval".into());
        }
        lo = lo.max(start);
        hi = hi.min(end);
    }
    if !lo.is_finite() || !hi.is_finite() || lo >= hi {
        return Err("Selected spectra have no common measured interval".into());
    }
    Ok((lo, hi))
}

impl StudioApp {
    pub(crate) fn analysis_energy_interval(&self, cx: &gpui::App) -> Option<(f64, f64)> {
        let tool = self.tools.open.filter(|t| t.is_analysis())?;
        if self.tools.lcf_space == LcfSpaceChoice::Chi {
            return None;
        }
        let (lo_field, hi_field) = tool.range_fields();
        if tool == Tool::Mcr
            && self.tools.field_value(lo_field, cx).is_none()
            && self.tools.field_value(hi_field, cx).is_none()
        {
            let marks = self.analysis_selection(tool);
            let spectra = marked_group_indices(&marks)
                .map(|ix| self.cache.peek(&(ix, self.effective_fingerprint(ix))))
                .collect::<Option<Vec<_>>>()?;
            return common_energy_interval(spectra.iter().map(|s| s.as_ref())).ok();
        }
        let first = if tool == Tool::Lcf {
            self.spectrum.as_ref()
        } else {
            let marks = self.analysis_selection(tool);
            let ix = marked_group_indices(&marks).next()?;
            self.cache.peek(&(ix, self.effective_fingerprint(ix)))
        }?;
        let e0 = first.e0()?;
        let lo = self.tools.field_value(lo_field, cx).unwrap_or(-20.);
        let hi = self.tools.field_value(hi_field, cx).unwrap_or(30.);
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
            if tool == Tool::Mcr && range.is_none() {
                for field in [ToolField::McrRangeLo, ToolField::McrRangeHi] {
                    if let Some((_, entity)) = self.tools.fields.iter().find(|(f, _)| *f == field) {
                        entity.update(cx, |field, cx| field.set_value(None, cx));
                    }
                }
                self.tools.message =
                    "MCR will use the full measured range shared by the selected spectra.".into();
                self.invalidate_explore_plots(cx);
                return Ok(());
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
                let (lo, hi) = common_energy_interval(spectra.iter().map(|s| s.as_ref()))?;
                (lo - e0, hi - e0)
            };
            let (lo_field, hi_field) = tool.range_fields();
            for (field, value) in [(lo_field, range.0), (hi_field, range.1)] {
                if let Some((_, entity)) = self.tools.fields.iter().find(|(f, _)| *f == field) {
                    entity.update(cx, |field, cx| field.set_value(Some(value), cx));
                }
            }
            if tool != Tool::Mcr {
                self.tools.sync_range(cx)?;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_range_uses_overlap_and_rejects_disjoint_or_empty_inputs() {
        let a = XASSpectrum::from_arrays(&[10., 20., 30., 40.], &[0., 1., 2., 3.]).unwrap();
        let b = XASSpectrum::from_arrays(&[15., 25., 35., 45.], &[0., 1., 2., 3.]).unwrap();
        let c = XASSpectrum::from_arrays(&[50., 60., 70., 80.], &[0., 1., 2., 3.]).unwrap();
        assert_eq!(common_energy_interval([&a, &b]).unwrap(), (15., 40.));
        assert_eq!(common_energy_interval([&a]).unwrap(), (10., 40.));
        assert!(common_energy_interval([&a, &c]).is_err());
        assert!(common_energy_interval([]).is_err());
    }
}
