//! Synthetic XTUNES encoding, table and project-boundary contracts.
use rexafs::io::parse_measurement;

#[test]
fn xtsd_encoding_empty_tables_and_project_boundaries() {
    let text = "DataFileName=日本語.001\r\n#QD Plot\t2\r\nE\tMu\r\n7101\t2\r\n7100\t1\r\n";
    let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(text);
    assert!(!errors);
    let doc = parse_measurement(&bytes).unwrap();
    assert_eq!(doc.scans[0].label, "日本語.001");
    assert_eq!(doc.scans[0].arrays(None).unwrap().0, vec![7101., 7100.]);
    assert!(doc.warnings.iter().any(|s| s.contains("Shift-JIS")));
    let empty = parse_measurement(
        b"DataFileName=chi\n#BG Plot\t0\nE Mu Back Base\n#Xi Plot\t2\nk Xi\n1 0.1\n2 0.2\n",
    )
    .unwrap();
    assert!(empty.scans[0].arrays(None).is_err());
    assert_eq!(empty.datasets[1].shape, vec![2, 2]);
    let repeated = format!("#XTSP FilePath=same\n{text}\n#XTSP FilePath=same\n{text}");
    let doc = parse_measurement(repeated.as_bytes()).unwrap();
    assert_eq!(doc.scans.len(), 2);
    assert_ne!(doc.scans[0].id, doc.scans[1].id);
    for malformed in [
        "DataFileName=x\n#QD Plot\t3\nE Mu\n1 2\n",
        "DataFileName=x\n#QD Plot\t1\nE Mu\n1 2 3\n",
        "DataFileName=x\n#QD Plot\t1\nE Mu\n1 nope\n",
        "#XTSP FilePath=x\nDataFileName=x\n#QD Plot\t2\nE Mu\n1 2\n#XTSP FilePath=y\nDataFileName=y\n#QD Plot\t1\nE Mu\n2 3\n",
        "DataFileName=x\n#QD Plot\t0\nE Mu\n#CF ImportFile\t1\n",
        "DataFileName=x\n#QD Plot\t18446744073709551615\nE Mu\n",
    ] { assert!(parse_measurement(malformed.as_bytes()).is_err(),"{malformed}"); }
}
