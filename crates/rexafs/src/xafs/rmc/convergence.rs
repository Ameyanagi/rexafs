//! Operational residual-trend diagnostics; not a structural convergence proof.
use super::*;

/// Empirical numerical plateau criterion (unreleased). Defaults compare three
/// consecutive 500-attempt windows after at least 3,000 attempts. Each window's
/// best-score improvement must be ≤ absolute_tolerance + 0.005*|previous best|,
/// and the mean score change must be ≤ absolute_tolerance + 0.01*|previous mean|.
/// These are transparent project defaults, not statistically calibrated tests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ResidualTrendSettings {
    /// Attempts per nonoverlapping window; default 500, range 2..=100,000.
    pub window: usize,
    /// Consecutive flat windows required; default 3, range 1..=20. One additional
    /// window supplies the initial comparison mean.
    pub stable_windows: usize,
    /// Earliest total attempt count eligible for a plateau; default 3,000.
    pub minimum_attempts: usize,
    /// Nonnegative absolute tolerance in objective units; default 1e-5.
    pub absolute_tolerance: f64,
    /// Nonnegative fractional best-score improvement tolerance; default 0.005.
    pub relative_best_tolerance: f64,
    /// Nonnegative fractional mean-score change tolerance; default 0.01.
    pub relative_mean_tolerance: f64,
}
impl Default for ResidualTrendSettings {
    fn default() -> Self {
        Self {
            window: 500,
            stable_windows: 3,
            minimum_attempts: 3000,
            absolute_tolerance: 1e-5,
            relative_best_tolerance: 0.005,
            relative_mean_tolerance: 0.01,
        }
    }
}
/// Numerical trend status. A plateau can be a poor local fit or constraint-limited
/// state; none of these variants establishes a unique physical structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResidualTrendStatus {
    /// Too few total attempts or retained consecutive records for the declared rule.
    InsufficientHistory,
    /// At least one complete recent window exceeds a declared change tolerance.
    StillChanging,
    /// All required recent windows satisfy both numerical change tolerances.
    ResidualPlateau,
}
/// One nonoverlapping window, including rejected attempts (unreleased).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResidualWindow {
    /// First attempted move in this window, inclusive.
    pub first_step: usize,
    /// Last attempted move in this window, inclusive.
    pub last_step: usize,
    /// Mean current-state objective after attempted moves, including rejections.
    pub mean_score: f64,
    /// Best encountered objective at the end of this window.
    pub best_score: f64,
    /// Previous boundary's best objective; None if no preceding retained record.
    pub previous_best: Option<f64>,
    /// Previous nonoverlapping window's mean, when available.
    pub previous_mean: Option<f64>,
    /// Previous best minus this best; nonnegative and in objective units.
    pub best_improvement: Option<f64>,
    /// This window's mean minus the previous mean, in objective units.
    pub mean_change: Option<f64>,
    /// Best improvement divided by |previous best|; None if unavailable or zero scale.
    pub relative_best_improvement: Option<f64>,
    /// Mean change divided by |previous mean|; None if unavailable or zero scale.
    pub relative_mean_change: Option<f64>,
    /// Whether both absolute-plus-relative tolerances pass; None without a baseline.
    pub flat: Option<bool>,
}
/// Owned diagnostic trace. It does not mutate a session, consume random numbers,
/// change its objective or stop it automatically. Record the settings with results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResidualTrendReport {
    /// Explicit criterion used for this assessment.
    pub settings: ResidualTrendSettings,
    /// Last attempted move in the supplied history, or zero when empty.
    pub completed: usize,
    /// Insufficient history, still changing, or a numerical residual plateau.
    pub status: ResidualTrendStatus,
    /// Up to stable_windows+1 recent complete windows in chronological order.
    pub windows: Vec<ResidualWindow>,
}
/// Assess recent changes without scattering or fitting (unreleased). History
/// must have finite scores, consecutive increasing steps and nonincreasing best
/// scores no larger than current scores. Retaining fewer records than required
/// reports InsufficientHistory rather than declaring convergence. Windows end at
/// the latest record; older incomplete data are omitted. A flat best score alone
/// is insufficient when the current-score mean is still changing.
pub fn residual_trend(
    history: &[SessionStep],
    settings: &ResidualTrendSettings,
) -> Result<ResidualTrendReport, RmcError> {
    assess(
        &history
            .iter()
            .map(|r| TrendSample {
                step: r.step,
                score: r.score,
                best_score: r.best_score,
            })
            .collect::<Vec<_>>(),
        settings,
    )
}
struct TrendSample {
    step: usize,
    score: f64,
    best_score: f64,
}
/// Apply the same best-plus-mean criterion to EA generation records (unreleased).
/// Here window/minimum_attempts and reported step indices count generations;
/// use explicit settings (for example window=10, minimum_attempts=60), not the
/// 500-attempt RMC default. Uses population means, including individuals that
/// survive unchanged. Only the consecutive suffix with recorded means is usable;
/// old checkpoints without means cannot establish a plateau.
pub fn evolution_residual_trend(
    history: &[EvolutionGeneration],
    settings: &ResidualTrendSettings,
) -> Result<ResidualTrendReport, RmcError> {
    let start = history
        .iter()
        .rposition(|r| r.mean_score.is_none())
        .map_or(0, |i| i + 1);
    let samples: Vec<_> = history[start..]
        .iter()
        .map(|r| TrendSample {
            step: r.generation,
            score: r.mean_score.unwrap(),
            best_score: r.best_score,
        })
        .collect();
    let mut report = assess(&samples, settings)?;
    report.completed = history.last().map_or(0, |r| r.generation);
    Ok(report)
}
fn assess(
    history: &[TrendSample],
    settings: &ResidualTrendSettings,
) -> Result<ResidualTrendReport, RmcError> {
    require(
        (2..=100_000).contains(&settings.window)
            && (1..=20).contains(&settings.stable_windows)
            && [
                settings.absolute_tolerance,
                settings.relative_best_tolerance,
                settings.relative_mean_tolerance,
            ]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.),
        "invalid residual trend settings",
    )?;
    require(
        history
            .iter()
            .all(|h| h.score.is_finite() && h.best_score.is_finite() && h.best_score <= h.score)
            && history.windows(2).all(|h| {
                h[1].step.checked_sub(h[0].step) == Some(1) && h[1].best_score <= h[0].best_score
            }),
        "residual trend needs finite consecutive history with valid best scores",
    )?;
    let count = (history.len() / settings.window).min(settings.stable_windows + 1);
    let start = history.len() - count * settings.window;
    let mut windows: Vec<ResidualWindow> = Vec::new();
    for index in 0..count {
        let from = start + index * settings.window;
        let to = from + settings.window;
        let samples = &history[from..to];
        let mean = samples
            .iter()
            .map(|s| s.score / settings.window as f64)
            .sum::<f64>();
        let best = samples.last().unwrap().best_score;
        let previous_best = from.checked_sub(1).map(|i| history[i].best_score);
        let previous_mean = windows.last().map(|w| w.mean_score);
        let improvement = previous_best.map(|v| v - best);
        let change = previous_mean.map(|v| mean - v);
        require(
            mean.is_finite()
                && improvement.is_none_or(f64::is_finite)
                && change.is_none_or(f64::is_finite),
            "residual trend overflow",
        )?;
        windows.push(ResidualWindow {
            first_step: samples[0].step,
            last_step: samples.last().unwrap().step,
            mean_score: mean,
            best_score: best,
            previous_best,
            previous_mean,
            best_improvement: improvement,
            mean_change: change,
            relative_best_improvement: previous_best
                .filter(|v| *v != 0.)
                .map(|v| (v - best) / v.abs()),
            relative_mean_change: previous_mean
                .filter(|v| *v != 0.)
                .map(|v| (mean - v) / v.abs()),
            flat: previous_best.zip(previous_mean).map(|(b, m)| {
                b - best <= settings.absolute_tolerance + settings.relative_best_tolerance * b.abs()
                    && (mean - m).abs()
                        <= settings.absolute_tolerance + settings.relative_mean_tolerance * m.abs()
            }),
        });
    }
    let completed = history.last().map_or(0, |h| h.step);
    let status = if completed < settings.minimum_attempts || count < settings.stable_windows + 1 {
        ResidualTrendStatus::InsufficientHistory
    } else if windows.iter().skip(1).all(|w| w.flat == Some(true)) {
        ResidualTrendStatus::ResidualPlateau
    } else {
        ResidualTrendStatus::StillChanging
    };
    Ok(ResidualTrendReport {
        settings: settings.clone(),
        completed,
        status,
        windows,
    })
}
