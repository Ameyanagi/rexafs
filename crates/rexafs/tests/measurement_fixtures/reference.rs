//! Original Demeter/Larch examples; source URLs and licenses are in the manifest.
//! These tests compare retained values and documented channel conventions, not
//! experimental accuracy or equivalence to upstream data-processing algorithms.
use super::support::xas_root;
use rexafs::io::*;

fn read(path: &str) -> Measurement {
    read_measurement(xas_root().join("samples").join(path)).unwrap()
}

#[test]
fn ex3_crystal_metadata_is_not_used_as_column_labels() {
    let doc = read("spring-8/bl14b2/mdr-3c5953dc/Pb-L1_PbTe_Si311_50ms_210210.ex3");
    let scan = &doc.scans[0];
    assert_eq!(scan.columns[0].name, "energy");
    assert_eq!(scan.columns[0].units.as_deref(), Some("eV"));
    assert_eq!(scan.columns[1].name, "mu");
    assert!(scan.header.contains("*EX_CRYSTAL= SI(111)"));
    let (energy, mu) = scan.arrays(None).unwrap();
    assert_eq!(energy.len(), 624);
    assert_eq!((energy[0], mu[0]), (15531.169236, 0.292268));
}

#[test]
fn hxma_preserves_event_and_pv_labels_and_uses_achieved_axis() {
    let doc = read("cls/hxma/xraylarch/CLSHXMA.dat");
    let scan = &doc.scans[0];
    assert_eq!(doc.format, "cls_acquisition");
    assert_eq!(scan.columns.len(), 11);
    assert_eq!(scan.columns[0].name, "Event-ID");
    assert_eq!(scan.columns[0].values, vec![1.; 414]);
    assert_eq!(scan.columns[3].name, "MONO16061I1001:Energy:sp");
    assert_eq!(scan.signals.len(), 2);
    assert!(scan.arrays(None).is_err());
    let (e, y) = scan.arrays(Some(&scan.signals[0].mapping)).unwrap();
    assert_eq!(e[0], 11667.001);
    assert_eq!(scan.columns[1].values[0], 11667.);
    assert!((y[0] - (180725f64 / 297589.).ln()).abs() < 1e-13);
    assert!(
        (scan.arrays(Some(&scan.signals[1].mapping)).unwrap().1[0] - 14965f64 / 180725.).abs()
            < 1e-13
    );
    assert!(scan.header.contains("$(EnergyAchieved)"));
}

#[test]
fn ssrl_energy_readback_and_micro_detector_roles_are_preserved() {
    let doc = read("ssrl/2-3/demeter/ssrla.dat");
    let (e, y) = doc.scans[0].arrays(None).unwrap();
    assert_eq!(e.len(), 455);
    assert_eq!(e[0], 25284.998);
    assert_eq!(e[454], 26534.926);
    assert!((y[0] - (147360.797f64 / 131905.5).ln()).abs() < 1e-13);
    assert!(doc.scans[0].header.contains("144.200"));
    let doc = read("ssrl/microexafs-unconfirmed/demeter/ssrlmicro.dat");
    let s = &doc.scans[0];
    assert_eq!(s.columns.len(), 69);
    assert_eq!(s.columns[37].name, "ICR.1");
    assert_eq!(s.columns[37].values[0], 621386.);
    assert_eq!(s.signals.len(), 32);
    assert!(s.arrays(None).is_err());
    for (i, signal) in s.signals.iter().enumerate() {
        assert_eq!(
            signal.mapping.signal,
            SignalConversion::Ratio {
                incident: 2,
                detectors: vec![i + 5]
            }
        );
    }
    let (e, y) = s.arrays(Some(&s.signals[0].mapping)).unwrap();
    assert_eq!(e.len(), 296);
    assert_eq!(e[0], 10900.);
    assert!((y[0] - 6f64 / 3229.).abs() < 1e-15);
}

