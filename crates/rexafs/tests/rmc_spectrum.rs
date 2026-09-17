//! Spectrum input must preserve preprocessing and the native complex-R objective.
use nalgebra::DVector;
use rexafs::fitting::{
    transform::{apply_dataset_transform, residual_for_dataset},
    FeffFitTransform, FitSpace,
};
use rexafs::rmc::*;
use rexafs::structure::Edge;
use rexafs::xafs::xafsutils::FTWindow;
use rexafs::{analysis::AnalysisSpace, BackgroundMethod, Spectrum, XrayFFTF};

fn spectrum() -> Spectrum {
    let energy: Vec<_> = (0..=650).map(|i| 8800. + 2. * i as f64).collect();
    let mu: Vec<_> = energy
        .iter()
        .map(|e| {
            let above = (e - 8980.).max(0.);
            let k = (above / 3.81).sqrt();
            1. / (1. + (-(e - 8980.) / 2.).exp())
                + 0.06 * (4. * k).sin() * (-above / 800.).exp() * (above / 20.).min(1.)
        })
        .collect();
    let mut spectrum = Spectrum::from_prepared(&energy, &mu, AnalysisSpace::Norm, 8980.).unwrap();
    spectrum.set_name("prepared Cu").calc_background().unwrap();
    spectrum
        .set_fft(XrayFFTF {
            kmin: Some(2.),
            kmax: Some(13.),
            kweight: Some(3.),
            ..Default::default()
        })
        .fft()
        .unwrap();
    spectrum.mu_stddev = Some(DVector::from_element(energy.len(), 0.25));
    spectrum
}

fn options() -> RmcSpectrumOptions {
    let mut options = RmcSpectrumOptions::new(
        vec![0],
        Edge::K,
        FeffFitTransform {
            kmin: 3.,
            kmax: 11.5,
            kweight: 2.,
            dk: 1.,
            dk2: Some(1.),
            window: FTWindow::Hanning,
            rmin: 1.15,
            rmax: 4.,
            ..Default::default()
        },
    );
    options.k_range = Some([2.5, 12.]);
    options.s02 = 0.98;
    options.delta_e0 = 8.7;
    options
}

fn configuration() -> Configuration {
    Configuration {
        atoms: vec![
            Atom {
                atomic_number: 29,
                position: [0.; 3],
            },
            Atom {
                atomic_number: 8,
                position: [1.9, 0., 0.],
            },
        ],
        cell: None,
    }
}

#[derive(Default)]
struct Toy {
    calls: usize,
    fail: bool,
}
impl ExafsCalculator for Toy {
    fn name(&self) -> &str {
        "spectrum adapter deterministic test"
    }
    fn calculate(
        &mut self,
        c: &Configuration,
        _: usize,
        _: Edge,
        k: &[f64],
    ) -> Result<Vec<f64>, RmcError> {
        self.calls += 1;
        if self.fail {
            return Err(RmcError::Calculator("test failure".into()));
        }
        let r = c.atoms[1]
            .position
            .iter()
            .map(|x| x * x)
            .sum::<f64>()
            .sqrt();
        Ok(k.iter().map(|q| 0.05 * (2. * q * r).sin()).collect())
    }
}

#[test]
fn authoritative_samples_and_full_snapshot_are_unchanged() {
    let mut spectrum = spectrum();
    // Legacy slots and plotting weights must not override background getters.
    spectrum.k = Some(DVector::from_element(3, 999.));
    spectrum.chi = Some(DVector::from_element(3, 999.));
    let before = serde_json::to_value(&spectrum).unwrap();
    let dataset = RmcDataset::from_spectrum(&spectrum, options()).unwrap();
    assert_eq!(serde_json::to_value(&spectrum).unwrap(), before);
    let source = dataset.source.as_ref().unwrap();
    assert_eq!(serde_json::to_value(source.spectrum()).unwrap(), before);
    assert_eq!(dataset.exafs.kweight, 2);
    assert_eq!(dataset.exafs.delta_e0, 8.7);
    assert_eq!(source.spectrum().e0(), Some(8980.));
    assert!(dataset.exafs.sigma.iter().all(|s| *s == 1.));
    for (j, &i) in source.selected_indices().iter().enumerate() {
        assert_eq!(dataset.exafs.k[j], spectrum.k().unwrap()[i]);
        assert_eq!(dataset.exafs.chi[j], spectrum.chi().unwrap()[i]);
    }
    spectrum.set_e0(8985.);
    assert_eq!(source.spectrum().e0(), Some(8980.));
    let restored: RmcDataset =
        serde_json::from_slice(&serde_json::to_vec(&dataset).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&dataset).unwrap(),
        serde_json::to_value(restored).unwrap()
    );
}

