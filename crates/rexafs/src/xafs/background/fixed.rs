//! Fixed mean-square endpoint penalty with reusable column-scaled SVD factors.
use super::*;

#[derive(Clone, PartialEq)]
struct Geometry {
    knots: DVector<f64>,
    kraw: DVector<f64>,
    kout: DVector<f64>,
    window: DVector<f64>,
    nfft: usize,
    cutoff: usize,
    endpoints: Vec<(usize, f64)>,
    lambda: f64,
}

struct Workspace {
    geometry: Geometry,
    basis: DMatrix<f64>,
    /// Column-scaled design, retained for per-spectrum stationarity checks.
    design: DMatrix<f64>,
    column_norms: DVector<f64>,
    svd: nalgebra::linalg::SVD<f64, Dyn, Dyn>,
    rank_tolerance: f64,
}

fn cache() -> &'static Mutex<VecDeque<Arc<Workspace>>> {
    static CACHE: OnceLock<Mutex<VecDeque<Arc<Workspace>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn failed(reason: impl Into<String>) -> BackgroundError {
    BackgroundError::DirectSolverFailed {
        reason: reason.into(),
    }
}

pub(super) fn validate(settings: &AUTOBK) -> Result<(), BackgroundError> {
    let lambda = settings.clamp_lambda.unwrap_or(DEFAULT_CLAMP_LAMBDA);
    if !lambda.is_finite() || lambda < 0.0 {
        return Err(failed("clamp_lambda must be finite and nonnegative"));
    }
    let limit = settings
        .linear_condition_limit
        .unwrap_or(DEFAULT_LINEAR_CONDITION_LIMIT);
    if !limit.is_finite() || limit < 1.0 {
        return Err(failed(
            "linear_condition_limit must be finite and at least one",
        ));
    }
    if settings.nclamp.unwrap_or(3) < 0 {
        return Err(failed("nclamp must be nonnegative"));
    }
    Ok(())
}

impl Geometry {
    fn head(&self, chi: &DVector<f64>) -> DVector<f64> {
        // Reference normalization is fixed at 0.05, matching Larch AUTOBK's
        // residual FFT. kstep still determines the physical R grid/cutoff.
        let fft = chi.component_mul(&self.window).xftf_fast(self.nfft, 0.05);
        fft[..self.cutoff.min(fft.len())].realimg()
    }

    fn residual(&self, chi: &DVector<f64>) -> DVector<f64> {
        let mut out = self.head(chi);
        if !self.endpoints.is_empty() {
            let scale = (self.lambda * out.len() as f64 / self.endpoints.len() as f64).sqrt();
            out.extend(
                self.endpoints
                    .iter()
                    .map(|&(i, weight)| scale * weight * chi[i]),
            );
        }
        out
    }

