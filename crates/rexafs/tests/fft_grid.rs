//! Independent Larch fixtures cover finite endpoints, asymmetric tapers,
//! irregular sampling and a complex R-space amplitude/phase fit.
use nalgebra::{DMatrix, DVector};
use rexafs::prelude::FTWindow;
use rexafs::{FFTGrid, XrayFFTF};
use serde::Deserialize;

#[derive(Deserialize)]
struct References {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    name: String,
    window: FTWindow,
    k: Vec<f64>,
    chi: Vec<f64>,
    kwin: Vec<f64>,
    real: Vec<f64>,
    imag: Vec<f64>,
    shell_amplitude: f64,
    shell_phase: f64,
}
fn run(ft: &mut XrayFFTF, k: &[f64], chi: &[f64]) -> Result<(), String> {
    #[cfg(not(feature = "ndarray-compat"))]
    let result = ft.xftf(
        &DVector::from_column_slice(k),
        &DVector::from_column_slice(chi),
    );
    #[cfg(feature = "ndarray-compat")]
    let result = ft.xftf(ndarray::ArrayView1::from(k), ndarray::ArrayView1::from(chi));
    result.map(|_| ()).map_err(|e| e.to_string())
}
fn settings(window: FTWindow) -> XrayFFTF {
    XrayFFTF {
        grid: FFTGrid::Larch,
        window: Some(window),
        kmin: Some(2.),
        kmax: Some(12.),
        dk: Some(1.),
        dk2: Some(1.5),
        kweight: Some(2.),
        nfft: Some(512),
        kstep: Some(0.05),
        ..Default::default()
    }
}
#[test]
fn larch_grid_matches_independent_windows_complex_fft_and_shell_fit() {
    let refs: References =
        serde_json::from_str(include_str!("testfiles/fft_grid_larch_reference.json")).unwrap();
    for c in refs.cases {
        let mut ft = settings(c.window);
        run(&mut ft, &c.k, &c.chi).unwrap();
        let win = ft.get_kwin().unwrap();
        assert_eq!(win.len(), c.kwin.len(), "{}", c.name);
        for (&actual, expected) in win.iter().zip(&c.kwin) {
            assert!(
                (actual - expected).abs() < 2e-12,
                "{} window: {actual} vs {expected}",
                c.name
            );
        }
        let chir = ft.get_chir().unwrap();
        for (i, actual) in chir.iter().enumerate() {
            assert!(
                (actual.re - c.real[i]).abs() < 2e-10,
                "{} real[{i}]: {} vs {}",
                c.name,
                actual.re,
                c.real[i]
            );
            assert!(
                (actual.im - c.imag[i]).abs() < 2e-10,
                "{} imag[{i}]",
                c.name
            );
        }
        let mut sine = settings(c.window);
        let mut cosine = settings(c.window);
        run(
            &mut sine,
            &c.k,
            &c.k.iter().map(|k| (4.4 * k).sin()).collect::<Vec<_>>(),
        )
        .unwrap();
        run(
            &mut cosine,
            &c.k,
            &c.k.iter().map(|k| (4.4 * k).cos()).collect::<Vec<_>>(),
        )
        .unwrap();
        let s = sine.get_chir().unwrap();
        let cos = cosine.get_chir().unwrap();
        let bins: Vec<_> = (0..chir.len())
            .filter(|i| (1.0..=3.0).contains(&(*i as f64 * std::f64::consts::PI / (0.05 * 512.0))))
            .collect();
        let mut matrix = DMatrix::zeros(2 * bins.len(), 2);
        let mut data = DVector::zeros(2 * bins.len());
        for (row, &i) in bins.iter().enumerate() {
            matrix[(row, 0)] = s[i].re;
            matrix[(row, 1)] = cos[i].re;
            matrix[(row + bins.len(), 0)] = s[i].im;
            matrix[(row + bins.len(), 1)] = cos[i].im;
            data[row] = chir[i].re;
            data[row + bins.len()] = chir[i].im;
        }
        let fit = matrix.svd(true, true).solve(&data, 1e-12).unwrap();
        assert!(
            (fit.norm() - c.shell_amplitude).abs() < 1e-10,
            "{} amplitude",
            c.name
        );
        assert!(
            (fit[1].atan2(fit[0]) - c.shell_phase).abs() < 1e-10,
            "{} phase",
            c.name
        );
    }
}
#[test]
fn explicit_input_grid_and_serialized_legacy_defaults_are_preserved() {
    let old: XrayFFTF = serde_json::from_str("{}").unwrap();
    #[cfg(not(feature = "ndarray-compat"))]
    assert_eq!(old.grid, FFTGrid::Input);
    #[cfg(feature = "ndarray-compat")]
    assert_eq!(old.grid, FFTGrid::Larch);
    let mut ft = settings(FTWindow::KaiserBessel);
    ft.grid = FFTGrid::Input;
    let k: Vec<_> = (0..241).map(|i| i as f64 * 0.05).collect();
    run(&mut ft, &k, &k).unwrap();
    let last = *ft.get_kwin().unwrap().iter().next_back().unwrap();
    assert!(last.abs() < 1e-12);
    ft.grid = FFTGrid::Larch;
    run(&mut ft, &k, &k).unwrap();
    assert!(*ft.get_kwin().unwrap().iter().next_back().unwrap() > 0.0);
    let json = serde_json::to_string(&ft).unwrap();
    assert_eq!(
        serde_json::from_str::<XrayFFTF>(&json).unwrap().grid,
        FFTGrid::Larch
    );
}
#[test]
fn larch_grid_rejects_invalid_input_and_unbounded_extension() {
    for k in [
        vec![0., 0.],
        vec![1., 0.],
        vec![0., f64::NAN],
        vec![-1., 0.],
    ] {
        assert!(run(&mut settings(FTWindow::Hanning), &k, &[1., 1.]).is_err());
    }
    let mut ft = settings(FTWindow::Hanning);
    ft.dk2 = Some(1e100);
    assert!(run(&mut ft, &[0., 1.], &[1., 1.]).is_err());
}