#[test]
fn normalized_score_matches_native_real_imaginary_residual() {
    let spectrum = spectrum();
    for scaled in [false, true] {
        let mut options = options();
        if scaled {
            options.sigma = Some(
                spectrum
                    .k()
                    .unwrap()
                    .iter()
                    .map(|k| 0.02 + 0.001 * k)
                    .collect(),
            );
        }
        let transform = options.transform.clone();
        let dataset = RmcDataset::from_spectrum(&spectrum, options).unwrap();
        let d = &dataset.exafs;
        let model: Vec<_> = d
            .chi
            .iter()
            .zip(&d.k)
            .map(|(y, k)| 0.8 * y + 0.003 * (3. * k).cos())
            .collect();
        // Independent native-fitter route: zero-pad, scale data/theory once,
        // then compare its interleaved real+imaginary residual vectors.
        let count = (d.k.last().unwrap() / 0.05).round() as usize + 1;
        let grid = DVector::from_iterator(count, (0..count).map(|i| i as f64 * 0.05));
        let pad = |values: &[f64]| {
            let mut out = DVector::zeros(count);
            for ((&k, &y), &sigma) in d.k.iter().zip(values).zip(&d.sigma) {
                out[(k / 0.05).round() as usize] = y / sigma;
            }
            out
        };
        let observed = apply_dataset_transform(&grid, &pad(&d.chi), &transform).unwrap();
        let predicted = apply_dataset_transform(&grid, &pad(&model), &transform).unwrap();
        let residual =
            residual_for_dataset(&observed, Some(&predicted), &transform, &[1.]).unwrap();
        let baseline = residual_for_dataset(&observed, None, &transform, &[1.]).unwrap();
        let expected = residual.norm_squared() / baseline.norm_squared();
        let actual = dataset.objective.score(d, &model).unwrap();
        assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
        assert!((dataset.objective.score(d, &vec![0.; d.k.len()]).unwrap() - 1.).abs() < 1e-12);
    }
}

#[test]
fn normalization_is_explicit_and_zero_power_errors_are_transactional() {
    let mut spectrum = spectrum();
    let mut input = options();
    input.normalize_by_experiment = false;
    let mut dataset = RmcDataset::from_spectrum(&spectrum, input.clone()).unwrap();
    assert_eq!(dataset.exafs.weight, 1.);
    dataset.normalize_experimental_power().unwrap();
    let weight = dataset.exafs.weight;
    dataset.normalize_experimental_power().unwrap();
    assert_eq!(dataset.exafs.weight, weight);
    dataset.exafs.chi.fill(0.);
    assert!(dataset.normalize_experimental_power().is_err());
    assert_eq!(dataset.exafs.weight, weight);
    let Some(BackgroundMethod::AUTOBK(background)) = &mut spectrum.background else {
        panic!()
    };
    background.chi.as_mut().unwrap().fill(0.);
    assert!(RmcDataset::from_spectrum(&spectrum, input).is_ok());
    assert!(RmcDataset::from_spectrum(&spectrum, options())
        .unwrap_err()
        .to_string()
        .contains("power"));
}

#[test]
fn rbkg_override_is_explicit_and_guard_is_rechecked_before_scattering() {
    let spectrum = spectrum();
    let mut low = options();
    low.transform.rmin = 0.8;
    assert!(RmcDataset::from_spectrum(&spectrum, low.clone())
        .unwrap_err()
        .to_string()
        .contains("Rbkg"));
    low.rbkg_policy = RmcRbkgPolicy::AllowBelowRbkg { reason: " ".into() };
    assert!(RmcDataset::from_spectrum(&spectrum, low.clone()).is_err());
    low.rbkg_policy = RmcRbkgPolicy::AllowBelowRbkg {
        reason: "Reproduce the archived low-R comparison".into(),
    };
    let allowed = RmcDataset::from_spectrum(&spectrum, low.clone()).unwrap();
    assert_eq!(
        allowed.source.as_ref().unwrap().options().rbkg_policy,
        low.rbkg_policy
    );
    let dataset = RmcDataset::from_spectrum(&spectrum, options()).unwrap();
    let mut problem = EnsembleProblem::single(configuration(), dataset);
    if let Objective::R(t) = &mut problem.datasets[0].objective {
        t.rmin = 0.8;
    }
    let mut calc = Toy::default();
    assert!(RmcSession::new(&problem, &SessionSettings::default(), &mut calc).is_err());
    assert_eq!(calc.calls, 0);
    problem.datasets[0] = allowed;
    assert!(RmcSession::new(&problem, &SessionSettings::default(), &mut calc).is_ok());
    problem.datasets[0].exafs.chi[0] += 0.01;
    let calls = calc.calls;
    assert!(RmcSession::new(&problem, &SessionSettings::default(), &mut calc).is_err());
    assert_eq!(calc.calls, calls);
}

