//! Source-checkout regressions using original, attributed beamline measurements.
//!
//! Cargo excludes this test and `tests/fixtures/xas/` from the published crate.
//! Tests read local files without downloading data. The corpus manifest records
//! licenses, citations and checksums; untested formats remain future fixtures.

use std::path::PathBuf;

use rexafs::io::{read_qas_transmission, AthenaProject, XdiFile, XdiSignal};

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/xas/samples")
        .join(path)
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

// Counts, declared beamlines and endpoint values were read from the original
// files independently of rexafs. Absorption expectations follow their declared
// columns: direct mutrans, ln(i0/itrans) for transmission, or ifluor/i0 for
// fluorescence. See XdiSignal for the convention and the corpus citations for
// each measurement. Negative transmission values from detector gains are kept.
macro_rules! xdi_case {
    ($name:ident, $path:literal, $beamline:literal, $rows:literal,
     $first:literal, $last:literal, $mu:literal, $signal:ident) => {
        #[test]
        fn $name() {
            let file = XdiFile::read(fixture($path)).unwrap();
            assert_eq!(file.header.get("beamline.name"), Some($beamline));
            assert_eq!(file.data.len(), $rows);
            let spectrum = file.to_spectrum(XdiSignal::$signal).unwrap();
            let energy = spectrum.energy.as_ref().unwrap();
            let mu = spectrum.mu.as_ref().unwrap();
            assert_eq!(energy.len(), $rows);
            assert_eq!(mu.len(), $rows);
            close(energy[0], $first);
            close(energy[$rows - 1], $last);
            close(mu[0], $mu);
            assert!(mu.iter().all(|value| value.is_finite()));
            assert!(spectrum.normalization.is_none());
        }
    };
}

xdi_case!(
    aps_13bmd_transmission_reordered_columns,
    "aps/13-bm-d/xasdatalibrary/Chorover13BM_ZnSO4_rt_01.xdi",
    "13-BM-D",
    415,
    9459.017,
    10207.66,
    0.9829411051222834,
    Transmission
);
xdi_case!(
    aps_13idc_cobalt_transmission,
    "aps/13-id-c/xasdatalibrary/co_metal_rt.xdi",
    "APS 13-ID-C",
    417,
    7519.0,
    8943.435,
    -0.7849349000011877,
    Transmission
);
xdi_case!(
    aps_13idc_copper_precomputed_absorption,
    "aps/13-id-c/xasdatalibrary/cu_metal_rt.xdi",
    "13-ID-C",
    408,
    8779.0,
    10145.86,
    -1.3070486,
    Transmission
);
xdi_case!(
    aps_13ide_sulfur_fluorescence,
    "aps/13-id-e/xasdatalibrary/CdS_rt_01.xdi",
    "13-ID-E",
    229,
    2450.0,
    2550.0,
    0.04026008302062878,
    Fluorescence
);
xdi_case!(
    aps_20bm_iron_oxide,
    "aps/20-bm/xasdatalibrary/Fe2O3_rt_01.xdi",
    "APS 20-BM-B",
    412,
    6911.8277,
    8084.2337,
    0.809265410001222,
    Transmission
);
xdi_case!(
    nsls_x11a_two_column_absorption,
    "nsls/x11a/xasdatalibrary/cu_metal_10K.xdi",
    "X11A",
    612,
    8786.204,
    11362.47,
    1.013661,
    Transmission
);
xdi_case!(
    ssrl_23_chromium_oxide,
    "ssrl/2-3/xasdatalibrary/cr2o3_100K_001.xdi",
    "SSRL 2-3",
    396,
    5759.989,
    7111.059,
    2.955306999960213,
    Transmission
);
xdi_case!(
    ssrl_23_arsenic_oxide,
    "ssrl/2-3/xasdatalibrary/as2o3_10K_scan1.xdi",
    "SSRL 2-3",
    413,
    11634.89,
    12885.069,
    2.175175999997152,
    Transmission
);
xdi_case!(
    ssrl_41_chromium_oxide,
    "ssrl/4-1/xasdatalibrary/cr2o3_ssrl4_1_rt_001.xdi",
    "SSRL 4-1",
    381,
    5788.999,
    6866.25,
    2.568085000004303,
    Transmission
);
xdi_case!(
    ssrl_43_copper_sulfide,
    "ssrl/4-3/xasdatalibrary/Cu2S_13K_01.xdi",
    "SSRL 4-3",
    454,
    8759.99,
    9852.277,
    -1.5581929999995054,
    Transmission
);
xdi_case!(
    ssrl_43_manganese_oxide,
    "ssrl/4-3/xasdatalibrary/MnO_rt_01.xdi",
    "SSRL 4-3",
    217,
    6520.003,
    6629.979,
    -0.23544549999563585,
    Transmission
);

