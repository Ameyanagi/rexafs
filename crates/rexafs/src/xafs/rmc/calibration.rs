use super::*;

/// Independent amplitude/energy calibration limits (since 0.2.10). Geometry and
/// mixture fractions remain fixed. Calibration on a known reference is preferable
/// to treating fitted S₀²/ΔE₀ as independent structural evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalibrationSettings {
    /// Explicit increasing energy-shift grid in eV; 1..=10,000 candidates.
    /// Positive shifts sample smaller theoretical k. No candidate is silently dropped.
    pub delta_e0: Vec<f64>,
    /// Inclusive positive bounds for S₀². At each energy, its bounded least-squares
    /// optimum is computed analytically using the chosen linear-transform objective.
    pub s02_bounds: [f64; 2],
}
/// One energy-grid calibration candidate; the amplitude is optimized at that energy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalibrationPoint {
    /// Energy shift in eV, under the RMC sign convention.
    pub delta_e0: f64,
    /// Bounded fitted amplitude reduction factor S₀².
    pub s02: f64,
    /// Spectral objective including the dataset's fixed weight; no structural penalties.
    pub score: f64,
}
/// Serializable calibration trace with a fixed geometry and calculator identity.
/// This records a grid search and numerical fit, not parameter confidence intervals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalibrationResult {
    /// Stable scientific calculator identity.
    pub calculator: String,
    /// Calibrated dataset's name.
    pub dataset: String,
    /// Ordered energy candidates and their bounded amplitude optima.
    pub candidates: Vec<CalibrationPoint>,
    /// Lowest-score candidate; ties retain the earliest energy-grid point.
    pub best: CalibrationPoint,
    /// Model χ on the experimental k grid at the best candidate.
    pub chi: Vec<f64>,
    /// Normalized fit diagnostics at that candidate.
    pub report: FitReport,
}
/// Numerical agreement measures without a degrees-of-freedom or independence claim.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FitReport {
    /// Dataset label.
    pub name: String,
    /// Number of measured k samples, not a count of independent observations.
    pub points: usize,
    /// Chosen objective score, including the fixed dataset weight.
    pub objective_score: f64,
    /// Root mean square unweighted χ residual, dimensionless.
    pub chi_rmse: f64,
    /// `Σ[k^w(model−experiment)/sigma]² / Σ[k^w experiment/sigma]²`.
    /// None when the denominator vanishes. Always measured in k space, even
    /// when the optimization objective uses R, q, wavelets, or STFT.
    pub normalized_k_residual: Option<f64>,
    /// Whether the fitted amplitude is at a declared calibration bound, when known.
    pub amplitude_at_bound: Option<bool>,
}
/// Report objective and normalized k-space residual. Applies noise/k weights once;
/// a small ratio does not establish structural uniqueness or calibrated uncertainty.
pub fn fit_report(
    dataset: &ExafsDataset,
    model: &[f64],
    objective: &Objective,
) -> Result<FitReport, RmcError> {
    let score = objective.score(dataset, model)?;
    let mut numerator = 0.;
    let mut denominator = 0.;
    let mut squares = 0.;
    for (((&q, &observed), &sigma), &calculated) in dataset
        .k
        .iter()
        .zip(&dataset.chi)
        .zip(&dataset.sigma)
        .zip(model)
    {
        let difference = calculated - observed;
        squares += difference * difference;
        let weight = q.powi(i32::from(dataset.kweight)) / sigma;
        numerator += (weight * difference).powi(2);
        denominator += (weight * observed).powi(2);
    }
    require(
        squares.is_finite() && numerator.is_finite() && denominator.is_finite(),
        "fit report overflow",
    )?;
    Ok(FitReport {
        name: dataset.name.clone(),
        points: model.len(),
        objective_score: score,
        chi_rmse: (squares / model.len() as f64).sqrt(),
        normalized_k_residual: if denominator > 0. {
            Some(numerator / denominator)
        } else {
            None
        },
        amplitude_at_bound: None,
    })
}