    fn workspace(self) -> Result<Workspace, BackgroundError> {
        let count = self.knots.len() - 4;
        let mut basis = spline::coefficient_jacobian(
            self.knots.as_slice(),
            count,
            3,
            self.kout.as_slice(),
            false,
        );
        // Interpolation of a sampled spline reproduces the same spline if all
        // its interior breakpoints are also interior knots of the raw-data
        // interpolant. Otherwise resample each column explicitly (still O(n)).
        let direct = self.knots.as_slice()[4..count].iter().all(|&k| {
            if k <= self.kraw[0] || k >= self.kraw[self.kraw.len() - 1] {
                return true;
            }
            self.kraw
                .as_slice()
                .binary_search_by(|v| v.total_cmp(&k))
                .is_ok_and(|i| i >= 2 && i + 2 < self.kraw.len())
        });
        if !direct {
            let raw_basis = spline::coefficient_jacobian(
                self.knots.as_slice(),
                count,
                3,
                self.kraw.as_slice(),
                false,
            );
            for (j, column) in raw_basis.column_iter().enumerate() {
                let values = spline::cubic_resample(
                    self.kraw.as_slice(),
                    column.as_slice(),
                    self.kout.as_slice(),
                )
                .map_err(failed)?;
                basis.set_column(j, &DVector::from_vec(values));
            }
        }
        let mut design = DMatrix::from_columns(
            &basis
                .column_iter()
                .map(|col| self.residual(&col.into_owned()))
                .collect::<Vec<_>>(),
        );
        if design.iter().any(|v| !v.is_finite()) {
            return Err(failed("non-finite fixed-penalty system"));
        }
        let mut column_norms = DVector::zeros(design.ncols());
        for (j, mut column) in design.column_iter_mut().enumerate() {
            column_norms[j] = column.norm();
            if !column_norms[j].is_finite() || column_norms[j] == 0.0 {
                return Err(failed("zero/non-finite design column"));
            }
            column /= column_norms[j];
        }
        let rows = design.nrows();
        let cols = design.ncols();
        if rows < cols {
            return Err(failed("underdetermined fixed-penalty system"));
        }
        // Only geometry determines the factorization. Spectrum data and an
        // optional chi standard enter the right-hand side in solve(), below.
        let svd = design.clone().svd(true, true);
        let rank_tolerance = f64::EPSILON * rows.max(cols) as f64 * svd.singular_values.max();
        Ok(Workspace {
            geometry: self,
            basis,
            design,
            column_norms,
            svd,
            rank_tolerance,
        })
    }
}