macro_rules! qas_case {
    ($name:ident, $file:literal, $rows:literal, $first:literal, $last:literal, $mu:literal) => {
        #[test]
        fn $name() {
            let spectrum =
                read_qas_transmission(fixture(concat!("nsls-ii/7-bm-qas/xasref/", $file))).unwrap();
            let energy = spectrum.energy.as_ref().unwrap();
            let mu = spectrum.mu.as_ref().unwrap();
            assert_eq!(energy.len(), $rows);
            assert_eq!(mu.len(), $rows);
            close(energy[0], $first);
            close(energy[$rows - 1], $last);
            close(mu[0], $mu);
            assert!(mu.iter().all(|value| value.is_finite()));
        }
    };
}

qas_case!(
    qas_molybdenum_small_detector_values,
    "Mo foil 0001-r0003.dat",
    651,
    19970.0,
    21153.530811,
    0.06270592290671477
);
qas_case!(
    qas_palladium_extra_detector_columns,
    "Pd foil 0001.dat",
    646,
    24174.199353,
    25330.257542,
    0.13494022086931526
);
qas_case!(
    qas_rhodium_extra_detector_columns,
    "Rh foil 0001.dat",
    647,
    23042.998506,
    24200.257542,
    0.15441081939298854
);

macro_rules! athena_case {
    ($name:ident, $path:literal, $groups:literal, $label:literal,
     $points:literal, $first:literal, $last:literal, $signal:literal) => {
        #[test]
        fn $name() {
            let project = AthenaProject::read(fixture($path)).unwrap();
            assert_eq!(project.groups.len(), $groups);
            let group = &project.groups[0];
            assert_eq!(group.label, $label);
            assert_eq!(group.x.len(), $points);
            assert_eq!(group.y.len(), $points);
            close(group.x[0], $first);
            close(group.x[$points - 1], $last);
            close(group.y[0], $signal);
            // Round-tripping must retain every measured array, including the
            // pixel axes in the dispersive project; no energy conversion here.
            let text = project.to_text();
            let reread = AthenaProject::from_text(&text).unwrap();
            assert_eq!(reread.groups.len(), project.groups.len());
            for (original, restored) in project.groups.iter().zip(&reread.groups) {
                assert_eq!(original.label, restored.label);
                assert_eq!(original.x, restored.x);
                assert_eq!(original.y, restored.y);
            }
        }
    };
}

athena_case!(
    als_compressed_iron_standards,
    "als/10-3-2/xraylarch/Fe-K-ALS-10.3.2.prj",
    8,
    "Chromite",
    251,
    7010.7057,
    7414.7002,
    0.0010534763
);
athena_case!(
    camd_compressed_silver_standards,
    "camd/unspecified/xraylarch/AgL3_CAMD.prj",
    29,
    "Ag metal (metal flakes)",
    581,
    3299.95,
    3499.88,
    2.04195522595114
);
athena_case!(
    camd_compressed_phosphorus_standards,
    "camd/unspecified/xraylarch/P_CAMD.prj",
    15,
    "H3PO4 solution 1 M",
    415,
    2100.02,
    2200.18,
    274.455
);
athena_case!(
    esrf_plain_text_dispersive_project,
    "esrf/id24/xraylarch/dispersive.prj",
    12,
    "cufoil_rt_txt",
    725,
    8679.01,
    10180.6,
    0.00110623
);
