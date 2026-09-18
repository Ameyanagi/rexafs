//! CSV snapshots of displayed scientific curves. Each series keeps its own grid;
//! missing values are blank cells, never interpolated onto another spectrum.
use std::io::Write;

pub(crate) struct Curve {
    pub label: String,
    pub x_label: String,
    pub y_label: String,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

pub(crate) fn curves_csv(curves: &[Curve], writer: impl Write) -> Result<(), String> {
    if curves.is_empty() {
        return Err("No plotted data to export".into());
    }
    let mut csv = csv::Writer::from_writer(writer);
    csv.write_record([
        "series_index",
        "series",
        "point",
        "x_label",
        "y_label",
        "x",
        "y",
    ])
    .map_err(|e| e.to_string())?;
    for (series_index, curve) in curves.iter().enumerate() {
        if curve.x.len() != curve.y.len() {
            return Err(format!("Coordinate length mismatch for {}", curve.label));
        }
        for (point, (&x, &y)) in curve.x.iter().zip(&curve.y).enumerate() {
            csv.write_record([
                (series_index + 1).to_string(),
                curve.label.clone(),
                (point + 1).to_string(),
                curve.x_label.clone(),
                curve.y_label.clone(),
                number(x),
                number(y),
            ])
            .map_err(|e| e.to_string())?;
        }
    }
    csv.flush().map_err(|e| e.to_string())
}

fn number(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        String::new()
    }
}

pub(crate) fn wavelet_csv(map: &rexafs::WaveletMap, writer: impl Write) -> Result<(), String> {
    let mut csv = csv::Writer::from_writer(writer);
    csv.write_record([
        "k (Å⁻¹)",
        "R (Å)",
        "real",
        "imaginary",
        "magnitude",
        "measured_support",
        "k_weight",
    ])
    .map_err(|e| e.to_string())?;
    for (row, r) in map.r().iter().enumerate() {
        for (col, k) in map.k().iter().enumerate() {
            let index = row * map.k().len() + col;
            let re = map.real()[index];
            let im = map.imaginary()[index];
            csv.write_record([
                number(*k),
                number(*r),
                number(re),
                number(im),
                number(re.hypot(im)),
                map.support()[col].to_string(),
                map.settings().kweight.to_string(),
            ])
            .map_err(|e| e.to_string())?;
        }
    }
    csv.flush().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn experimental_wavelet_csv_keeps_native_complex_values_and_grid_orientation() {
        use rexafs::io::*;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../rexafs/tests/fixtures/xas/samples/aps/13-id-c/xasdatalibrary/cu_metal_rt.xdi",
        );
        let doc = parse_measurement(&std::fs::read(path).unwrap()).unwrap();
        let scan = &doc.scans[0];
        let signal = scan
            .signals
            .iter()
            .find(|s| s.mapping.signal == SignalConversion::Direct { column: 3 })
            .unwrap();
        let (energy, mu) = scan.arrays(Some(&signal.mapping)).unwrap();
        let sp = crate::params::prepare_arrays(
            energy,
            mu,
            &Default::default(),
            crate::params::RequiredStage::Background,
        )
        .unwrap();
        let map = sp
            .wavelet(&rexafs::Wavelet::new(2. ..=8.).rmax(3.).rstep(0.25))
            .unwrap();
        let mut bytes = Vec::new();
        wavelet_csv(&map, &mut bytes).unwrap();
        let rows: Vec<_> = csv::Reader::from_reader(bytes.as_slice())
            .records()
            .map(Result::unwrap)
            .collect();
        assert_eq!(rows.len(), map.k().len() * map.r().len());
        for index in [0, map.k().len(), 7 * map.k().len() + 55, rows.len() - 1] {
            let row = &rows[index];
            assert_eq!(
                row[0].parse::<f64>().unwrap(),
                map.k()[index % map.k().len()]
            );
            assert_eq!(
                row[1].parse::<f64>().unwrap(),
                map.r()[index / map.k().len()]
            );
            assert_eq!(row[2].parse::<f64>().unwrap(), map.real()[index]);
            assert_eq!(row[3].parse::<f64>().unwrap(), map.imaginary()[index]);
            assert_eq!(
                row[4].parse::<f64>().unwrap(),
                map.real()[index].hypot(map.imaginary()[index])
            );
            assert_eq!(
                row[5].parse::<bool>().unwrap(),
                map.support()[index % map.k().len()]
            );
        }
    }

    #[test]
    fn comparison_export_retains_both_independent_grids_labels_and_gaps() {
        let curves = vec![
            Curve {
                label: "Cu, foil\n10 K".into(),
                x_label: "Energy (eV)".into(),
                y_label: "flat μ(E)".into(),
                x: vec![8970., 8971., 8972.],
                y: vec![0.1, f64::NAN, 1.2],
            },
            Curve {
                label: "Cu oxide".into(),
                x_label: "Energy (eV)".into(),
                y_label: "flat μ(E)".into(),
                x: vec![8970.5, 8973.],
                y: vec![0.2, 0.9],
            },
        ];
        let mut bytes = Vec::new();
        curves_csv(&curves, &mut bytes).unwrap();
        let rows: Vec<_> = csv::Reader::from_reader(bytes.as_slice())
            .records()
            .map(Result::unwrap)
            .collect();
        assert_eq!(rows.len(), 5);
        assert_eq!(&rows[0][0], "1");
        assert_eq!(&rows[4][0], "2");
        assert_eq!(&rows[0][1], "Cu, foil\n10 K");
        assert_eq!(&rows[1][6], "");
        assert_eq!(&rows[3][5], "8970.5");
        assert_eq!(&rows[4][1], "Cu oxide");
        assert_eq!(&rows[4][6], "0.9");
    }
}
