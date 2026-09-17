//! Transform-stage local Fourier maps. No fitting or scattering runs here.
use rexafs::{Spectrum, transform::*};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Component {
    #[default]
    Magnitude,
    Real,
    Imaginary,
}
impl Component {
    pub const ALL: [Self; 3] = [Self::Magnitude, Self::Real, Self::Imaginary];
    pub fn label(self) -> &'static str {
        match self {
            Self::Magnitude => "Magnitude",
            Self::Real => "Real",
            Self::Imaginary => "Imaginary",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub rmin: f64,
    pub rmax: f64,
    pub omega0: f64,
    pub centers: usize,
    pub distances: usize,
    pub component: Component,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            rmin: 0.25,
            rmax: 6.,
            omega0: 6.,
            centers: 64,
            distances: 64,
            component: Component::Magnitude,
        }
    }
}
pub struct Map {
    pub k: Vec<f64>,
    pub r: Vec<f64>,
    pub magnitude: Vec<Vec<f64>>,
    pub real: Vec<Vec<f64>>,
    pub imaginary: Vec<Vec<f64>>,
    pub kweight: f64,
    pub uses_fft: bool,
}
impl Map {
    pub fn values(&self, component: Component) -> &[Vec<f64>] {
        match component {
            Component::Magnitude => &self.magnitude,
            Component::Real => &self.real,
            Component::Imaginary => &self.imaginary,
        }
    }
}

/// Read existing unweighted χ(k), select measured support and apply k weighting
/// exactly once. Does not run AUTOBK or alter Spectrum caches. Display grids are
/// bounded; irregular input is transformed directly without implicit resampling.
pub fn calculate(
    spectrum: &Spectrum,
    settings: &Settings,
    range: [f64; 2],
    kweight: f64,
) -> Result<Map, String> {
    if ![
        settings.rmin,
        settings.rmax,
        settings.omega0,
        range[0],
        range[1],
        kweight,
    ]
    .iter()
    .all(|v| v.is_finite())
        || settings.rmin <= 0.
        || settings.rmin >= settings.rmax
        || settings.omega0 <= 0.
        || range[0] < 0.
        || range[0] >= range[1]
        || !(0. ..=3.).contains(&kweight)
        || !(2..=128).contains(&settings.centers)
        || !(2..=128).contains(&settings.distances)
    {
        return Err("Set positive increasing R bounds, a positive Morlet width, 2–128 grid points and a k weight from 0 to 3.".into());
    }
    let k = spectrum
        .k()
        .ok_or("Prepare χ(k) in Background before calculating a wavelet map.")?;
    let chi = spectrum
        .chi()
        .ok_or("The processed spectrum has no χ(k).")?;
    if k.len() != chi.len() {
        return Err("The processed k and χ arrays have different lengths.".into());
    }
    let samples: Vec<_> = k
        .iter()
        .copied()
        .zip(chi.iter().copied())
        .filter(|(q, _)| *q >= range[0] && *q <= range[1])
        .collect();
    if samples.len() < 2 {
        return Err("The selected k range contains fewer than two samples.".into());
    }
    let grid: Vec<_> = samples.iter().map(|s| s.0).collect();
    let values: Vec<_> = samples.iter().map(|&(q, x)| q.powf(kweight) * x).collect();
    let step = grid[1] - grid[0];
    let uniform = grid
        .iter()
        .enumerate()
        .all(|(i, q)| (q - (grid[0] + i as f64 * step)).abs() <= 1e-10 * q.abs().max(1.));
    let centers: Vec<_> = if uniform {
        let stride = (grid.len() - 1).div_ceil(settings.centers - 1).max(1);
        grid.iter().step_by(stride).copied().collect()
    } else {
        (0..settings.centers)
            .map(|i| {
                grid[0]
                    + (grid[grid.len() - 1] - grid[0]) * i as f64 / (settings.centers - 1) as f64
            })
            .collect()
    };
    let r: Vec<_> = (0..settings.distances)
        .map(|i| {
            settings.rmin
                + (settings.rmax - settings.rmin) * i as f64 / (settings.distances - 1) as f64
        })
        .collect();
    let transform = LocalSpectrumTransform::new(
        &grid,
        &LocalSpectrumSettings::morlet(WaveletSettings {
            k_centers: centers.clone(),
            r: r.clone(),
            omega0: settings.omega0,
        }),
    )
    .map_err(|e| e.to_string())?;
    let coefficients = transform.transform(&values).map_err(|e| e.to_string())?;
    let mut magnitude = vec![vec![0.; centers.len()]; r.len()];
    let mut real = magnitude.clone();
    let mut imaginary = magnitude.clone();
    for ci in 0..centers.len() {
        for ri in 0..r.len() {
            let z = coefficients[ci * r.len() + ri];
            magnitude[ri][ci] = z.norm();
            real[ri][ci] = z.re;
            imaginary[ri][ci] = z.im;
        }
    }
    Ok(Map {
        k: centers,
        r,
        magnitude,
        real,
        imaginary,
        kweight,
        uses_fft: transform.uses_fft(),
    })
}

pub fn plot(map: &Map, component: Component, theme: crate::theme::Theme) -> ruviz::prelude::Plot {
    use ruviz::{
        plots::heatmap::{HeatmapConfig, HeatmapOrigin},
        prelude::Plot,
    };
    let dx = (map.k[map.k.len() - 1] - map.k[0]) / (map.k.len() - 1) as f64 / 2.;
    let dy = (map.r[map.r.len() - 1] - map.r[0]) / (map.r.len() - 1) as f64 / 2.;
    let config = HeatmapConfig::new()
        .colorbar(true)
        .origin(HeatmapOrigin::Lower)
        .extent(
            map.k[0] - dx,
            map.k[map.k.len() - 1] + dx,
            map.r[0] - dy,
            map.r[map.r.len() - 1] + dy,
        );
    let config = if component != Component::Magnitude {
        let max = map
            .values(component)
            .iter()
            .flatten()
            .fold(1e-15_f64, |a, b| a.max(b.abs()));
        config.vmin(-max).vmax(max)
    } else {
        config.vmin(0.)
    };
    Plot::new()
        .theme(theme.plot_theme())
        .xlabel("k (Å⁻¹)")
        .ylabel("R (Å; Fourier distance)")
        .title(format!(
            "Morlet · {} · k weight {}",
            component.label(),
            map.kweight
        ))
        .heatmap_with(map.values(component), config)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn map_uses_processed_data_without_mutating_spectrum_and_transposes_truthfully() {
        let spectrum = crate::rmc_fitting::tests::spectrum();
        let before = serde_json::to_value(&spectrum).unwrap();
        let map = calculate(&spectrum, &Settings::default(), [3., 10.], 2.).unwrap();
        assert!(map.k.len() >= 2);
        assert_eq!(map.magnitude.len(), 64);
        assert_eq!(map.real[0].len(), map.k.len());
        for ri in 0..map.r.len() {
            for ci in 0..map.k.len() {
                assert!(
                    (map.magnitude[ri][ci] - map.real[ri][ci].hypot(map.imaginary[ri][ci])).abs()
                        < 1e-12
                );
            }
        }
        assert_eq!(serde_json::to_value(&spectrum).unwrap(), before);
        let invalid = Settings {
            rmin: 0.,
            ..Default::default()
        };
        assert!(calculate(&spectrum, &invalid, [3., 10.], 2.).is_err());
    }
}
