//! Named, unit-bearing peak summaries shared by charts and numerical export.
use super::*;
use rexafs::prelude::{MeasurementSpace, PeakShape};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeakMetric {
    Center(String),
    Area(String),
    Height(String),
    Fwhm(String),
    WeightedCenter,
    Objective,
    Parameter(String),
}

impl PeakMetric {
    pub fn choices(model: &PeakFit) -> Vec<Self> {
        let mut choices = Vec::new();
        for c in &model.components {
            match c.shape {
                PeakShape::Constant | PeakShape::Linear => continue,
                PeakShape::ErfStep | PeakShape::ArctanStep => {
                    choices.push(Self::Center(c.name.clone()));
                }
                _ => choices.extend([
                    Self::Area(c.name.clone()),
                    Self::Center(c.name.clone()),
                    Self::Height(c.name.clone()),
                    Self::Fwhm(c.name.clone()),
                ]),
            }
        }
        choices.extend([Self::WeightedCenter, Self::Objective]);
        choices.extend(model.parameters.vars.keys().cloned().map(Self::Parameter));
        choices
    }
    pub fn key(&self) -> String {
        match self {
            Self::Center(n) => format!("{n}_center_absolute_eV"),
            Self::Area(n) => format!("{n}_whole_axis_area"),
            Self::Height(n) => format!("{n}_peak_height"),
            Self::Fwhm(n) => format!("{n}_combined_fwhm_eV"),
            Self::WeightedCenter => "peak_area_weighted_center_eV".into(),
            Self::Objective => "objective".into(),
            Self::Parameter(n) => n.clone(),
        }
    }
    pub fn unit(&self, model: &PeakFit) -> String {
        let signal = if model.space == MeasurementSpace::Mu {
            "μ units"
        } else {
            "dimensionless"
        };
        match self {
            Self::Center(_) | Self::Fwhm(_) | Self::WeightedCenter => "eV".into(),
            Self::Area(_) => {
                if signal == "dimensionless" {
                    "eV".into()
                } else {
                    "μ units × eV".into()
                }
            }
            Self::Height(_) => signal.into(),
            Self::Objective => format!("({signal})²"), // Desktop fits are unweighted.
            Self::Parameter(name) => {
                let role = model
                    .components
                    .iter()
                    .flat_map(|c| &c.parameters)
                    .find_map(|(role, n)| (n == name).then_some(role.as_str()));
                match role {
                    Some("center" | "width" | "lorentz_width" | "scale" | "reference") => {
                        "eV".into()
                    }
                    Some("area") => Self::Area(String::new()).unit(model),
                    Some("fraction") => "dimensionless".into(),
                    Some("slope") => format!("{signal}/eV"),
                    _ => signal.into(),
                }
            }
        }
    }
    pub fn label(&self, model: &PeakFit) -> String {
        let title = match self {
            Self::Center(n) => format!("{n} · center"),
            Self::Area(n) => format!("{n} · area"),
            Self::Height(n) => format!("{n} · height"),
            Self::Fwhm(n) => format!("{n} · FWHM"),
            Self::WeightedCenter => "Peak-area-weighted center".into(),
            Self::Objective => "Residual sum of squares".into(),
            Self::Parameter(n) => format!("Parameter · {n}"),
        };
        format!("{title} ({})", self.unit(model))
    }
    pub fn value(&self, summary: &PeakSummary) -> (Option<f64>, Option<f64>) {
        match self {
            Self::WeightedCenter => (
                summary.peak_center_ev,
                summary.peak_center_standard_error_ev,
            ),
            Self::Objective => (Some(summary.objective), None),
            Self::Parameter(n) => summary
                .parameters
                .vars
                .get(n)
                .map(|p| (Some(p.value), p.stderr))
                .unwrap_or_default(),
            Self::Center(n) | Self::Area(n) | Self::Height(n) | Self::Fwhm(n) => {
                let Some(c) = summary.components.iter().find(|c| &c.name == n) else {
                    return (None, None);
                };
                match self {
                    Self::Center(_) => (c.center_ev, c.center_standard_error_ev),
                    Self::Area(_) => (c.area, c.area_standard_error),
                    Self::Height(_) => (c.height, c.height_standard_error),
                    _ => (c.fwhm_ev, c.fwhm_standard_error_ev),
                }
            }
        }
    }
}

impl PeakRun {
    /// Use physical coordinates only when every retained frame supplies one.
    /// Otherwise preserve the acquisition sequence; never invent coordinates.
    pub fn plot_coordinates(&self) -> (Vec<f64>, String) {
        if let Some(s) = &self.series {
            let coordinates: Option<Vec<_>> = s
                .frames
                .iter()
                .map(|f| f.coordinate.filter(|v| v.is_finite()))
                .collect();
            if let Some(x) = coordinates.filter(|x| x.len() == self.rows.len()) {
                return (x, format!("{} ({})", s.coordinate.label, s.coordinate.unit));
            }
        }
        (
            self.rows.iter().map(|r| r.sequence as f64).collect(),
            "Frame sequence".into(),
        )
    }
}
