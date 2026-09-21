use super::geometry::{distance, distances_allowed};
use super::*;
use crate::xafs::xafsutils::constants::ETOK;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;
use std::ops::ControlFlow;

pub(super) fn validate_problem(problem: &RmcProblem) -> Result<(), RmcError> {
    problem.configuration.validate()?;
    require(
        !problem.datasets.is_empty(),
        "at least one dataset is required",
    )?;
    let mut names = HashSet::new();
    for d in &problem.datasets {
        require(
            !d.name.trim().is_empty() && names.insert(&d.name),
            "dataset names must be nonempty and unique",
        )?;
        let n = d.k.len();
        require(
            n >= 2 && d.chi.len() == n && d.sigma.len() == n,
            format!("{}: k, chi and sigma need equal lengths >=2", d.name),
        )?;
        require(
            d.k.iter().all(|k| k.is_finite() && *k >= 0.0) && d.k.windows(2).all(|w| w[1] > w[0]),
            "k must be finite, nonnegative and strictly increasing",
        )?;
        require(
            d.chi.iter().all(|x| x.is_finite())
                && d.sigma.iter().all(|s| s.is_finite() && *s > 0.0),
            "chi must be finite and sigma must be finite and positive",
        )?;
        require(
            d.weight.is_finite()
                && d.weight > 0.0
                && d.s02.is_finite()
                && d.s02 > 0.0
                && d.delta_e0.is_finite()
                && d.kweight <= 3,
            "invalid dataset weight, S02, energy shift or k weight",
        )?;
        let mut indices = HashSet::new();
        require(
            !d.absorbers.is_empty()
                && d.absorbers
                    .iter()
                    .all(|&a| a < problem.configuration.atoms.len() && indices.insert(a)),
            "absorber indices must be nonempty, distinct and in range",
        )?;
        let z = problem.configuration.atoms[d.absorbers[0]].atomic_number;
        require(
            d.absorbers
                .iter()
                .all(|&a| problem.configuration.atoms[a].atomic_number == z),
            "one dataset cannot average absorbers of different elements",
        )?;
        shifted_grid(d)?;
    }
    Ok(())
}

pub(super) fn shifted_grid(dataset: &ExafsDataset) -> Result<Vec<f64>, RmcError> {
    let grid: Vec<_> = dataset
        .k
        .iter()
        .map(|k| (k * k - ETOK * dataset.delta_e0).sqrt())
        .collect();
    require(
        grid.iter().all(|q| q.is_finite()) && grid.windows(2).all(|w| w[1] > w[0]),
        "energy shift produces invalid or unresolved theoretical k values",
    )?;
    Ok(grid)
}

/// Calculate all datasets for the problem's geometry without making moves.
/// Validates inputs, averages absorber spectra before scoring, and returns owned
/// arrays. Hard pair/displacement constraints are specific to `refine` and are
/// not applied here. Backend errors and nonfinite calculations propagate.
pub fn evaluate<C: ExafsCalculator + ?Sized>(
    problem: &RmcProblem,
    calculator: &mut C,
) -> Result<Evaluation, RmcError> {
    validate_problem(problem)?;
    evaluate_configuration(&problem.configuration, &problem.datasets, calculator)
}

fn evaluate_configuration<C: ExafsCalculator + ?Sized>(
    configuration: &Configuration,
    datasets: &[ExafsDataset],
    calculator: &mut C,
) -> Result<Evaluation, RmcError> {
    let mut fits = Vec::with_capacity(datasets.len());
    let mut total = 0.0;
    for d in datasets {
        let q = shifted_grid(d)?;
        let mut chi = vec![0.0; q.len()];
        for &absorber in &d.absorbers {
            let calculated = calculator.calculate(configuration, absorber, d.edge, &q)?;
            require(
                calculated.len() == q.len() && calculated.iter().all(|x| x.is_finite()),
                format!(
                    "{} returned an invalid spectrum for {}",
                    calculator.name(),
                    d.name
                ),
            )?;
            for (sum, value) in chi.iter_mut().zip(calculated) {
                *sum += value / d.absorbers.len() as f64;
            }
        }
        for value in &mut chi {
            *value *= d.s02;
        }
        let score = d.weight
            * chi
                .iter()
                .zip(&d.chi)
                .zip(&d.sigma)
                .zip(&d.k)
                .map(|(((model, observed), sigma), k)| {
                    ((model - observed) / sigma * k.powi(i32::from(d.kweight))).powi(2)
                })
                .sum::<f64>()
            / chi.len() as f64;
        require(
            score.is_finite(),
            "objective overflow; check data, weights and noise scales",
        )?;
        total += score;
        fits.push(DatasetFit {
            name: d.name.clone(),
            chi,
            score,
        });
    }
    require(total.is_finite(), "total objective overflow")?;
    Ok(Evaluation {
        score: total,
        datasets: fits,
    })
}

