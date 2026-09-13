use easyfft::prelude::DynRealFft;
use easyfft::{dyn_size::realfft::DynRealDft, num_complex::Complex};
use nalgebra::DVector;
use serde::{Deserialize, Serialize};

use super::errors::FFTError;
use super::mathutils::MathUtils;
use super::xafsutils::{ftwindow, FTWindow};

pub use super::fft_grid::FFTGrid;

/// Forward EXAFS transform from chi(k) to complex chi(R).
///
/// The transform is an unnormalized negative-exponent FFT multiplied by
/// `kstep / sqrt(pi)`; no additional `1/nfft`, phase factor, or window-area
/// normalization is applied. Use `new()` for recommended settings, then `xftf`
/// to calculate. Mutating these standalone fields does not recalculate caches;
/// use `Spectrum::set_fft` when managing a spectrum pipeline.
///
/// The sign and FFT normalization match the
/// [NumPy convention](https://numpy.org/doc/stable/reference/routines.fft.html#normalization).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct XrayFFTF {
    /// Sampling and window domain. Default: `FFTGrid::Input` in the nalgebra backend.
    /// Input preserves samples; Larch resamples on a uniform grid starting at zero.
    pub grid: FFTGrid,
    /// Maximum displayed R in Å; default 10.0. The full transform is retained for
    /// inverse filtering, so this does not set an R-space filter cutoff.
    pub rmax_out: Option<f64>,
    /// Window family; default `Some(FTWindow::KaiserBessel)`. `None` selects
    /// the window helper's Hanning default, not an all-pass window.
    pub window: Option<FTWindow>,
    /// Low-side window parameter; default 1.0. Usually a taper width in Å⁻¹;
    /// Kaiser–Bessel also uses its numerical value as a shape parameter. Gaussian
    /// uses it as the width, and fractional Hanning uses fractional taper geometry.
    pub dk: Option<f64>,
    /// High-side window parameter; `None` uses `dk`. Interpretation depends on
    /// window family; equal values do not make different families equivalent.
    pub dk2: Option<f64>,
    /// Lower window boundary in Å⁻¹; default 2.0. `None` uses the first input k.
    pub kmin: Option<f64>,
    /// Upper window boundary in Å⁻¹; default 15.0. `None` uses the last input k.
    /// In Larch mode, the window domain extends through `kmax + dk2`.
    pub kmax: Option<f64>,
    /// Power of k multiplying chi before transformation; default 2.0.
    /// Nonnegative fractional values are floored. Larger values emphasize high-k
    /// signal and noise; this weighting is separate from AUTOBK's own k-weight.
    pub kweight: Option<f64>,
    /// FFT length; default 2048. Short input is zero-padded; Input mode truncates
    /// longer input to this length. Larch mode requires room for its extended window.
    /// More zero-padding refines the R grid without adding experimental resolution.
    pub nfft: Option<usize>,
    /// FFT sample spacing in Å⁻¹; `None` infers `k[1] - k[0]`. This controls both
    /// R spacing `pi / (nfft * kstep)` and amplitude `kstep / sqrt(pi)`.
    /// AUTOBK normally supplies 0.05 Å⁻¹; Input mode does not resample when this changes.
    pub kstep: Option<f64>,
    /// Displayed R grid in Å, populated by `xftf`; `None` before calculation.
    pub r: Option<DVector<f64>>,
    /// Full complex real-FFT coefficients, including DC and the even-length Nyquist
    /// bin. For dimensionless chi and k-weight w, units are Å⁻⁽ʷ⁺¹⁾. This cache is
    /// not truncated by `rmax_out`; `None` before calculation.
    pub chir: Option<DynRealDft<f64>>,
    /// Magnitude of the displayed complex transform, paired with `r`.
    /// Units are Å⁻⁽ʷ⁺¹⁾ for dimensionless chi; `None` before calculation.
    pub chir_mag: Option<DVector<f64>>,
    /// Dimensionless window on the prepared k grid, before k-weight multiplication.
    /// Use `Spectrum::kwin_k` to obtain the matching axis; `None` before calculation.
    pub kwin: Option<DVector<f64>>,
}

