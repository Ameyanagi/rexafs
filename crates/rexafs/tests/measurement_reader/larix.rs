//! Synthetic Larix framing, metadata, dtype and allocation-limit contracts.
use rexafs::io::parse_measurement;
use serde_json::Value;

fn session(value: Value) -> Vec<u8> {
    format!("##LARIX: 1.0 Larch Session File\n##<Symbols: count=1>\n<:sample:>\n{value}\n##</Symbols>\n").into_bytes()
}

#[test]
fn rejects_bad_shapes_dtypes_payloads_and_incomplete_framing() {
    let base = serde_json::json!({"__class__":"b64ndarray","__value__": ""});
    for inner in [
        serde_json::json!({"_type_":"b64ndarray","dtype":"<f8","shape":[u64::MAX,2],"value":""}),
        serde_json::json!({"_type_":"b64ndarray","dtype":"<f8","shape":[40_000_000],"value":""}),
        serde_json::json!({"_type_":"b64ndarray","dtype":"=f8","shape":[0],"value":""}),
        serde_json::json!({"_type_":"b64ndarray","dtype":"<f8","shape":[1],"value":"!!!!!!!!!!!!"}),
    ] {
        let mut value = base.clone();
        value["__value__"] = inner.to_string().into();
        assert!(parse_measurement(&session(value)).is_err());
    }
    let valid = session(serde_json::json!({"__class__":"Group"}));
    assert!(parse_measurement(&valid[..valid.len() - 15]).is_err());
    let duplicate = String::from_utf8(valid.clone())
        .unwrap()
        .replace("##</Symbols>", "<:sample:>\n{}\n##</Symbols>");
    assert!(parse_measurement(duplicate.as_bytes())
        .unwrap_err()
        .to_string()
        .contains("duplicate"));
    let mut nested = serde_json::json!(0);
    for _ in 0..70 {
        nested = serde_json::json!({"nested":nested});
    }
    assert!(parse_measurement(&session(nested))
        .unwrap_err()
        .to_string()
        .contains("nesting"));
    let history = String::from_utf8(valid).unwrap().replace("##<Symbols", "##<Session Commands>\nraise RuntimeError('must remain text')\n##</Session Commands>\n##<Symbols");
    assert!(
        parse_measurement(history.as_bytes()).unwrap().metadata["larix.command_history"]
            .contains("must remain text")
    );
}

#[test]
fn duplicate_json_keys_and_invalid_session_encoding_fail_without_losing_values() {
    for raw in [
        r#"{"__class__":"Group","energy":1,"energy":2}"#,
        r#"{"metadata":{"units":"eV","units":"keV"}}"#,
        r#"{"__class__":"b64ndarray","__value__":"{\"_type_\":\"b64ndarray\",\"dtype\":\"<f8\",\"shape\":[0],\"shape\":[1],\"value\":\"\"}"}"#,
    ] {
        let source =
            format!("##LARIX: 1.0\n##<Symbols: count=1>\n<:sample:>\n{raw}\n##</Symbols>\n");
        let error = parse_measurement(source.as_bytes())
            .unwrap_err()
            .to_string();
        assert!(error.contains("duplicate object key"), "{error}");
    }
    let source = session(serde_json::json!({"label":"日本語"}));
    let source = String::from_utf8(source).unwrap();
    let (legacy, _, _) = encoding_rs::SHIFT_JIS.encode(&source);
    assert!(parse_measurement(&legacy)
        .unwrap_err()
        .to_string()
        .contains("UTF-8"));
}