#[test]
fn xdac_keeps_independent_samples_and_uncorrected_vortex_channels() {
    let doc = read("nsls/x23a2/demeter/re4chan.000");
    let s = &doc.scans[0];
    assert_eq!(s.columns.len(), 12);
    assert_eq!(s.signals.len(), 4);
    assert!(s.arrays(None).is_err());
    for (i, (i0, it)) in [
        (13720.5f64, 2864.),
        (14600.25, 5182.5),
        (8839., 5645.25),
        (13071., 3671.75),
    ]
    .into_iter()
    .enumerate()
    {
        let (e, y) = s.arrays(Some(&s.signals[i].mapping)).unwrap();
        assert_eq!(e.len(), 387);
        assert_eq!(e[0], 10334.9987);
        assert_eq!(e[386], 11394.35648);
        assert!((y[0] - (i0 / it).ln()).abs() < 1e-13);
    }
    assert_eq!(s.columns[9].values[0], 72.75);
    let doc = read("nsls/x23a2/demeter/x23a2med.dat");
    let s = &doc.scans[0];
    assert_eq!(s.columns.len(), 17);
    assert_eq!(s.signals.len(), 6);
    assert_eq!(s.columns[7].values[0], 9743.);
    assert_eq!(s.columns[11].values[0], 9799.);
    let detector = s
        .signals
        .iter()
        .find(|s| s.name == "Ifch1 / I0 (uncorrected)")
        .unwrap();
    let (e, y) = s.arrays(Some(&detector.mapping)).unwrap();
    assert_eq!(e.len(), 422);
    assert_eq!(e[0], 6912.0019);
    assert!((y[0] - 845f64 / 4484.).abs() < 1e-13);
}

#[test]
fn larch_reference_axes_distinguish_energy_and_wave_number() {
    let doc = read("aps/12-bm/xraylarch/APS12BM_2019.dat");
    assert_eq!(doc.scans[0].columns[0].units.as_deref(), Some("keV"));
    let (e, y) = doc.scans[0].arrays(None).unwrap();
    assert_eq!(e.len(), 492);
    assert_eq!(e[0], 6339.);
    assert_eq!(y[0], 0.884456);
    for (name, n, first, yfirst, ylast) in [
        (
            "FDMNES_2022_Mo2C_out.dat",
            559,
            -25.,
            0.00011943264,
            0.070504961,
        ),
        (
            "FDMNES_2022_Mo2C_out_conv.dat",
            589,
            -40.,
            0.0017372707,
            0.071744752,
        ),
    ] {
        let doc = read(&format!("unspecified/fdmnes/xraylarch/{name}"));
        let s = &doc.scans[0];
        assert_eq!(doc.format, "fdmnes");
        assert_eq!(s.columns[0].values[0], first);
        let (e, y) = s.arrays(None).unwrap();
        assert_eq!(e.len(), n);
        assert_eq!(e[0], 20000. + first);
        assert_eq!(e[n - 1], 20119.8);
        assert_eq!(y[0], yfirst);
        assert_eq!(y[n - 1], ylast);
        assert_eq!(s.metadata["data_kind"], "calculated reference");
    }
    let doc = read("unspecified/chi/xraylarch/nonuniform.chi");
    assert_eq!(doc.scans[0].columns[0].values.len(), 476);
    assert_eq!(doc.scans[0].columns[0].name, "k");
    assert!(doc.scans[0].signals.is_empty());
    assert!(doc.scans[0].arrays(None).is_err());
}

#[test]
fn historical_athena_metadata_does_not_block_original_spectra() {
    for (name, groups, n, first) in [
        ("AsKa_standards.prj", 3, 312, 11767.),
        ("AsXAFS_a.prj", 46, 397, 11718.03),
        ("Br.prj", 5, 545, 13273.37561),
        ("ESRF_Athena0920.prj", 2, 565, 6999.97),
    ] {
        let doc = read(&format!("unspecified/athena-legacy/xraylarch/{name}"));
        assert_eq!(doc.scans.len(), groups, "{name}");
        let (e, _) = doc.scans[0].arrays(None).unwrap();
        assert_eq!(e.len(), n, "{name}");
        assert_eq!(e[0], first, "{name}");
        if name == "AsXAFS_a.prj" {
            assert!(doc
                .scans
                .iter()
                .any(|s| s.metadata["athena.extra"].contains("@stddev = (undef)")));
        }
        if name == "Br.prj" {
            assert!(doc.scans[0].header.contains("'name' => {'name' => {}}"));
        }
        if name == "AsKa_standards.prj" {
            assert!(doc.scans[0].metadata["athena.project_extra"].contains("@journal = {}"));
        }
    }
}