#[test]
fn rejects_unprocessed_invalid_and_unsupported_inputs_without_mutation() {
    let mut empty = Spectrum::new();
    empty.k = Some(DVector::from_vec(vec![2., 3.]));
    empty.chi = Some(DVector::from_vec(vec![1., 2.]));
    assert!(RmcDataset::from_spectrum(&empty, options())
        .unwrap_err()
        .to_string()
        .contains("calc_background"));
    let mut spectrum = spectrum();
    let before = serde_json::to_value(&spectrum).unwrap();
    let mut variants = Vec::new();
    let mut o = options();
    o.transform.kweight = 1.5;
    variants.push(o);
    let mut o = options();
    o.transform.kweights = vec![1., 2.];
    variants.push(o);
    let mut o = options();
    o.transform.fitspace = FitSpace::K;
    variants.push(o);
    let mut o = options();
    o.sigma = Some(vec![1.; 2]);
    variants.push(o);
    let mut o = options();
    o.sigma = Some(vec![0.; spectrum.k().unwrap().len()]);
    variants.push(o);
    let mut o = options();
    o.k_range = Some([3., 2.]);
    variants.push(o);
    let mut o = options();
    o.k_range = Some([100., 200.]);
    variants.push(o);
    let mut o = options();
    o.absorbers = vec![0, 0];
    variants.push(o);
    let mut o = options();
    o.s02 = f64::NAN;
    variants.push(o);
    let mut o = options();
    o.k_range = None;
    variants.push(o); // Positive ΔE₀ at k=0.
    for o in variants {
        assert!(RmcDataset::from_spectrum(&spectrum, o).is_err());
    }
    assert_eq!(serde_json::to_value(&spectrum).unwrap(), before);
    let Some(BackgroundMethod::AUTOBK(bkg)) = &mut spectrum.background else {
        panic!()
    };
    bkg.rbkg = None;
    assert!(RmcDataset::from_spectrum(&spectrum, options()).is_err());
}

#[test]
fn processing_survives_failure_and_exact_rmc_and_evolution_resume() {
    let dataset = RmcDataset::from_spectrum(&spectrum(), options()).unwrap();
    let problem = EnsembleProblem::single(configuration(), dataset);
    let settings = SessionSettings {
        moves: RmcSettings {
            steps: 12,
            movable_atoms: vec![1],
            temperature: 0.001,
            seed: 42,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut calc = Toy::default();
    let mut session = RmcSession::new(&problem, &settings, &mut calc).unwrap();
    for _ in 0..5 {
        session.step(&mut calc).unwrap();
    }
    let before = serde_json::to_value(session.checkpoint()).unwrap();
    calc.fail = true;
    assert!(session.step(&mut calc).is_err());
    assert_eq!(serde_json::to_value(session.checkpoint()).unwrap(), before);
    calc.fail = false;
    let checkpoint: RmcCheckpoint = serde_json::from_value(before).unwrap();
    let mut resumed = RmcSession::resume(checkpoint, &mut Toy::default()).unwrap();
    session.run(&mut calc).unwrap();
    resumed.run(&mut Toy::default()).unwrap();
    assert_eq!(
        serde_json::to_value(session.checkpoint()).unwrap(),
        serde_json::to_value(resumed.checkpoint()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&resumed.problem().datasets[0].source).unwrap(),
        serde_json::to_value(&problem.datasets[0].source).unwrap()
    );
    assert_eq!(
        residual_trend(resumed.history(), &ResidualTrendSettings::default())
            .unwrap()
            .status,
        ResidualTrendStatus::InsufficientHistory
    );
    // Legacy array-only checkpoints remain readable without the optional source.
    let mut legacy = serde_json::to_value(session.checkpoint()).unwrap();
    legacy["problem"]["datasets"][0]
        .as_object_mut()
        .unwrap()
        .remove("source");
    let legacy = RmcSession::resume(serde_json::from_value(legacy).unwrap(), &mut calc).unwrap();
    assert!(legacy.problem().datasets[0].source.is_none());
    let evolution = EvolutionSettings {
        population: 4,
        elite: 1,
        generations: 3,
        local_steps: 2,
        ..Default::default()
    };
    let mut ea = EvolutionSession::new(&problem, &settings, &evolution, &mut calc).unwrap();
    ea.step(&mut calc).unwrap();
    let checkpoint =
        serde_json::from_slice(&serde_json::to_vec(&ea.checkpoint()).unwrap()).unwrap();
    let mut resumed = EvolutionSession::resume(checkpoint, &mut Toy::default()).unwrap();
    ea.run(&mut calc).unwrap();
    resumed.run(&mut Toy::default()).unwrap();
    assert_eq!(
        serde_json::to_value(ea.checkpoint()).unwrap(),
        serde_json::to_value(resumed.checkpoint()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&resumed.problem().datasets[0].source).unwrap(),
        serde_json::to_value(&problem.datasets[0].source).unwrap()
    );
}