/// Calibrate one dataset using a copy of fixed structures and normalized mixture
/// fractions. Calculates all requested shifted k values in one union-grid batch,
/// so each absorber geometry is evaluated only once. The union is bounded to one
/// million points. Backend support errors abort; there is no extrapolation.
/// Returns owned results and leaves the problem unchanged. Apply `best.s02` and
/// `best.delta_e0` explicitly to a new RMC problem and retain this trace as provenance.
pub fn calibrate_dataset<C: ExafsCalculator + ?Sized>(
    problem: &EnsembleProblem,
    dataset: usize,
    settings: &CalibrationSettings,
    calculator: &mut C,
) -> Result<CalibrationResult, RmcError> {
    require(
        dataset < problem.datasets.len(),
        "calibration dataset out of range",
    )?;
    require(
        !settings.delta_e0.is_empty()
            && settings.delta_e0.len() <= 10_000
            && settings.delta_e0.iter().all(|v| v.is_finite())
            && settings.delta_e0.windows(2).all(|v| v[1] > v[0])
            && settings.s02_bounds.iter().all(|v| v.is_finite() && *v > 0.)
            && settings.s02_bounds[1] >= settings.s02_bounds[0],
        "invalid calibration grid or amplitude bounds",
    )?;
    let mut fixed = problem.clone();
    super::session::normalize(&mut fixed.structures)?;
    fixed.datasets = vec![problem.datasets[dataset].clone()];
    let session = SessionSettings {
        moves: RmcSettings {
            steps: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    super::session::prepare(&fixed, &session)?;
    let data = &fixed.datasets[0];
    let objective = data.objective.prepare(&data.exafs.k)?;
    require(
        settings.delta_e0.len().saturating_mul(data.exafs.k.len()) <= 1_000_000,
        "calibration union grid exceeds one million points",
    )?;
    let mut shifted = Vec::new();
    let mut grid = Vec::new();
    for &energy in &settings.delta_e0 {
        let mut candidate = data.exafs.clone();
        candidate.delta_e0 = energy;
        let q = super::engine::shifted_grid(&candidate)?;
        grid.extend(&q);
        shifted.push(q);
    }
    grid.sort_by(f64::total_cmp);
    grid.dedup();
    let mut requests = Vec::new();
    let mut weights = Vec::new();
    for (s, structure) in fixed.structures.iter().enumerate() {
        let absorbers = if data.absorbers_by_structure.is_empty() {
            &data.exafs.absorbers
        } else {
            &data.absorbers_by_structure[s]
        };
        for &absorber in absorbers {
            requests.push(CalculationRequest {
                structure: s,
                configuration: &structure.configuration,
                absorber,
                edge: data.exafs.edge,
                k: &grid,
                options: data.refeff.as_ref(),
                paths: false,
            });
            weights.push(structure.weight / absorbers.len() as f64);
        }
    }
    let spectra = calculator.calculate_batch(&requests)?;
    require(
        spectra.len() == requests.len(),
        "calibration backend returned wrong batch size",
    )?;
    let mut theory = vec![0.; grid.len()];
    for (spectrum, weight) in spectra.into_iter().zip(weights) {
        require(
            spectrum.chi.len() == grid.len() && spectrum.chi.iter().all(|v| v.is_finite()),
            "calibration backend returned invalid spectrum",
        )?;
        for (v, x) in theory.iter_mut().zip(spectrum.chi) {
            *v += weight * x;
        }
    }
    let zero = vec![0.; data.exafs.k.len()];
    let f0 = objective.score(&data.exafs, &zero)?;
    let mut zero_data = data.exafs.clone();
    zero_data.chi = zero;
    let mut candidates = Vec::new();
    let mut best_model = Vec::new();
    let mut best_score = f64::INFINITY;
    let mut best_index = 0;
    for (index, (&energy, q)) in settings.delta_e0.iter().zip(shifted).enumerate() {
        let raw: Vec<_> = q
            .iter()
            .map(|v| {
                theory[grid
                    .binary_search_by(|q| q.total_cmp(v))
                    .expect("union contains each requested k")]
            })
            .collect();
        let quadratic = objective.score(&zero_data, &raw)?;
        require(
            quadratic.is_finite() && quadratic > 0.,
            "calibration amplitude is unresolved: model has no objective-space norm",
        )?;
        // Balance model/observation norms before polarization of the quadratic
        // form; this avoids subtracting enormous nearly equal residual scores.
        let balance = if f0 > 0. {
            quadratic.sqrt() / f0.sqrt()
        } else {
            1.
        };
        require(
            balance.is_finite() && balance > 0.,
            "calibration normalization is numerically unresolved",
        )?;
        let mut balanced = data.exafs.clone();
        for v in &mut balanced.chi {
            *v *= balance;
        }
        let fp = objective.score(&balanced, &raw)?;
        let negative: Vec<_> = raw.iter().map(|v| -v).collect();
        let fm = objective.score(&balanced, &negative)?;
        let linear = (fp - fm) / 4.;
        let s02 =
            (-linear / quadratic / balance).clamp(settings.s02_bounds[0], settings.s02_bounds[1]);
        let model: Vec<_> = raw.iter().map(|v| s02 * v).collect();
        let score = objective.score(&data.exafs, &model)?;
        candidates.push(CalibrationPoint {
            delta_e0: energy,
            s02,
            score,
        });
        if score < best_score {
            best_score = score;
            best_index = index;
            best_model = model;
        }
    }
    let best = candidates[best_index].clone();
    let mut report = fit_report(&data.exafs, &best_model, &data.objective)?;
    report.amplitude_at_bound =
        Some(best.s02 == settings.s02_bounds[0] || best.s02 == settings.s02_bounds[1]);
    Ok(CalibrationResult {
        calculator: calculator.identity(),
        dataset: data.exafs.name.clone(),
        candidates,
        best,
        chi: best_model,
        report,
    })
}