// Preserve the original equality contract: the complex FFT cache is omitted.
impl PartialEq for XrayFFTF {
    fn eq(&self, other: &Self) -> bool {
        self.grid == other.grid
            && self.rmax_out == other.rmax_out
            && self.window == other.window
            && self.dk == other.dk
            && self.dk2 == other.dk2
            && self.kmin == other.kmin
            && self.kmax == other.kmax
            && self.kweight == other.kweight
            && self.nfft == other.nfft
            && self.kstep == other.kstep
            && self.r == other.r
            && self.chir_mag == other.chir_mag
            && self.kwin == other.kwin
    }
}

impl Default for XrayFFTF {
    fn default() -> Self {
        Self {
            grid: FFTGrid::Input,
            rmax_out: Some(10.0),
            window: Some(FTWindow::KaiserBessel),
            dk: Some(1.0),
            dk2: None,
            kmin: Some(2.0),
            kmax: Some(15.0),
            kweight: Some(2.0),
            nfft: Some(2048),
            kstep: None,
            r: None,
            chir: None,
            chir_mag: None,
            kwin: None,
        }
    }
}

impl XrayFFTF {
    /// Create recommended forward settings with automatic input spacing.
    pub fn new() -> XrayFFTF {
        Self::default()
    }

    /// Resolve automatic settings from a nonempty k grid without calculating.
    ///
    /// Prefer the validated transform method for public input. This lower-level
    /// helper indexes the supplied grid directly and can panic on an empty grid.
    pub fn fill_parameter(&mut self, k: &DVector<f64>) -> &mut Self {
        if self.kweight.is_none() {
            self.kweight = Some(2.0);
        }

        self.kweight = Some(self.kweight.unwrap().max(0.0).floor());

        if self.kstep.is_none() {
            self.kstep = Some(if k.len() > 1 { k[1] - k[0] } else { 0.05 });
        }

        if self.kmin.is_none() {
            self.kmin = Some(k[0]);
        }

        if self.kmax.is_none() {
            self.kmax = Some(k[k.len() - 1]);
        }

        if self.dk.is_none() {
            self.dk = Some(1.0);
        }

        if self.dk2.is_none() {
            self.dk2 = self.dk;
        }

        if self.nfft.is_none() {
            self.nfft = Some(2048);
        }

        if self.rmax_out.is_none() {
            self.rmax_out = Some(10.0);
        }

        self
    }

    /// Calculate and cache chi(R) from equally sized finite k/chi arrays.
    ///
    /// At least two strictly increasing k samples are required. For a physical R
    /// interpretation, Input mode requires a zero-origin uniform k grid consistent
    /// with `kstep`; it does not enforce uniformity or correct a nonzero origin.
    /// Larch mode resamples from zero. Invalid settings, insufficient input or an
    /// undersized Larch transform return `FFTError`. Inputs are borrowed unchanged.
    pub fn xftf(&mut self, k: &DVector<f64>, chi: &DVector<f64>) -> Result<&mut Self, FFTError> {
        if self.nfft.is_some_and(|n| n < 2) {
            return Err(FFTError::InvalidParameter {
                parameter: "nfft".into(),
                reason: "must be at least 2".into(),
            });
        }
        for (name, value, strictly_positive) in [
            ("kstep", self.kstep, true),
            ("kweight", self.kweight, false),
            ("dk", self.dk, false),
            ("dk2", self.dk2, false),
            ("rmax_out", self.rmax_out, false),
        ] {
            if value.is_some_and(|v| !v.is_finite() || v < 0.0 || (strictly_positive && v == 0.0)) {
                return Err(FFTError::InvalidParameter {
                    parameter: name.into(),
                    reason: "must be finite and nonnegative (kstep must be positive)".into(),
                });
            }
        }
        if self.kmin.is_some_and(|v| !v.is_finite())
            || self.kmax.is_some_and(|v| !v.is_finite())
            || self.kmin.zip(self.kmax).is_some_and(|(lo, hi)| lo >= hi)
        {
            return Err(FFTError::InvalidParameter {
                parameter: "kmin/kmax".into(),
                reason: "must be finite with kmin < kmax".into(),
            });
        }
        if k.len() != chi.len() {
            return Err(FFTError::InterpolationFailed {
                reason: "k/chi length mismatch".to_string(),
            });
        }
        if k.len() < 2 {
            return Err(FFTError::InsufficientPoints {
                min: 2,
                actual: k.len(),
                kmin: 0.0,
                kmax: 0.0,
            });
        }

        if k.iter().chain(chi.iter()).any(|v| !v.is_finite())
            || k.as_slice().windows(2).any(|w| w[0] >= w[1])
        {
            return Err(FFTError::InvalidParameter {
                parameter: "k/chi".into(),
                reason: "must be finite with strictly increasing k".into(),
            });
        }

        self.fill_parameter(k);
        let nfft = self.nfft.unwrap();
        let (mut chi_weighted, win) = self.prepare(k, chi)?;

        for i in 0..chi_weighted.len() {
            chi_weighted[i] *= win[i];
        }

        let cchi_fft = xftf_fast_nalgebra(&chi_weighted, nfft, self.kstep.unwrap());
        let rstep = std::f64::consts::PI / self.kstep.unwrap() / nfft as f64;
        let irmax = (nfft / 2 + 1)
            .min((1.01 + self.rmax_out.unwrap() / rstep) as usize)
            .max(1);

        self.r = Some(linspace(0.0, (irmax - 1) as f64 * rstep, irmax));
        self.chir_mag = Some(DVector::from_iterator(
            irmax,
            cchi_fft.iter().take(irmax).map(|x| x.norm()),
        ));
        self.kwin = Some(win);
        self.chir = Some(cchi_fft);

        Ok(self)
    }

