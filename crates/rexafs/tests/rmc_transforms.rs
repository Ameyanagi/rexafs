use rexafs::rmc::*;
use rexafs::structure::Edge;

#[test]
fn moment_sums_respect_tolerance_and_fall_back_for_wide_distributions() {
    let lengths: Vec<_> = (0..40).map(|i| 2.5 + (i as f64 - 19.5) * 0.0001).collect();
    let expansion = PathMomentExpansion::new(&lengths, MomentSettings::default()).unwrap();
    let k: Vec<_> = (0..191).map(|i| 2.5 + i as f64 * 0.05).collect();
    assert!(expansion.check_direct(&k).unwrap() < 1e-10);
    assert!(expansion.evaluate(12.).unwrap().order.is_some());
    let broad = PathMomentExpansion::new(
        &(0..40).map(|i| 1. + i as f64 * 0.1).collect::<Vec<_>>(),
        MomentSettings {
            max_order: 4,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(broad.evaluate(12.).unwrap().order, None);
    assert_eq!(broad.check_direct(&k).unwrap(), 0.);
    assert!(PathMomentExpansion::new(&[], MomentSettings::default()).is_err());
}

#[test]
fn fft_wavelet_and_stft_equal_direct_at_edges_and_with_masks() {
    let k: Vec<_> = (0..101).map(|i| 2. + i as f64 * 0.1).collect();
    let values: Vec<_> = k
        .iter()
        .enumerate()
        .map(|(i, &q)| (7. * q).sin() + if i == 0 || i == 100 { 3. } else { 0. })
        .collect();
    for window in [
        LocalSpectrumWindow::Morlet { omega0: 2. },
        LocalSpectrumWindow::GaussianStft { width: 0.4 },
    ] {
        let mut settings = LocalSpectrumSettings {
            k_centers: k.clone(),
            r: vec![0.5, 1., 2., 3., 4.],
            window,
            mask: Vec::new(),
            algorithm: LocalSpectrumAlgorithm::Direct,
        };
        settings.mask = (0..settings.k_centers.len() * settings.r.len())
            .map(|i| if i % 3 == 0 { 0. } else { 0.5 })
            .collect();
        let direct = LocalSpectrumTransform::new(&k, &settings).unwrap();
        let a = direct.transform(&values).unwrap();
        settings.algorithm = LocalSpectrumAlgorithm::Fft;
        let fast = LocalSpectrumTransform::new(&k, &settings).unwrap();
        assert!(fast.uses_fft());
        let b = fast.transform(&values).unwrap();
        assert!(a.iter().zip(&b).all(|(x, y)| (x - y).norm() < 1e-11));
        let data = ExafsDataset {
            name: "local".into(),
            absorbers: vec![0],
            edge: Edge::K,
            k: k.clone(),
            chi: vec![0.; k.len()],
            sigma: vec![2.; k.len()],
            weight: 3.,
            kweight: 2,
            s02: 1.,
            delta_e0: 0.,
        };
        let expected = Objective::LocalSpectrum(settings.clone())
            .score(&data, &values)
            .unwrap();
        settings.algorithm = LocalSpectrumAlgorithm::Direct;
        let actual = Objective::LocalSpectrum(settings)
            .score(&data, &values)
            .unwrap();
        assert!((actual - expected).abs() < 1e-10 * expected);
    }
}

#[test]
fn morlet_matches_existing_objective_and_fft_does_not_resample_irregular_grids() {
    let k: Vec<_> = (0..101).map(|i| 2. + i as f64 * 0.1).collect();
    let values: Vec<_> = k.iter().map(|&q| (3. * q).sin()).collect();
    let data = ExafsDataset {
        name: "local".into(),
        absorbers: vec![0],
        edge: Edge::K,
        k: k.clone(),
        chi: vec![0.; k.len()],
        sigma: vec![0.1; k.len()],
        weight: 0.7,
        kweight: 1,
        s02: 1.,
        delta_e0: 0.,
    };
    let wavelet = WaveletSettings {
        k_centers: k.clone(),
        r: vec![0.2, 1., 2., 3.],
        omega0: 6.,
    };
    let legacy = Objective::Wavelet(wavelet.clone())
        .score(&data, &values)
        .unwrap();
    let mut settings = LocalSpectrumSettings::morlet(wavelet);
    let modern = Objective::LocalSpectrum(settings.clone())
        .score(&data, &values)
        .unwrap();
    assert!((modern - legacy).abs() < 1e-12 * legacy);
    let mut irregular = k.clone();
    irregular[50] += 0.005;
    settings.algorithm = LocalSpectrumAlgorithm::Fft;
    assert!(LocalSpectrumTransform::new(&irregular, &settings).is_err());
    settings.algorithm = LocalSpectrumAlgorithm::Auto;
    assert!(!LocalSpectrumTransform::new(&irregular, &settings)
        .unwrap()
        .uses_fft());
    settings.mask = vec![0.; settings.r.len() * settings.k_centers.len()];
    assert!(LocalSpectrumTransform::new(&k, &settings).is_err());
}

#[test]
fn reported_path_fourier_and_local_maps_add_as_complex_contributions() {
    let k: Vec<_> = (0..161).map(|i| 2. + 0.05 * i as f64).collect();
    let path = |distance: f64| PathContribution {
        index: 1,
        legs: 2,
        degeneracy: 1.,
        half_length: distance,
        chi: k
            .iter()
            .map(|k| (2. * k * distance).sin() / distance.powi(2))
            .collect(),
    };
    let a = path(2.);
    let b = path(2.5);
    let mut sum = a.clone();
    for (v, x) in sum.chi.iter_mut().zip(&b.chi) {
        *v += x;
    }
    let settings = rexafs::fitting::FeffFitTransform {
        kmin: 2.5,
        kmax: 9.5,
        kstep: Some(0.05),
        rmin: 1.,
        rmax: 4.,
        ..Default::default()
    };
    let fa = transform_path_fourier(&a, &k, 2, &settings).unwrap();
    let fb = transform_path_fourier(&b, &k, 2, &settings).unwrap();
    let fs = transform_path_fourier(&sum, &k, 2, &settings).unwrap();
    for ((a, b), s) in fa
        .r_space
        .chir
        .iter()
        .zip(&fb.r_space.chir)
        .zip(&fs.r_space.chir)
    {
        assert!((*a + *b - *s).norm() < 1e-11);
    }
    for ((a, b), s) in fa.chiq.iter().zip(&fb.chiq).zip(&fs.chiq) {
        assert!((a + b - s).abs() < 1e-10);
    }
    let transform = LocalSpectrumTransform::new(
        &k,
        &LocalSpectrumSettings::morlet(WaveletSettings {
            k_centers: k.clone(),
            r: vec![1., 2., 3.],
            omega0: 6.,
        }),
    )
    .unwrap();
    let weighted = |path: &PathContribution| {
        path.chi
            .iter()
            .zip(&k)
            .map(|(v, k)| v * k * k)
            .collect::<Vec<_>>()
    };
    let ma = transform.transform(&weighted(&a)).unwrap();
    let mb = transform.transform(&weighted(&b)).unwrap();
    let ms = transform.transform(&weighted(&sum)).unwrap();
    for ((a, b), s) in ma.iter().zip(&mb).zip(&ms) {
        assert!((*a + *b - *s).norm() < 1e-10);
    }
}
