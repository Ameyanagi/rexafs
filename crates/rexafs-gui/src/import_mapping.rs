//! Mapping drafts are independent of current selection and processing settings.
use std::hash::{Hash, Hasher};

use rexafs::prelude::XdiHeader;
use serde::{Deserialize, Serialize};

use crate::params::{DetectionMode, ImportConfig, ImportPreview, reference_mu_column};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AxisConversion {
    /// Preserve the released behavior: XDI metadata, otherwise an eV axis.
    #[default]
    Auto,
    EnergyEv,
    EnergyKev,
    AngleDegrees {
        d_spacing: f64,
    },
    AngleRadians {
        d_spacing: f64,
    },
}

impl AxisConversion {
    pub fn fingerprint(self) -> u64 {
        let mut hash = std::hash::DefaultHasher::new();
        std::mem::discriminant(&self).hash(&mut hash);
        if let Self::AngleDegrees { d_spacing } | Self::AngleRadians { d_spacing } = self {
            d_spacing.to_bits().hash(&mut hash);
        }
        hash.finish()
    }

    pub fn energy_ev(
        self,
        header: Option<&XdiHeader>,
        column: usize,
        value: f64,
    ) -> Result<f64, String> {
        match self {
            Self::Auto => header.map_or(Ok(value), |h| {
                h.energy_ev(column, value).map_err(|e| e.to_string())
            }),
            Self::EnergyEv => Ok(value),
            Self::EnergyKev => Ok(value * 1000.),
            Self::AngleDegrees { d_spacing } | Self::AngleRadians { d_spacing } => {
                if !d_spacing.is_finite() || d_spacing <= 0. {
                    return Err("Angle conversion requires positive finite d spacing (Å).".into());
                }
                let radians = if matches!(self, Self::AngleDegrees { .. }) {
                    value.to_radians()
                } else {
                    value
                };
                if !radians.is_finite() || radians <= 0. || radians > std::f64::consts::FRAC_PI_2 {
                    return Err(
                        "Monochromator angle must be greater than 0 and at most 90 degrees.".into(),
                    );
                }
                Ok(12398.419843320026 / (2. * d_spacing * radians.sin()))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColumnRole {
    Energy,
    I0,
    It,
    Ir,
    Mu,
}

/// The channel is frozen at opening. Add channel is a separate operation.
#[derive(Clone)]
pub struct MappingDraft {
    channel: DetectionMode,
    config: ImportConfig,
    detected: ImportConfig,
    pub revision: u64,
    pub column_count: usize,
    pub names: Option<Vec<String>>,
    pub xdi: Option<XdiHeader>,
}

impl MappingDraft {
    pub fn new(preview: &ImportPreview, config: &ImportConfig) -> Self {
        let channel = preview.resolved.mode;
        let mut effective = config.clone();
        effective.mode = channel;
        effective.energy_col = Some(preview.resolved.energy_col);
        effective.i0_col = Some(preview.resolved.i0_col);
        effective.it_col = Some(preview.resolved.it_col);
        effective.ir_col = Some(preview.resolved.ir_col);
        effective.fluor_cols = Some(preview.resolved.fluor_cols.clone());
        effective.mu_col = preview.resolved.mu_col;
        effective.reference_mu_col = if channel == DetectionMode::Reference {
            reference_mu_column(preview.names.as_deref(), config)
        } else {
            config.reference_mu_col
        };
        let detected = ImportConfig {
            mode: channel,
            energy_col: Some(preview.detected.energy_col),
            i0_col: Some(preview.detected.i0_col),
            it_col: Some(preview.detected.it_col),
            ir_col: Some(preview.detected.ir_col),
            fluor_cols: Some(preview.detected.fluor_cols.clone()),
            mu_col: preview.detected.mu_col,
            reference_mu_col: if channel == DetectionMode::Reference {
                reference_mu_column(preview.names.as_deref(), &ImportConfig::default())
            } else {
                None
            },
            ..Default::default()
        };
        Self {
            channel,
            config: effective,
            detected,
            revision: 0,
            column_count: preview.column_count,
            names: preview.names.clone(),
            xdi: preview.xdi.clone(),
        }
    }

    pub fn config(&self) -> &ImportConfig {
        &self.config
    }
    pub fn channel(&self) -> DetectionMode {
        self.channel
    }

    fn edit(&mut self, edit: impl FnOnce(&mut ImportConfig)) {
        let before = self.config.clone();
        edit(&mut self.config);
        if before != self.config {
            self.revision += 1;
        }
    }

    pub fn set_axis(&mut self, axis: AxisConversion) {
        self.edit(|c| c.axis = axis);
    }

    pub fn set_column(&mut self, role: ColumnRole, column: usize) {
        self.edit(|c| match role {
            ColumnRole::Energy => c.energy_col = Some(column),
            ColumnRole::I0 => c.i0_col = Some(column),
            ColumnRole::It => {
                c.it_col = Some(column);
                if c.mode == DetectionMode::Reference {
                    c.reference_mu_col = None;
                }
            }
            ColumnRole::Ir => {
                c.ir_col = Some(column);
                if c.mode == DetectionMode::Reference {
                    c.reference_mu_col = None;
                }
            }
            ColumnRole::Mu => {
                if c.mode == DetectionMode::Reference {
                    c.reference_mu_col = Some(column);
                } else {
                    c.mu_col = Some(column);
                }
            }
        });
    }

    pub fn toggle_roi(&mut self, column: usize) {
        self.edit(|c| {
            let columns = c.fluor_cols.get_or_insert_default();
            if columns.contains(&column) {
                columns.retain(|&c| c != column);
            } else {
                columns.push(column);
            }
            columns.sort_unstable();
            columns.dedup();
        });
    }

    pub fn use_detected(&mut self) {
        let detected = self.detected.clone();
        self.edit(|c| *c = detected);
    }

    pub fn column_label(&self, column: usize) -> String {
        self.names.as_ref().and_then(|n| n.get(column)).map_or_else(
            || format!("{}", column + 1),
            |name| format!("{}: {name}", column + 1),
        )
    }

    pub fn roles(&self) -> Vec<(String, Option<usize>)> {
        let c = &self.config;
        let mut roles = vec![("Energy".into(), c.energy_col)];
        match self.channel {
            DetectionMode::Transmission => {
                roles.extend([("I0".into(), c.i0_col), ("It".into(), c.it_col)])
            }
            DetectionMode::Reference if c.reference_mu_col.is_some() => {
                roles.push(("Reference μ".into(), c.reference_mu_col))
            }
            DetectionMode::Reference => {
                roles.extend([("It".into(), c.it_col), ("Ir".into(), c.ir_col)])
            }
            DetectionMode::Fluorescence => {
                roles.push(("I0".into(), c.i0_col));
                roles.extend(
                    c.fluor_cols
                        .iter()
                        .flatten()
                        .map(|&col| ("ROI".into(), Some(col))),
                );
            }
            DetectionMode::MuColumn => roles.push(("μ".into(), c.mu_col)),
            DetectionMode::Auto => {}
        }
        roles
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut used = std::collections::BTreeMap::new();
        for (role, col) in self.roles() {
            let col = col.ok_or_else(|| format!("Choose a column for {role}."))?;
            if col >= self.column_count {
                return Err(format!(
                    "{role} column {} exceeds the {} source columns.",
                    col + 1,
                    self.column_count
                ));
            }
            if let Some(previous) = used.insert(col, role.clone()) {
                return Err(format!(
                    "Column {} is assigned to both {previous} and {role}.",
                    col + 1
                ));
            }
        }
        if self.channel == DetectionMode::Fluorescence
            && self.config.fluor_cols.as_ref().is_none_or(Vec::is_empty)
        {
            return Err("Choose at least one fluorescence ROI.".into());
        }
        // Validate units/parameters without treating this as a full signal check.
        self.config
            .axis
            .energy_ev(self.xdi.as_ref(), self.config.energy_col.unwrap(), 1.)?;
        Ok(())
    }

    pub fn formula(&self) -> String {
        let c = &self.config;
        let name = |col: Option<usize>| col.map_or("?".into(), |c| self.column_label(c));
        let signal = match self.channel {
            DetectionMode::Transmission => format!("ln({} / {})", name(c.i0_col), name(c.it_col)),
            DetectionMode::Reference if c.reference_mu_col.is_some() => name(c.reference_mu_col),
            DetectionMode::Reference => format!("ln({} / {})", name(c.it_col), name(c.ir_col)),
            DetectionMode::Fluorescence => format!(
                "({}) / {}",
                c.fluor_cols
                    .iter()
                    .flatten()
                    .map(|&c| self.column_label(c))
                    .collect::<Vec<_>>()
                    .join(" + "),
                name(c.i0_col)
            ),
            DetectionMode::MuColumn => name(c.mu_col),
            DetectionMode::Auto => "?".into(),
        };
        format!("{}: μ(E) = {signal}", self.channel.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{PipelineParams, preview_import_raw};

    fn preview(
        text: &str,
        config: &ImportConfig,
    ) -> (ImportPreview, Result<crate::params::RawData, String>) {
        let path = std::env::temp_dir().join(format!(
            "rexafs-draft-{}-{}.dat",
            std::process::id(),
            GroupCounter::next()
        ));
        std::fs::write(&path, text).unwrap();
        let result = preview_import_raw(&path, config).unwrap();
        std::fs::remove_file(path).unwrap();
        result
    }
    struct GroupCounter;
    impl GroupCounter {
        fn next() -> usize {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        }
    }

    #[test]
    fn axis_conversion_retains_original_table_and_converts_once() {
        let text =
            "# XDI/1.0\n# Column.1: energy keV\n# Column.2: mutrans\n# ---\n8.9 .2\n9.0 .4\n";
        for axis in [AxisConversion::Auto, AxisConversion::EnergyKev] {
            let config = ImportConfig {
                axis,
                ..Default::default()
            };
            let (table, raw) = preview(text, &config);
            assert_eq!(table.rows[0], [8.9, 0.2]);
            assert_eq!(raw.unwrap().energy, [8900., 9000.]);
        }
        let (_, raw) = preview(
            "# energy mu\n8900 .2\n9000 .4\n",
            &ImportConfig {
                axis: AxisConversion::EnergyEv,
                ..Default::default()
            },
        );
        assert_eq!(raw.unwrap().energy, [8900., 9000.]);
        for (axis, rows) in [
            (
                AxisConversion::AngleDegrees { d_spacing: 3.1356 },
                "30 .2\n29 .4\n".to_string(),
            ),
            (
                AxisConversion::AngleRadians { d_spacing: 3.1356 },
                format!("{} .2\n{} .4\n", 30_f64.to_radians(), 29_f64.to_radians()),
            ),
        ] {
            let (table, raw) = preview(
                &format!("# angle mu\n{rows}"),
                &ImportConfig {
                    axis,
                    ..Default::default()
                },
            );
            let raw = raw.unwrap();
            assert!((raw.energy[0] - 3954.0821033677844).abs() < 1e-8);
            assert_eq!(raw.mu, [0.2, 0.4]);
            assert!(table.rows[0][0] < 31.);
        }
    }

    #[test]
    fn draft_roles_are_unique_and_reset_does_not_edit_source_config() {
        let config = ImportConfig {
            mode: DetectionMode::Fluorescence,
            ..Default::default()
        };
        let (table, raw) = preview(
            "# energy i0 it ir sdd1 sdd2\n8900 10 5 2 1 2\n9000 20 10 4 2 4\n",
            &config,
        );
        assert_eq!(raw.unwrap().mu, [0.3, 0.3]);
        let mut draft = MappingDraft::new(&table, &config);
        assert!(draft.validate().is_ok());
        draft.toggle_roi(4);
        draft.toggle_roi(4);
        assert_eq!(draft.config().fluor_cols, Some(vec![4, 5]));
        draft.set_column(ColumnRole::I0, 4);
        assert!(draft.validate().unwrap_err().contains("both"));
        draft.use_detected();
        assert!(draft.validate().is_ok());
        assert!(draft.formula().contains("5: sdd1 + 6: sdd2"));
        assert_eq!(config.fluor_cols, None);
        draft.toggle_roi(4);
        draft.toggle_roi(5);
        assert!(draft.validate().is_err());
    }

    #[test]
    fn reference_uses_explicit_mu_or_intensities_and_full_signal() {
        let text = "# energy i0 it ir murefer\n8900 10 5 2 .7\n9000 20 10 4 .8\n9010 10 0 0 NaN\n";
        for (config, expected) in [
            (
                ImportConfig {
                    mode: DetectionMode::Transmission,
                    ..Default::default()
                },
                vec![2_f64.ln(), 2_f64.ln()],
            ),
            (
                ImportConfig {
                    mode: DetectionMode::Reference,
                    ..Default::default()
                },
                vec![0.7, 0.8],
            ),
            (
                ImportConfig {
                    mode: DetectionMode::Reference,
                    it_col: Some(2),
                    ir_col: Some(3),
                    ..Default::default()
                },
                vec![2.5_f64.ln(), 2.5_f64.ln()],
            ),
        ] {
            let (table, raw) = preview(text, &config);
            let raw = raw.unwrap();
            assert_eq!(raw.mu, expected);
            assert_eq!(raw.diagnostics.excluded_signal_points.count, 1);
            let draft = MappingDraft::new(&table, &config);
            assert!(draft.validate().is_ok());
            assert_eq!(preview(text, draft.config()).1.unwrap(), raw);
        }
    }

    #[test]
    fn axis_settings_persist_and_invalidate_raw_cache() {
        let original = PipelineParams::default();
        let mut changed = original.clone();
        changed.import.axis = AxisConversion::EnergyKev;
        assert_ne!(changed.raw_fingerprint(), original.raw_fingerprint());
        let restored: PipelineParams =
            serde_json::from_str(&serde_json::to_string(&changed).unwrap()).unwrap();
        assert_eq!(restored.raw_fingerprint(), changed.raw_fingerprint());
        assert_eq!(
            serde_json::from_str::<ImportConfig>("{}").unwrap().axis,
            AxisConversion::Auto
        );
        assert!(
            AxisConversion::AngleDegrees { d_spacing: 0. }
                .energy_ev(None, 0, 30.)
                .is_err()
        );
    }
}
