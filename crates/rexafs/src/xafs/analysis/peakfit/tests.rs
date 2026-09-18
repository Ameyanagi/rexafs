use super::*;
use std::f64::consts::{LN_2, PI};

fn gaussian(x: f64, center: f64, area: f64, width: f64) -> f64 {
    area * 2. * (LN_2 / PI).sqrt() / width * (-4. * LN_2 * ((x - center) / width).powi(2)).exp()
}
fn close(a: f64, b: f64, tol: f64) {
    assert!(
        (a - b).abs() <= tol,
        "{a:.15e} != {b:.15e}, tolerance {tol}"
    );
}

#[test]
fn overlapping_peaks_joint_baseline_ties_masks_and_irregular_grid() {
    let x: Vec<_> = (0..501)
        .map(|i| -20. + 0.12 * i as f64 + 0.012 * (i as f64).sin())
        .collect();
    let energy: Vec<_> = x.iter().map(|v| 9000. + v).collect();
    let y: Vec<_> = x
        .iter()
        .map(|&v| 0.2 + 0.003 * v + gaussian(v, 2., 4., 5.) + gaussian(v, 8., 3., 5.))
        .collect();
    let definition = PeakFit::new(-19.0..=39.0)
        .gaussian("p1", 1., 3., 4.)
        .gaussian("p2", 9., 2., 4.)
        .linear_baseline(0.1, 0.)
        .parameter(Param::expr("p2_width", "p1_width"))
        .exclude(3.0..=4.0);
    let result = definition
        .fit_prepared(&energy, &y, Some(9000.), None)
        .unwrap();
    assert_eq!(
        result.termination,
        PeakTermination::Converged,
        "{}",
        result.termination_detail
    );
    for (name, truth) in [
        ("p1_center", 2.),
        ("p1_area", 4.),
        ("p1_width", 5.),
        ("p2_center", 8.),
        ("p2_area", 3.),
        ("p2_width", 5.),
        ("baseline_offset", 0.2),
        ("baseline_slope", 0.003),
    ] {
        close(result.parameters.vars[name].value, truth, 1e-7);
    }
    close(result.peak_center_ev.unwrap(), 9000. + 32. / 7., 1e-7);
    assert!(result.objective < 1e-20);
    assert!(
        result.covariance.is_some(),
        "{:?}",
        result.uncertainty_unavailable
    );
    assert_eq!(
        result.parameters.vars["p1_width"].stderr,
        result.parameters.vars["p2_width"].stderr
    );
    assert!(!result.energy.iter().any(|e| (9003.0..=9004.0).contains(e)));
    assert_eq!(definition.parameters.vars["p1_center"].value, 1.);
    assert!(result.components[0].sampled_integral < result.components[0].area.unwrap());
    let roundtrip: PeakFitResult =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert_eq!(roundtrip.source_indices, result.source_indices);
    assert_eq!(roundtrip.definition.exclude, vec![[3., 4.]]);
    assert_eq!(
        roundtrip.parameters.vars["p2_width"].expr.as_deref(),
        Some("p1_width")
    );
}

#[test]
fn weighted_linear_fit_and_covariance_match_closed_form() {
    let x: Vec<_> = (0..41).map(|i| i as f64 / 2.).collect();
    let y: Vec<_> = x
        .iter()
        .enumerate()
        .map(|(i, &v)| 3. + 0.2 * v + 0.03 * (i as f64 * 0.7).cos())
        .collect();
    let sigma: Vec<_> = x.iter().map(|v| 0.02 + 0.003 * v).collect();
    let (mut sw, mut sx, mut sxx, mut sy, mut sxy) = (0., 0., 0., 0., 0.);
    for ((x, y), sigma) in x.iter().zip(&y).zip(&sigma) {
        let w = 1. / (sigma * sigma);
        sw += w;
        sx += x * w;
        sxx += x * x * w;
        sy += y * w;
        sxy += x * y * w;
    }
    let det = sw * sxx - sx * sx;
    let truth = [(sxx * sy - sx * sxy) / det, (sw * sxy - sx * sy) / det];
    let r = PeakFit::new(0.0..=20.0)
        .absolute()
        .linear_baseline(0., 0.)
        .fit_prepared(&x, &y, None, Some(&sigma))
        .unwrap();
    assert_eq!(r.termination, PeakTermination::Converged);
    close(r.parameters.vars["baseline_offset"].value, truth[0], 1e-9);
    close(r.parameters.vars["baseline_slope"].value, truth[1], 1e-9);
    let cov = r.covariance.as_ref().unwrap();
    close(cov[0][0], sxx / det, 1e-10);
    close(cov[0][1], -sx / det, 1e-10);
    close(cov[1][1], sw / det, 1e-10);
    assert_eq!(r.free_parameters, 2);
    assert_eq!(r.degrees_of_freedom, 39);
    assert_eq!(r.jacobian_rank, 2);
    assert_eq!(r.peak_center_ev, None);
    let expected: f64 = x
        .iter()
        .zip(&y)
        .zip(&sigma)
        .map(|((x, y), s)| ((y - truth[0] - truth[1] * x) / s).powi(2))
        .sum();
    close(r.objective, expected, 1e-10);
}