#[test]
fn multielement_xdi_keeps_count_rates_and_explicit_roi_selection() {
    let doc = read("unspecified/multielement/xraylarch/fe_xanes_8ch.xdi");
    let s = &doc.scans[0];
    assert_eq!(s.columns.len(), 39);
    assert_eq!(s.columns[0].values.len(), 100);
    assert_eq!(s.columns[31].name, "DTFactor_mca1");
    assert_eq!(s.columns[31].values[0], 1.000512);
    let mapping = SpectrumMapping {
        energy_column: 0,
        energy: EnergyConversion::Ev,
        signal: SignalConversion::Ratio {
            incident: 4,
            detectors: vec![15, 16],
        },
    };
    let (e, y) = s.arrays(Some(&mapping)).unwrap();
    assert_eq!(e[0], 7012.);
    assert!((y[0] - 54f64 / 118317.).abs() < 1e-15);
    let generic = read("unspecified/generic/xraylarch/generic_columns_no_header.dat");
    assert_eq!(generic.scans[0].columns.len(), 7);
    assert_eq!(generic.scans[0].columns[0].values.len(), 198);
    assert!(generic.scans[0].signals.is_empty());
}

#[test]
fn japanese_9809_sources_convert_observed_angles_using_the_recorded_crystal() {
    let coverage = include_str!("coverage.csv");
    let mut files = 0;
    for row in coverage.lines().skip(1) {
        let cells: Vec<_> = row.splitn(7, ',').collect();
        if cells[1] != "9809" {
            continue;
        }
        let doc = read_measurement(xas_root().join(cells[0])).unwrap();
        let scan = &doc.scans[0];
        let header_d: f64 = scan
            .header
            .lines()
            .find(|s| s.contains("Mono") && s.contains("D="))
            .unwrap()
            .split_once("D=")
            .unwrap()
            .1
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        for signal in &scan.signals {
            assert_eq!(signal.mapping.energy_column, 1, "{}", cells[0]);
            assert_eq!(
                signal.mapping.energy,
                EnergyConversion::Bragg {
                    d_spacing: header_d,
                    degrees_per_unit: 1.
                }
            );
            let (energy, _) = scan.arrays(Some(&signal.mapping)).unwrap();
            for (index, &angle) in scan.columns[1].values.iter().enumerate() {
                let expected = 12398.419843320026 / (2. * header_d * angle.to_radians().sin());
                assert!(
                    (energy[index] - expected).abs() < 1e-8,
                    "{} row {index}",
                    cells[0]
                );
            }
        }
        files += 1;
    }
    assert!(files >= 18);
}

#[test]
fn esrf_bm16_original_bliss_channels_are_recovered_with_explicit_partial_coverage() {
    // Independent h5py inspection of the complete upstream acquisition file,
    // not values generated by rexafs. The upstream converter identifies keV
    // and these background-subtracted transmission detectors explicitly.
    let doc = read("esrf/bm16/pynxxas/test_Assolution_Mauro_0001.h5");
    assert_eq!(doc.datasets.len(), 1974);
    assert!(doc.warnings.iter().any(|w| w.contains("cannot enumerate")));
    let paths = ["energy_enc", "p201_1_bkg_sub", "p201_3_bkg_sub"]
        .map(|name| format!("/1.1/instrument/{name}/data"));
    let scan = doc.dataset_scan(&paths).unwrap();
    assert_eq!(scan.columns[0].values.len(), 4001);
    assert!((scan.columns[0].values[0] - 11.799814637405676).abs() < 1e-12);
    assert!((scan.columns[1].values[0] - 170171.33).abs() < 1e-8);
    assert!((scan.columns[2].values[0] - 106942.54).abs() < 1e-8);
    assert!(scan.arrays(None).is_err());
    let (energy, mu) = scan
        .arrays(Some(&SpectrumMapping {
            energy_column: 0,
            energy: EnergyConversion::Kev,
            signal: SignalConversion::Transmission {
                incident: 1,
                transmitted: 2,
            },
        }))
        .unwrap();
    assert!((energy[0] - 11799.814637405676).abs() < 1e-8);
    assert!((mu[0] - (170171.33_f64 / 106942.54).ln()).abs() < 1e-12);
}