pub(super) fn solve(
    problem: &AUTOBKSpline,
    settings: &AUTOBK,
) -> Result<(DVector<f64>, DVector<f64>), BackgroundError> {
    let lambda = settings.clamp_lambda.unwrap_or(DEFAULT_CLAMP_LAMBDA);
    let n = (problem.nclamp.max(0) as usize).min(problem.kout.len());
    let mut endpoints = Vec::new();
    if lambda > 0.0 && n > 0 {
        for (start, weight) in [
            (0, problem.clamp_lo),
            (problem.kout.len() - n, problem.clamp_hi),
        ] {
            if weight != 0 {
                endpoints.extend((start..start + n).map(|i| (i, (weight as f64).abs())));
            }
        }
    }
    let geometry = Geometry {
        knots: problem.knots.clone(),
        kraw: problem.kraw.clone(),
        kout: problem.kout.clone(),
        window: problem.ftwin.clone(),
        nfft: problem.nfft,
        cutoff: problem.irbkg,
        endpoints,
        lambda,
    };
    let use_cache = settings.linear_workspace_cache.unwrap_or(true);
    let existing = if use_cache {
        cache().lock().ok().and_then(|mut entries| {
            let i = entries.iter().position(|w| w.geometry == geometry)?;
            let workspace = entries.remove(i)?;
            entries.push_front(workspace.clone());
            Some(workspace)
        })
    } else {
        None
    };
    let workspace = if let Some(workspace) = existing {
        workspace
    } else {
        let workspace = Arc::new(geometry.workspace()?);
        if use_cache {
            if let Ok(mut entries) = cache().lock() {
                entries.push_front(workspace.clone());
                entries.truncate(AUTOBK_WORKSPACE_CACHE_CAPACITY);
            }
        }
        workspace
    };
    let data = if let Some(standard) = &problem.chi_std {
        &problem.mu - standard
    } else {
        problem.mu.clone()
    };
    let rhs = workspace.geometry.residual(&data);
    let matrix = &workspace.design;
    if rhs.iter().any(|v| !v.is_finite()) {
        return Err(failed("non-finite fixed-penalty system"));
    }
    let svd = &workspace.svd;
    let largest = svd.singular_values.max();
    let smallest = svd.singular_values.min();
    let tolerance = workspace.rank_tolerance;
    // The acceptance limit is a per-call setting, not part of the geometry
    // key. A cached factorization must still honor a newly tightened limit.
    let limit = settings
        .linear_condition_limit
        .unwrap_or(DEFAULT_LINEAR_CONDITION_LIMIT);
    if !smallest.is_finite() || smallest <= tolerance || largest / smallest > limit {
        return Err(failed(format!(
            "rank-deficient or ill-conditioned fixed-penalty system (condition {})",
            largest / smallest
        )));
    }
    let scaled = svd.solve(&rhs, tolerance).map_err(failed)?;
    let residual = matrix * &scaled - &rhs;
    let stationarity = (matrix.transpose() * residual).norm();
    if scaled.iter().any(|v| !v.is_finite()) || stationarity > 1.0e-9 * rhs.norm().max(1.0) {
        return Err(failed(
            "fixed-penalty solution failed stationarity/finite validation",
        ));
    }
    let coefs = scaled.component_div(&workspace.column_norms);
    let chi = &problem.mu - &workspace.basis * &coefs;
    let bkg = DVector::from_vec(spline::evaluate(
        problem.knots.as_slice(),
        coefs.as_slice(),
        3,
        problem.kraw.as_slice(),
        false,
    ));
    Ok((bkg, chi))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use rayon::prelude::*;

    #[test]
    fn defaults_and_legacy_deserialization_are_distinct() {
        assert_eq!(
            AUTOBK::new().clamp_scale_policy,
            Some(AUTOBKClampScalePolicy::FixedPenalty)
        );
        assert_eq!(AUTOBK::new().clamp_lambda, Some(0.001));
        let legacy: AUTOBK = serde_json::from_str("{}").unwrap();
        assert_eq!(
            legacy.clamp_scale_policy,
            Some(AUTOBKClampScalePolicy::Fixed)
        );
        let restored: AUTOBK =
            serde_json::from_str(&serde_json::to_string(&AUTOBK::new()).unwrap()).unwrap();
        assert_eq!(restored, AUTOBK::new());
    }

    #[test]
    fn endpoint_penalty_is_mean_of_active_weighted_points_including_last() {
        let geometry = Geometry {
            knots: DVector::zeros(0),
            kraw: DVector::zeros(0),
            kout: DVector::zeros(8),
            window: DVector::from_element(8, 1.0),
            nfft: 16,
            cutoff: 3,
            endpoints: vec![(0, 2.0), (1, 2.0), (6, 5.0), (7, 5.0)],
            lambda: 0.001,
        };
        let chi = DVector::from_iterator(8, (1..=8).map(|i| i as f64));
        let residual = geometry.residual(&chi);
        let head = geometry.head(&chi);
        let expected =
            head.norm_squared() / 6.0 + 0.001 * (4.0 * (1.0 + 4.0) + 25.0 * (49.0 + 64.0)) / 4.0;
        assert_abs_diff_eq!(residual.norm_squared() / 6.0, expected, epsilon = 1e-12);
    }

    #[test]
    fn invalid_strengths_are_rejected() {
        for lambda in [-1.0, f64::NAN, f64::INFINITY] {
            let settings = AUTOBK {
                clamp_lambda: Some(lambda),
                ..AUTOBK::new()
            };
            assert!(validate(&settings).is_err());
        }
        for lambda in [0.0, 0.001, 1.0] {
            assert!(validate(&AUTOBK {
                clamp_lambda: Some(lambda),
                ..AUTOBK::new()
            })
            .is_ok());
        }
    }

    #[derive(Deserialize)]
    struct Reference {
        name: String,
        energy: Vec<f64>,
        mu: Vec<f64>,
        k: Vec<f64>,
        edge_step: f64,
        settings: AUTOBK,
        chi: Vec<Vec<f64>>,
    }

    fn references() -> Vec<Reference> {
        serde_json::from_str(include_str!(
            "../../../tests/testfiles/autobk_fixed_reference.json"
        ))
        .unwrap()
    }

    fn calculate(
        case: &Reference,
        mut settings: AUTOBK,
        mu: &[f64],
    ) -> Result<AUTOBK, BackgroundError> {
        let mut norm = Some(normalization::NormalizationMethod::PrePostEdge(
            normalization::PrePostEdge {
                e0: settings.ek0,
                edge_step: Some(case.edge_step),
                ..Default::default()
            },
        ));
        settings.calc_background(
            &DVector::from_column_slice(&case.energy),
            &DVector::from_column_slice(mu),
            &mut norm,
        )?;
        Ok(settings)
    }

    #[test]
    fn cached_factorization_honors_changed_condition_limit() {
        let cases = references();
        let case = &cases[0];
        let mut settings = case.settings.clone();
        settings.linear_workspace_cache = Some(true);
        let first = calculate(case, settings.clone(), &case.mu).unwrap();

        // Geometry is unchanged; accepting a prior solve must not bypass a
        // stricter per-call limit, even when a legacy fallback is requested.
        let mut strict = settings.clone();
        strict.linear_condition_limit = Some(1.0);
        strict.linear_fallback_to_lm = Some(true);
        assert!(matches!(
            calculate(case, strict, &case.mu),
            Err(BackgroundError::DirectSolverFailed { reason })
                if reason.contains("ill-conditioned")
        ));

        let restored = calculate(case, settings, &case.mu).unwrap();
        assert_eq!(restored.chi, first.chi);
        assert_eq!(restored.bkg, first.bkg);
    }

    #[test]
    fn cached_factorization_solves_new_data_and_chi_standards() {
        let cases = references();
        let case = &cases[0];
        let changed_mu: Vec<_> = case
            .mu
            .iter()
            .enumerate()
            .map(|(i, mu)| mu + 0.02 * case.edge_step * (i as f64 * 0.07).sin())
            .collect();
        for lambda in [0.0, 0.001, 1.0] {
            let mut settings = case.settings.clone();
            settings.clamp_lambda = Some(lambda);
            settings.linear_workspace_cache = Some(true);
            let first = calculate(case, settings.clone(), &case.mu).unwrap();
            for standard_gain in [0.0, 0.2, -0.2] {
                if standard_gain != 0.0 {
                    settings.k_std = Some(DVector::from_column_slice(&case.k));
                    settings.chi_std =
                        Some(DVector::from_column_slice(&case.chi[1]) * standard_gain);
                }
                let cached = calculate(case, settings.clone(), &changed_mu).unwrap();
                let mut uncached_settings = settings.clone();
                uncached_settings.linear_workspace_cache = Some(false);
                let uncached = calculate(case, uncached_settings, &changed_mu).unwrap();
                assert_eq!(cached.chi, uncached.chi);
                assert_eq!(cached.bkg, uncached.bkg);
                assert_ne!(cached.chi, first.chi, "the new spectrum needs a new solve");
            }
            // Solving different data and standards must not mutate the factors.
            settings.k_std = None;
            settings.chi_std = None;
            let restored = calculate(case, settings, &case.mu).unwrap();
            assert_eq!(restored.chi, first.chi);
            assert_eq!(restored.bkg, first.bkg);
        }
    }

    #[test]
    fn cached_factorization_matches_reference_with_parallel_mixed_geometries() {
        let cases = references();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(10)
            .build()
            .unwrap();
        // More configurations than the cache capacity, repeated to exercise
        // concurrent cache hits, misses and eviction across all three lambdas.
        pool.install(|| {
            (0..cases.len() * 3 * 3).into_par_iter().for_each(|i| {
                let case = &cases[i % cases.len()];
                let penalty = (i / cases.len()) % 3;
                let mut settings = case.settings.clone();
                settings.clamp_lambda = Some([0.0, 0.001, 1.0][penalty]);
                settings.linear_workspace_cache = Some(true);
                let actual = calculate(case, settings, &case.mu).unwrap();
                let chi = actual.chi.unwrap();
                assert_eq!(chi.len(), case.chi[penalty].len());
                for (actual, expected) in chi.iter().zip(&case.chi[penalty]) {
                    assert!(
                        (actual - expected).abs() <= 1e-12 + 1e-11 * expected.abs(),
                        "{} penalty={penalty}: {actual} != {expected}",
                        case.name
                    );
                }
            });
        });
    }

    #[test]
    fn factorization_rejects_invalid_designs_and_rhs() {
        let grid = DVector::from_iterator(81, (0..81).map(|i| i as f64 * 0.05));
        let problem = AUTOBKSpline {
            knots: DVector::from_vec(vec![0.0, 0.0, 0.0, 0.0, 2.0, 4.0, 4.0, 4.0, 4.0]),
            kraw: grid.clone(),
            kout: grid,
            mu: DVector::from_element(81, 1.0),
            ftwin: DVector::from_element(81, 1.0),
            nfft: 128,
            irbkg: 8,
            ..Default::default()
        };
        let settings = AUTOBK::new();
        assert!(solve(&problem, &settings).is_ok());
        for (mut invalid, expected) in [
            (problem.clone(), "non-finite fixed-penalty system"),
            (problem.clone(), "zero/non-finite design column"),
            (problem.clone(), "underdetermined fixed-penalty system"),
        ] {
            match expected {
                "non-finite fixed-penalty system" => invalid.ftwin[0] = f64::NAN,
                "zero/non-finite design column" => invalid.ftwin.fill(0.0),
                _ => invalid.irbkg = 1,
            }
            assert!(matches!(
                solve(&invalid, &settings),
                Err(BackgroundError::DirectSolverFailed { reason }) if reason == expected
            ));
        }
        // The valid geometry above is warm; non-finite data must still fail.
        let mut invalid = problem.clone();
        invalid.mu[0] = f64::NAN;
        assert!(matches!(
            solve(&invalid, &settings),
            Err(BackgroundError::DirectSolverFailed { reason })
                if reason == "non-finite fixed-penalty system"
        ));
        assert!(solve(&problem, &settings).is_ok());
    }

    #[test]
    fn production_matches_independent_larch_model_fixed_objective() {
        for case in references() {
            for (lambda, expected) in [0.0, 0.001, 1.0].into_iter().zip(&case.chi) {
                for (cached, gain) in [(false, 1.0), (true, 1.0), (true, 10.0), (true, 0.1)] {
                    let mut settings = case.settings.clone();
                    settings.clamp_lambda = Some(lambda);
                    settings.linear_workspace_cache = Some(cached);
                    let mut norm = Some(normalization::NormalizationMethod::PrePostEdge(
                        normalization::PrePostEdge {
                            e0: settings.ek0,
                            edge_step: Some(case.edge_step * gain),
                            ..Default::default()
                        },
                    ));
                    settings
                        .calc_background(
                            &DVector::from_vec(case.energy.clone()),
                            &(DVector::from_vec(case.mu.clone()) * gain),
                            &mut norm,
                        )
                        .unwrap();
                    assert_eq!(settings.k.as_ref().unwrap().len(), case.k.len());
                    assert_eq!(settings.chi.as_ref().unwrap().len(), expected.len());
                    for (actual, expected) in settings.k.as_ref().unwrap().iter().zip(&case.k) {
                        assert_abs_diff_eq!(actual, expected, epsilon = 1e-14);
                    }
                    for (actual, expected) in settings.chi.as_ref().unwrap().iter().zip(expected) {
                        // Same quadratic objective, independent SciPy reference.
                        // Keep an absolute floor at zero crossings and room for
                        // different SVD/FFT roundoff across native and Wasm builds.
                        assert!(
                            (actual - expected).abs() <= 1e-12 + 1e-11 * expected.abs(),
                            "{} lambda={lambda} gain={gain}: {actual} != {expected}",
                            case.name
                        );
                    }
                    if lambda == 0.0 {
                        let expected = settings.chi.clone();
                        settings.nclamp = Some(0);
                        settings
                            .calc_background(
                                &DVector::from_vec(case.energy.clone()),
                                &(DVector::from_vec(case.mu.clone()) * gain),
                                &mut norm,
                            )
                            .unwrap();
                        assert_eq!(settings.chi, expected);
                    }
                }
            }
        }
    }
}