#[test]
fn exactly_zero_residual_does_not_invent_correlations() {
    let x: Vec<_> = (0..11).map(|i| i as f64).collect();
    let result = PeakFit::new(0.0..=10.0)
        .absolute()
        .constant_baseline(3.)
        .fit_prepared(&x, &[3.; 11], None, None)
        .unwrap();
    assert_eq!(result.objective, 0.);
    assert_eq!(result.covariance, Some(vec![vec![0.]]));
    assert!(result.correlation.is_none());
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("variance is zero")));
}

#[test]
fn bound_optimum_is_retained_without_false_covariance() {
    let x: Vec<_> = (0..101).map(|i| i as f64 / 10.).collect();
    let y: Vec<_> = x.iter().map(|x| 3. + 0.2 * x).collect();
    let definition = PeakFit::new(0.0..=10.0)
        .absolute()
        .linear_baseline(0.5, 0.)
        .parameter(Param::new("baseline_offset", 0.5).bounds(0., 1.));
    let r = definition.fit_prepared(&x, &y, None, None).unwrap();
    let expected_slope = x.iter().zip(&y).map(|(x, y)| x * (y - 1.)).sum::<f64>()
        / x.iter().map(|x| x * x).sum::<f64>();
    close(r.parameters.vars["baseline_offset"].value, 1., 1e-10);
    close(
        r.parameters.vars["baseline_slope"].value,
        expected_slope,
        1e-6,
    );
    assert_eq!(
        r.termination,
        PeakTermination::Converged,
        "{}",
        r.termination_detail
    );
    assert!(r.covariance.is_none());
    assert!(r.warnings.iter().any(|s| s.contains("baseline_offset")));
}

#[test]
fn fixed_models_and_unidentifiable_models_have_explicit_uncertainty_status() {
    let x: Vec<_> = (0..101).map(|i| i as f64 / 10.).collect();
    let y = vec![3.; x.len()];
    let fixed = PeakFit::new(0.0..=10.0)
        .absolute()
        .constant_baseline(3.)
        .parameter(Param::fixed("baseline_offset", 3.));
    let r = fixed.fit_prepared(&x, &y, None, None).unwrap();
    assert_eq!(r.termination, PeakTermination::FixedModel);
    assert_eq!(r.objective, 0.);
    assert!(r.covariance.is_none());
    let degenerate = PeakFit::new(0.0..=10.0)
        .absolute()
        .constant_baseline(1.)
        .component(
            "second".into(),
            PeakShape::Constant,
            PeakRole::Baseline,
            &[1.],
        );
    let r = degenerate.fit_prepared(&x, &y, None, None).unwrap();
    assert!(r.objective < 1e-20);
    assert!(r.jacobian_rank < r.free_parameters);
    assert!(r.covariance.is_none());
    assert!(r.parameters.vars.values().all(|p| p.stderr.is_none()));
}

#[test]
fn malformed_inputs_and_models_return_errors() {
    let x = vec![0., 1., 2., 3., 4.];
    let y = vec![1.; 5];
    let model = PeakFit::new(0.0..=4.0).absolute().constant_baseline(1.);
    assert!(model
        .fit_prepared(&x, &y, None, Some(&[1., 1., 0., 1., 1.]))
        .is_err());
    assert!(model
        .fit_prepared(&[0., 1., 1., 3., 4.], &y, None, None)
        .is_err());
    assert!(model
        .fit_prepared(&x, &[1., f64::NAN, 1., 1., 1.], None, None)
        .is_err());
    assert!(model
        .clone()
        .exclude(0.0..=4.0)
        .fit_prepared(&x, &y, None, None)
        .is_err());
    assert!(model
        .clone()
        .parameter(Param::expr("baseline_offset", "absent+1"))
        .validate()
        .is_err());
    assert!(model
        .clone()
        .parameter(Param::expr("baseline_offset", "baseline_offset"))
        .validate()
        .is_err());
    assert!(model.clone().reference(f64::NAN).validate().is_err());
    assert!(PeakFit::new(4.0..=0.0)
        .constant_baseline(1.)
        .validate()
        .is_err());
    assert!(PeakFit::new(0.0..=4.0)
        .absolute()
        .gaussian("p", 2., 1., 0.)
        .validate()
        .is_err());
    assert!(PeakFit::new(0.0..=4.0)
        .absolute()
        .voigt("p", 2., 1., 0., 0.)
        .validate()
        .is_err());
    assert!(PeakFit::new(0.0..=4.0)
        .absolute()
        .gaussian("p", 2., 1., 1.)
        .gaussian("p", 3., 1., 1.)
        .validate()
        .is_err());
}