/// Refine coordinates using the validated problem and settings. The caller's
/// problem is unchanged. Zero steps evaluates only the starting geometry.
/// All hard constraints must hold initially. Calculation failures abort with an
/// error; use `refine_with_progress` to observe completed moves or stop early.
pub fn refine<C: ExafsCalculator + ?Sized>(
    problem: &RmcProblem,
    settings: &RmcSettings,
    calculator: &mut C,
) -> Result<RmcResult, RmcError> {
    refine_with_progress(problem, settings, calculator, |_| ControlFlow::Continue(()))
}

/// Refine with a callback after each attempted move. Return `ControlFlow::Break`
/// to finish successfully with `stopped=true` and the last accepted state. The
/// callback runs synchronously, outside scattering calculations; it cannot
/// interrupt an in-flight calculation. ReFEFF offers a separate cancellation token
/// and per-calculation timeout. Callback panics are not caught.
pub fn refine_with_progress<C, F>(
    problem: &RmcProblem,
    settings: &RmcSettings,
    calculator: &mut C,
    mut progress: F,
) -> Result<RmcResult, RmcError>
where
    C: ExafsCalculator + ?Sized,
    F: FnMut(&RmcStep) -> ControlFlow<()>,
{
    validate_problem(problem)?;
    require(
        settings.step_size.is_finite() && settings.step_size > 0.0 && settings.step_size <= 1e6,
        "step_size must be positive and at most 1e6 Å",
    )?;
    require(
        settings.temperature.is_finite() && settings.temperature >= 0.0,
        "temperature must be finite and nonnegative",
    )?;
    require(
        settings.min_distance.is_finite() && settings.min_distance > 0.0,
        "minimum distance must be finite and positive",
    )?;
    if let Some(limit) = settings.max_displacement {
        require(
            limit.is_finite() && limit > 0.0,
            "maximum displacement must be finite and positive",
        )?;
    }
    let mut indices = HashSet::new();
    require(
        settings
            .movable_atoms
            .iter()
            .all(|&a| a < problem.configuration.atoms.len() && indices.insert(a)),
        "movable atom indices must be distinct and in range",
    )?;
    let movable = if settings.movable_atoms.is_empty() {
        (0..problem.configuration.atoms.len()).collect()
    } else {
        settings.movable_atoms.clone()
    };
    let lattice = problem.configuration.lattice()?;
    require(
        distances_allowed(
            &problem.configuration,
            lattice.as_ref(),
            settings.min_distance,
            None,
        )?,
        "initial configuration violates the minimum distance",
    )?;
    let evaluation = evaluate_configuration(&problem.configuration, &problem.datasets, calculator)?;
    let initial = RmcState {
        configuration: problem.configuration.clone(),
        evaluation,
    };
    let mut current = initial.clone();
    let mut best = initial.clone();
    let mut rng = ChaCha8Rng::seed_from_u64(settings.seed);
    let mut history = Vec::new();
    let mut stopped = false;
    for step in 1..=settings.steps {
        let atom = movable[rng.random_range(0..movable.len())];
        // Trial owns its coordinates: rejected geometry and spectra cannot leak.
        let mut trial = current.configuration.clone();
        for value in &mut trial.atoms[atom].position {
            *value += settings.step_size * (2.0 * rng.random::<f64>() - 1.0);
        }
        let position = trial.atoms[atom].position;
        let allowed = position.iter().all(|v| v.is_finite() && v.abs() <= 1e8)
            && settings.max_displacement.is_none_or(|limit| {
                distance(position, initial.configuration.atoms[atom].position) <= limit
            })
            && distances_allowed(&trial, lattice.as_ref(), settings.min_distance, Some(atom))?;
        let mut record = RmcStep {
            step,
            atom,
            accepted: false,
            constraint_rejected: !allowed,
            trial_score: None,
            score: current.evaluation.score,
            best_score: best.evaluation.score,
        };
        if allowed {
            let evaluation = evaluate_configuration(&trial, &problem.datasets, calculator)?;
            let delta = evaluation.score - current.evaluation.score;
            record.trial_score = Some(evaluation.score);
            // Log form avoids explicitly constructing a tiny acceptance
            // probability. The uniform variate lies in [0,1).
            let accepted = delta <= 0.0
                || (settings.temperature > 0.0
                    && rng.random::<f64>().ln() < -0.5 * (delta / settings.temperature));
            if accepted {
                current = RmcState {
                    configuration: trial,
                    evaluation,
                };
                if current.evaluation.score < best.evaluation.score {
                    best = current.clone();
                }
                record.accepted = true;
            }
        }
        record.score = current.evaluation.score;
        record.best_score = best.evaluation.score;
        stopped = progress(&record).is_break();
        history.push(record);
        if stopped {
            break;
        }
    }
    Ok(RmcResult {
        calculator: calculator.name().to_string(),
        settings: settings.clone(),
        initial,
        best,
        final_state: current,
        history,
        stopped,
    })
}