    fn prepare(
        &self,
        k: &DVector<f64>,
        chi: &DVector<f64>,
    ) -> Result<(DVector<f64>, DVector<f64>), FFTError> {
        let kweight = self.kweight.unwrap();
        let grid_owned;
        let chi_owned;
        let (grid, chi, npts) = match self.grid {
            FFTGrid::Input => (k, chi, k.len()),
            FFTGrid::Larch => {
                let step = self.kstep.unwrap();
                let last = k[k.len() - 1];
                let npts = (1.01 + last / step).floor();
                let extent = last.max(self.kmax.unwrap() + self.dk2.unwrap());
                let nwin = (1.01 + extent / step).floor();
                // Bound both arrays before float-to-integer conversion/allocation.
                if k[0] < 0.0 || !nwin.is_finite() || npts < 2.0 || nwin > self.nfft.unwrap() as f64
                {
                    return Err(FFTError::InvalidParameter {
                        parameter: "Larch grid".into(),
                        reason: "requires nonnegative k, at least two resampled points, and nfft large enough for kmax+dk2".into(),
                    });
                }
                grid_owned = DVector::from_iterator(
                    nwin as usize,
                    (0..nwin as usize).map(|i| i as f64 * step),
                );
                chi_owned = grid_owned
                    .interpolate(k.as_slice(), chi.as_slice())
                    .map_err(|e| FFTError::InterpolationFailed {
                        reason: e.to_string(),
                    })?;
                (&grid_owned, &chi_owned, npts as usize)
            }
        };
        let window =
            ftwindow(grid, self.kmin, self.kmax, self.dk, self.dk2, self.window).map_err(|e| {
                FFTError::WindowCalculationFailed {
                    reason: e.to_string(),
                }
            })?;
        let weighted =
            DVector::from_iterator(npts, (0..npts).map(|i| chi[i] * grid[i].powf(kweight)));
        let window = if npts == window.len() {
            window
        } else {
            window.rows(0, npts).into_owned()
        };
        Ok((weighted, window))
    }

