//! Trend boundaries share the processing plots' handle layer. Only the scalar
//! is recomputed during a drag; the spectrum and viewport stay in place.
use super::*;
use crate::app::shell::{Stage, handles::HandleKey};

pub(crate) const PLOT_MEASUREMENT: usize = 400;

pub(super) struct MeasurementPreview {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// Bounds in the displayed axis coordinates, including any E₀ offset.
    pub metric: Metric,
    pub origin: f64,
    pub space: MeasurementSpace,
    pub label: String,
}

impl MeasurementPreview {
    fn field_bounds(&self) -> (f64, f64) {
        let coordinate = |x| {
            let value = x - self.origin;
            if self.x.first() == Some(&x) || self.x.last() == Some(&x) {
                // Preserve exact coverage when a handle reaches a measured endpoint.
                value
            } else {
                // Subtracting an energy origin must not expose cancellation noise
                // such as 30.299999999999272 in a field snapped to 0.1 eV.
                (value * 1e6).round() / 1e6
            }
        };
        let (lo, hi) = self.metric.bounds();
        (coordinate(lo), coordinate(hi))
    }

    fn drag(&mut self, end: bool, x: f64) -> bool {
        if !x.is_finite() || self.x.is_empty() {
            return false;
        }
        let step = match self.space {
            MeasurementSpace::Chi { .. } | MeasurementSpace::Fourier => 0.01,
            _ => 0.1,
        };
        let value = (((x - self.origin) / step).round() * step * 1e6).round() / 1e6;
        let x = (value + self.origin).clamp(self.x[0], self.x[self.x.len() - 1]);
        match &mut self.metric {
            Metric::Point { x: point } => {
                if *point == x {
                    return false;
                }
                *point = x;
            }
            Metric::Maximum { start, end: stop }
            | Metric::Integral { start, end: stop }
            | Metric::Mean { start, end: stop }
            | Metric::Centroid { start, end: stop } => {
                if (end && x <= *start) || (!end && x >= *stop) {
                    return false;
                }
                let slot = if end { stop } else { start };
                if *slot == x {
                    return false;
                }
                *slot = x;
            }
        }
        true
    }
}

impl StudioApp {
    pub(crate) fn clear_measurement_handles(&mut self) {
        if self
            .handles
            .armed
            .is_some_and(|(p, _)| p == PLOT_MEASUREMENT)
        {
            self.handles.armed = None;
        }
        if self
            .handles
            .dragging
            .is_some_and(|(p, _)| p == PLOT_MEASUREMENT)
        {
            self.handles.dragging = None;
        }
    }

    fn editable_measurement_preview(&self) -> Option<&MeasurementPreview> {
        (self.stage == Stage::Series
            && !self.measurements.overview
            && !self.measurements.results
            && self.measurements.kind != 4
            && self.measurements.preview.is_some())
        .then_some(self.measurements.preview_data.as_ref())
        .flatten()
    }

    pub(crate) fn measurement_preview_entity(&self) -> Option<Entity<RuvizPlot>> {
        self.editable_measurement_preview()?;
        self.measurements.preview.clone()
    }

    pub(crate) fn measurement_handle_specs(&self) -> Vec<(HandleKey, f64)> {
        let Some(preview) = self.editable_measurement_preview() else {
            return vec![];
        };
        let (lo, hi) = preview.metric.bounds();
        let mut handles = vec![(HandleKey::MeasurementStart, lo)];
        if hi != lo {
            handles.push((HandleKey::MeasurementEnd, hi));
        }
        handles
    }

    pub(crate) fn measurement_handle_readout(&self, x: f64) -> String {
        let Some(preview) = self.editable_measurement_preview() else {
            return String::new();
        };
        let unit = match preview.space {
            MeasurementSpace::Chi { .. } => "Å⁻¹",
            MeasurementSpace::Fourier => "Å",
            _ if self.measurements.relative => "eV from E₀",
            _ => "eV",
        };
        format!("{:.2} {unit}", x - preview.origin)
    }

    pub(crate) fn drag_measurement_boundary(
        &mut self,
        key: HandleKey,
        x: f64,
        cx: &mut Context<Self>,
    ) {
        if self.editable_measurement_preview().is_none() {
            return;
        }
        let preview = self.measurements.preview_data.as_mut().unwrap();
        if !preview.drag(key == HandleKey::MeasurementEnd, x) {
            return;
        }
        let (lo, hi) = preview.field_bounds();
        for (field, value) in self.measurements.fields.iter().zip([lo, hi]) {
            field.update(cx, |field, cx| field.set_value(Some(value), cx));
        }
        // Cancel pending field previews, without invalidating this plot's session.
        self.measurements.preview_timer += 1;
        self.measurements.preview_label = match rexafs::xafs::analysis::metrics::measure(
            &preview.x,
            &preview.y,
            preview.metric,
        ) {
            Ok(value) => format!("{} · {:.6}", preview.label, value.value),
            Err(error) => format!("{} · {error}", preview.label),
        };
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preview(metric: Metric, space: MeasurementSpace, origin: f64) -> MeasurementPreview {
        MeasurementPreview {
            x: vec![origin, origin + 10.],
            y: vec![0., 10.],
            metric: metric.shifted(origin),
            space,
            origin,
            label: "test".into(),
        }
    }

    #[test]
    fn energy_drag_keeps_origin_and_recalculates_the_selected_interval() {
        let mut p = preview(
            Metric::Mean { start: 1., end: 5. },
            MeasurementSpace::Flat,
            10000.,
        );
        assert!(p.drag(true, 10008.));
        assert_eq!(p.metric.shifted(-p.origin).bounds(), (1., 8.));
        assert_eq!(
            rexafs::xafs::analysis::metrics::measure(&p.x, &p.y, p.metric)
                .unwrap()
                .value,
            4.5
        );
        assert!(p.drag(false, 10002.));
        assert_eq!(p.metric.shifted(-p.origin).bounds(), (2., 8.));
        assert!(!p.drag(false, 10009.));
        assert!(!p.drag(true, 10001.));
        assert!(p.drag(true, 10008.6));
        assert_eq!(p.field_bounds(), (2., 8.6));
    }

    #[test]
    fn drag_clamps_to_coverage_and_handles_points_and_nonenergy_units() {
        let mut p = preview(
            Metric::Point { x: 2. },
            MeasurementSpace::Chi { kweight: 2 },
            0.,
        );
        assert!(p.drag(false, 4.236));
        assert_eq!(p.metric.bounds(), (4.24, 4.24));
        assert!(p.drag(false, 30.));
        assert_eq!(p.metric.bounds(), (10., 10.));
        assert!(!p.drag(false, f64::NAN));
        assert!(p.drag(false, -30.));
        assert_eq!(p.metric.bounds(), (0., 0.));
        let mut p = preview(
            Metric::Integral { start: 1., end: 3. },
            MeasurementSpace::Fourier,
            0.,
        );
        assert!(p.drag(true, 3.128));
        assert_eq!(p.metric.bounds(), (1., 3.13));
    }
}