#[test]
fn cancellation_and_iteration_limits_never_publish_convergence_or_errors() {
    let x: Vec<_> = (0..101).map(|i| i as f64 / 10.).collect();
    let y: Vec<_> = x.iter().map(|x| gaussian(*x, 4., 3., 2.)).collect();
    let model = PeakFit::new(0.0..=10.0)
        .absolute()
        .gaussian("p", 2., 1., 5.);
    let r = model
        .fit_prepared_with_progress(&x, &y, None, None, |iteration, _| iteration < 2)
        .unwrap();
    assert_eq!(r.termination, PeakTermination::Cancelled);
    assert!(r.covariance.is_none());
    assert!(r.parameters.vars.values().all(|p| p.stderr.is_none()));
    let mut limited = model;
    limited.max_iterations = 1;
    let r = limited.fit_prepared(&x, &y, None, None).unwrap();
    assert_eq!(r.termination, PeakTermination::NotConverged);
    assert!(r.covariance.is_none());
}

#[test]
fn parameters_can_move_inward_from_initial_bounds() {
    let x: Vec<_> = (0..101).map(|i| i as f64 / 10.).collect();
    let y: Vec<_> = x.iter().map(|x| 2. + 0.3 * x).collect();
    for initial in [0., 4.] {
        let r = PeakFit::new(0.0..=10.0)
            .absolute()
            .linear_baseline(initial, 0.)
            .parameter(Param::new("baseline_offset", initial).bounds(0., 4.))
            .fit_prepared(&x, &y, None, None)
            .unwrap();
        close(r.parameters.vars["baseline_offset"].value, 2., 1e-7);
        close(r.parameters.vars["baseline_slope"].value, 0.3, 1e-7);
        assert!(r.covariance.is_some());
    }
}

#[test]
fn profile_scaling_and_baseline_initialization_keep_the_final_peak_interval() {
    for width in [1e-5, 1., 1e5] {
        for shape in [
            PeakShape::Gaussian,
            PeakShape::Lorentzian,
            PeakShape::PseudoVoigt,
            PeakShape::Voigt,
        ] {
            let p = [
                0.,
                3.,
                width,
                if shape == PeakShape::Voigt {
                    0.4 * width
                } else {
                    0.4
                },
            ];
            let q = [0., 3., 1., 0.4];
            close(
                profiles::evaluate(shape, 0.3 * width, &p) * width,
                profiles::evaluate(shape, 0.3, &q),
                1e-12,
            );
        }
    }
    let x: Vec<_> = (0..301).map(|i| -15. + i as f64 / 10.).collect();
    let y: Vec<_> = x
        .iter()
        .map(|&x| 0.1 + 0.002 * x + gaussian(x, 1., 3., 2.))
        .collect();
    let sp = XASSpectrum::from_arrays(&x, &y).unwrap();
    let model = PeakFit::new(-15.0..=15.0)
        .raw_mu()
        .absolute()
        .gaussian("p", 0., 2., 3.)
        .linear_baseline(1., 0.);
    let initialized = model.initialize_baseline(&sp, &[-6.0..=8.0]).unwrap();
    assert!(initialized.exclude.is_empty());
    close(
        initialized.parameters.vars["baseline_offset"].value,
        0.1,
        1e-9,
    );
    close(
        initialized.parameters.vars["baseline_slope"].value,
        0.002,
        1e-9,
    );
    assert_eq!(model.parameters.vars["baseline_offset"].value, 1.);
    let r = sp.fit_peaks(&initialized).unwrap();
    assert_eq!(r.points, x.len());
    close(r.parameters.vars["p_center"].value, 1., 1e-7);
    let reconstructed = r.fitted_model().evaluate(&x, None).unwrap();
    for (a, b) in r.model.iter().zip(&reconstructed) {
        close(*a, *b, 1e-15);
    }
}