    /// Read the stored value without recalculating.
    ///
    /// Maximum displayed R in Å; default 10.0. The full transform is retained for
    /// inverse filtering, so this does not set an R-space filter cutoff.
    pub fn get_rmax_out(&self) -> Option<&f64> {
        self.rmax_out.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Window family; default `Some(FTWindow::KaiserBessel)`. `None` selects
    /// the window helper's Hanning default, not an all-pass window.
    pub fn get_window(&self) -> Option<&FTWindow> {
        self.window.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Low-side window parameter; default 1.0. Usually a taper width in Å⁻¹;
    /// Kaiser–Bessel also uses its numerical value as a shape parameter. Gaussian
    /// uses it as the width, and fractional Hanning uses fractional taper geometry.
    pub fn get_dk(&self) -> Option<&f64> {
        self.dk.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// High-side window parameter; `None` uses `dk`. Interpretation depends on
    /// window family; equal values do not make different families equivalent.
    pub fn get_dk2(&self) -> Option<&f64> {
        self.dk2.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Lower window boundary in Å⁻¹; default 2.0. `None` uses the first input k.
    pub fn get_kmin(&self) -> Option<&f64> {
        self.kmin.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Upper window boundary in Å⁻¹; default 15.0. `None` uses the last input k.
    /// In Larch mode, the window domain extends through `kmax + dk2`.
    pub fn get_kmax(&self) -> Option<&f64> {
        self.kmax.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Power of k multiplying chi before transformation; default 2.0.
    /// Nonnegative fractional values are floored. Larger values emphasize high-k
    /// signal and noise; this weighting is separate from AUTOBK's own k-weight.
    pub fn get_kweight(&self) -> Option<&f64> {
        self.kweight.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Displayed R grid in Å, populated by `xftf`; `None` before calculation.
    pub fn get_r(&self) -> Option<&DVector<f64>> {
        self.r.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Full complex real-FFT coefficients, including DC and the even-length Nyquist
    /// bin. For dimensionless chi and k-weight w, units are Å⁻⁽ʷ⁺¹⁾. This cache is
    /// not truncated by `rmax_out`; `None` before calculation.
    pub fn get_chir(&self) -> Option<&DynRealDft<f64>> {
        self.chir.as_ref()
    }

    /// Return a newly allocated real component on the displayed R grid.
    /// Units are Å⁻⁽ʷ⁺¹⁾ for dimensionless chi. Returns `None` before calculation.
    pub fn get_chir_real(&self) -> Option<DVector<f64>> {
        let len_r = self.r.as_ref()?.len();
        let chir = self.chir.as_ref()?;
        Some(DVector::from_iterator(
            len_r,
            chir.iter().take(len_r).map(|x| x.re),
        ))
    }

    /// Return a newly allocated imaginary component on the displayed R grid.
    /// Units are Å⁻⁽ʷ⁺¹⁾ for dimensionless chi. Returns `None` before calculation.
    pub fn get_chir_imag(&self) -> Option<DVector<f64>> {
        let len_r = self.r.as_ref()?.len();
        let chir = self.chir.as_ref()?;
        Some(DVector::from_iterator(
            len_r,
            chir.iter().take(len_r).map(|x| x.im),
        ))
    }

    /// Read the stored value without recalculating.
    ///
    /// Magnitude of the displayed complex transform, paired with `r`.
    /// Units are Å⁻⁽ʷ⁺¹⁾ for dimensionless chi; `None` before calculation.
    pub fn get_chir_mag(&self) -> Option<&DVector<f64>> {
        self.chir_mag.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Dimensionless window on the prepared k grid, before k-weight multiplication.
    /// Use `Spectrum::kwin_k` to obtain the matching axis; `None` before calculation.
    pub fn get_kwin(&self) -> Option<&DVector<f64>> {
        self.kwin.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// FFT sample spacing in Å⁻¹; `None` infers `k[1] - k[0]`. This controls both
    /// R spacing `pi / (nfft * kstep)` and amplitude `kstep / sqrt(pi)`.
    /// AUTOBK normally supplies 0.05 Å⁻¹; Input mode does not resample when this changes.
    pub fn get_kstep(&self) -> Option<&f64> {
        self.kstep.as_ref()
    }
}

/// Real inverse EXAFS transform with an R-space filter.
///
/// The inverse restores conjugate symmetry and applies `sqrt(pi)/(kstep*nfft)`
/// to an unnormalized inverse FFT. An all-pass filter with zero R-weight recovers
/// the forward weighted/windowed signal, not unweighted chi. This real-output
/// convention differs from Larch's complex analytic inverse.
///
/// Change settings through `Spectrum::set_ifft` to invalidate dependent caches.
/// Direct field mutation requires an explicit `xftr` call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct XrayFFTR {
    /// Maximum returned q in Å⁻¹; default 10.0, bounded by the inverse array length.
    pub qmax_out: Option<f64>,
    /// R-window family; default Kaiser–Bessel. `None` selects Hanning, not all-pass.
    pub window: Option<FTWindow>,
    /// Low-side R-window parameter; default 1.0. Usually a width in Å; Kaiser–Bessel
    /// also uses its numerical value for shape, and fractional Hanning uses fractions.
    pub dr: Option<f64>,
    /// High-side R-window parameter; `None` uses `dr`. Its meaning depends on the family.
    pub dr2: Option<f64>,
    /// Lower R-window boundary in Å; default 0.0. `None` uses the first supplied R.
    pub rmin: Option<f64>,
    /// Upper R-window boundary in Å; default 20.0. `None` uses the last supplied R.
    pub rmax: Option<f64>,
    /// Additional power of R applied before inversion; default 0.0.
    /// Nonnegative fractional values are floored. This does not remove forward k-weighting.
    pub rweight: Option<f64>,
    /// Inverse FFT length; default 2048. Changing it resizes the positive-frequency
    /// coefficients and changes automatic q spacing; leave `kstep` automatic.
    pub nfft: Option<usize>,
    /// Output q spacing in Å⁻¹. `None` infers `pi / (R_step * nfft)`.
    /// An explicit positive value must agree with the supplied uniform R grid.
    pub kstep: Option<f64>,
    /// Returned q grid in Å⁻¹, beginning at zero; `None` before calculation.
    pub q: Option<DVector<f64>>,
    /// Full real inverse signal before the `qmax_out` display limit. It retains
    /// forward weighting/windowing. Units are Å⁽ᵘ⁻ʷ⁾ for dimensionless input chi,
    /// forward k-weight w and inverse R-weight u; `None` before calculation.
    pub chiq: Option<DVector<f64>>,
    /// Full R-window multiplied by `R.powf(rweight)`, paired with all positive
    /// forward bins, not just the displayed R range. It is dimensionless only when
    /// `rweight=0`; `None` before calculation.
    pub rwin: Option<DVector<f64>>,
}

impl Default for XrayFFTR {
    fn default() -> Self {
        Self {
            qmax_out: Some(10.0),
            window: Some(FTWindow::KaiserBessel),
            dr: Some(1.0),
            dr2: None,
            rmin: Some(0.0),
            rmax: Some(20.0),
            rweight: Some(0.0),
            nfft: Some(2048),
            kstep: None,
            q: None,
            chiq: None,
            rwin: None,
        }
    }
}

impl XrayFFTR {
    /// Create recommended inverse settings with automatic q spacing.
    pub fn new() -> XrayFFTR {
        Self::default()
    }

    /// Resolve automatic settings from a nonempty r grid without calculating.
    ///
    /// Prefer the validated transform method for public input. This lower-level
    /// helper indexes the supplied grid directly and can panic on an empty grid.
    pub fn fill_parameter(&mut self, r: &DVector<f64>) -> &mut Self {
        if self.rweight.is_none() {
            self.rweight = Some(0.0);
        }

        self.rweight = Some(self.rweight.unwrap().max(0.0).floor());

        if self.rmin.is_none() {
            self.rmin = Some(r[0]);
        }

        if self.rmax.is_none() {
            self.rmax = Some(r[r.len() - 1]);
        }

        if self.dr.is_none() {
            self.dr = Some(1.0);
        }

        if self.nfft.is_none() {
            self.nfft = Some(2048);
        }

        if self.qmax_out.is_none() {
            self.qmax_out = Some(10.0);
        }

        if self.kstep.is_none() {
            self.kstep = Some(if r.len() > 1 {
                std::f64::consts::PI / (r[1] - r[0]) / self.nfft.unwrap() as f64
            } else {
                0.05
            });
        }
        self
    }

    /// Filter full complex forward coefficients and cache a real inverse signal.
    ///
    /// `r` must contain at least two uniformly increasing finite samples starting
    /// at zero. Explicit `kstep` and `nfft` must match its spacing. Invalid grids,
    /// negative/nonfinite parameters or `rmin >= rmax` return `FFTError`. The input
    /// coefficients are borrowed unchanged; `rmax_out` never truncates this filter.
    pub fn xftr(
        &mut self,
        r: &DVector<f64>,
        chir: &DynRealDft<f64>,
    ) -> Result<&mut Self, FFTError> {
        super::inverse_fft::validate(r.as_slice(), self)?;
        self.fill_parameter(r);
        let rstep = std::f64::consts::PI / self.kstep.unwrap() / self.nfft.unwrap() as f64;
        let full_r = DVector::from_iterator(chir.len(), (0..chir.len()).map(|i| i as f64 * rstep));
        let mut win = ftwindow(
            &full_r,
            self.rmin,
            self.rmax,
            self.dr,
            self.dr2,
            self.window,
        )
        .map_err(|e| FFTError::WindowCalculationFailed {
            reason: e.to_string(),
        })?;
        let weight = self.rweight.unwrap();
        if weight > 0.0 {
            for (window, radius) in win.iter_mut().zip(full_r.iter()) {
                *window *= radius.powf(weight);
            }
        }
        // The multiplication covers DC, all positive bins and Nyquist, at
        // their actual R positions. Display rmax_out never truncates the filter.
        let chir_scaled = chir * win.as_slice();
        let out = xftr_fast_nalgebra(&chir_scaled, self.nfft.unwrap(), self.kstep.unwrap());
        self.q = Some(DVector::from_vec(super::inverse_fft::q_grid(
            self.qmax_out.unwrap(),
            self.kstep.unwrap(),
            out.len(),
        )));
        self.rwin = Some(win);
        self.chiq = Some(out);

        Ok(self)
    }

    /// Read the stored value without recalculating.
    ///
    /// Returned q grid in Å⁻¹, beginning at zero; `None` before calculation.
    pub fn get_q(&self) -> Option<&DVector<f64>> {
        self.q.as_ref()
    }

    /// Return a newly allocated real inverse signal limited to the returned q grid.
    /// Forward k-weighting and windowing remain; returns `None` before calculation.
    pub fn get_chiq(&self) -> Option<DVector<f64>> {
        let len_q = self.q.as_ref()?.len();
        let chiq = self.chiq.as_ref()?;
        Some(DVector::from_iterator(
            len_q.min(chiq.len()),
            chiq.iter().take(len_q).copied(),
        ))
    }

    /// Read the stored value without recalculating.
    ///
    /// Full R-window multiplied by `R.powf(rweight)`, paired with all positive
    /// forward bins, not just the displayed R range. It is dimensionless only when
    /// `rweight=0`; `None` before calculation.
    pub fn get_rwin(&self) -> Option<&DVector<f64>> {
        self.rwin.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Output q spacing in Å⁻¹. `None` infers `pi / (R_step * nfft)`.
    /// An explicit positive value must agree with the supplied uniform R grid.
    pub fn get_kstep(&self) -> Option<&f64> {
        self.kstep.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Additional power of R applied before inversion; default 0.0.
    /// Nonnegative fractional values are floored. This does not remove forward k-weighting.
    pub fn get_rweight(&self) -> Option<&f64> {
        self.rweight.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// Inverse FFT length; default 2048. Changing it resizes the positive-frequency
    /// coefficients and changes automatic q spacing; leave `kstep` automatic.
    pub fn get_nfft(&self) -> Option<&usize> {
        self.nfft.as_ref()
    }

    /// Read the stored value without recalculating.
    ///
    /// R-window family; default Kaiser–Bessel. `None` selects Hanning, not all-pass.
    pub fn get_window(&self) -> Option<&FTWindow> {
        self.window.as_ref()
    }
}

/// Transform an already weighted/windowed real signal with `kstep / sqrt(pi)`.
///
/// Uses an unnormalized negative-exponent FFT and returns all positive-frequency
/// bins. The input is zero-padded or truncated to `nfft`. This low-level helper
/// does not validate sizes or spacing; prefer `XrayFFTF::xftf` for checked input.
pub fn xftf_fast_nalgebra(chi: &DVector<f64>, nfft: usize, kstep: f64) -> DynRealDft<f64> {
    let mut cchi = vec![0.0_f64; nfft];
    cchi[..chi.len().min(nfft)].copy_from_slice(&chi.as_slice()[..chi.len().min(nfft)]);

    let mut freq = cchi.real_fft();
    freq *= kstep / std::f64::consts::PI.sqrt();
    freq
}

/// Invert already filtered positive-frequency coefficients into a real signal.
///
/// Restores conjugate symmetry, resizes to `nfft`, and multiplies the unnormalized
/// inverse by `sqrt(pi)/(kstep*nfft)`. This helper does not validate settings; use
/// `XrayFFTR::xftr` for checked input.
pub fn xftr_fast_nalgebra(chir: &DynRealDft<f64>, nfft: usize, kstep: f64) -> DVector<f64> {
    DVector::from_vec(super::inverse_fft::inverse(chir, nfft, kstep))
}

/// Convenience extension for the unnormalized forward FFT with XAFS scaling.
pub trait XFFT {
    /// Transform already weighted/windowed input; see the corresponding fast helper.
    fn xftf_fast(&self, nfft: usize, kstep: f64) -> DynRealDft<f64>;
}

impl XFFT for DVector<f64> {
    fn xftf_fast(&self, nfft: usize, kstep: f64) -> DynRealDft<f64> {
        xftf_fast_nalgebra(self, nfft, kstep)
    }
}

/// Convenience extension for a real inverse transform with XAFS scaling.
pub trait XFFTReverse<T> {
    /// Invert already filtered positive-frequency coefficients into a real array.
    fn xftr_fast(&self, nfft: usize, kstep: f64) -> T;
}

impl XFFTReverse<DVector<f64>> for DynRealDft<f64> {
    fn xftr_fast(&self, nfft: usize, kstep: f64) -> DVector<f64> {
        xftr_fast_nalgebra(self, nfft, kstep)
    }
}

/// Allocate component arrays from complex Fourier coefficients.
pub trait FFTUtils<T> {
    /// Allocate interleaved real and imaginary values `[re0, im0, re1, im1, ...]`.
    /// Allocate the real component of every coefficient.
    fn realimg(&self) -> T;
    /// Allocate the real component of every coefficient.
    fn re(&self) -> T;
    /// Allocate the imaginary component of every coefficient.
    fn im(&self) -> T;
    /// Allocate each coefficient magnitude `sqrt(re*re + im*im)`.
    fn norm(&self) -> T;
    /// Allocate each coefficient magnitude `sqrt(re*re + im*im)`.
    /// Allocate each squared magnitude `re*re + im*im`.
    fn norm_sqr(&self) -> T;
}

impl FFTUtils<DVector<f64>> for DynRealDft<f64> {
    fn realimg(&self) -> DVector<f64> {
        DVector::from_iterator(self.len() * 2, self.iter().flat_map(|x| [x.re, x.im]))
    }

    fn re(&self) -> DVector<f64> {
        DVector::from_iterator(self.len(), self.iter().map(|x| x.re))
    }

    fn im(&self) -> DVector<f64> {
        DVector::from_iterator(self.len(), self.iter().map(|x| x.im))
    }

    fn norm(&self) -> DVector<f64> {
        DVector::from_iterator(self.len(), self.iter().map(|x| x.norm()))
    }

    fn norm_sqr(&self) -> DVector<f64> {
        DVector::from_iterator(self.len(), self.iter().map(|x| x.norm_sqr()))
    }
}

impl FFTUtils<DVector<f64>> for [Complex<f64>] {
    fn realimg(&self) -> DVector<f64> {
        DVector::from_iterator(self.len() * 2, self.iter().flat_map(|x| [x.re, x.im]))
    }

    fn re(&self) -> DVector<f64> {
        DVector::from_iterator(self.len(), self.iter().map(|x| x.re))
    }

    fn im(&self) -> DVector<f64> {
        DVector::from_iterator(self.len(), self.iter().map(|x| x.im))
    }

    fn norm(&self) -> DVector<f64> {
        DVector::from_iterator(self.len(), self.iter().map(|x| x.norm()))
    }

    fn norm_sqr(&self) -> DVector<f64> {
        DVector::from_iterator(self.len(), self.iter().map(|x| x.norm_sqr()))
    }
}

fn linspace(start: f64, end: f64, n: usize) -> DVector<f64> {
    if n <= 1 {
        return DVector::from_vec(vec![start]);
    }

    let step = (end - start) / (n as f64 - 1.0);
    DVector::from_iterator(n, (0..n).map(|i| start + step * i as f64))
}
