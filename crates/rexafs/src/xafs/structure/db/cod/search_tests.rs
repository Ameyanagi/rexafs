//! Offline HTTP regressions for bounded COD searches.
use super::*;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

fn server(replies: Vec<(String, u16, String)>) -> (Cod, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        for (path, status, body) in replies {
            let deadline = Instant::now() + Duration::from_secs(15);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "request not received: {path}");
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            request.read_line(&mut line).unwrap();
            assert_eq!(line.trim(), format!("GET /{path} HTTP/1.1"));
            loop {
                line.clear();
                request.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
            }
            write!(
                stream,
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    (
        Cod::new(CodConfig {
            base_url,
            timeout_sec: 3,
        }),
        task,
    )
}

fn metadata_path(ids: &[String]) -> String {
    format!("result?format=json&id={}", encode(&ids.join(",")))
}

fn record(id: &str, formula: &str, coordinates: bool) -> Value {
    serde_json::json!({
        "file": id, "formula": formula,
        "flags": if coordinates { "has coordinates" } else { "" }
    })
}

#[test]
fn broad_search_downloads_ids_and_only_two_metadata_batches_for_default_limit() {
    let ids: Vec<_> = (1_000_000..1_036_668).map(|id| id.to_string()).collect();
    let mut replies = vec![("result?format=lst&el1=Cu".into(), 200, ids.join("\n"))];
    for batch in ids[..200].chunks(100) {
        let records: Vec<_> = batch
            .iter()
            .rev()
            .map(|id| record(id, "Cu", true))
            .collect();
        replies.push((
            metadata_path(batch),
            200,
            serde_json::to_string(&records).unwrap(),
        ));
    }
    let (cod, task) = server(replies);
    let hits = cod
        .search(&StructureQuery::default().with_elements(["Cu"]))
        .unwrap();
    task.join().unwrap();
    assert_eq!(
        hits.iter().map(|h| &h.id).collect::<Vec<_>>(),
        ids[..200].iter().collect::<Vec<_>>()
    );
}

#[test]
fn search_continues_after_rejected_records_and_retains_full_element_filters() {
    let ids: Vec<_> = (1_000_001..=1_000_004).map(|id| id.to_string()).collect();
    let (cod, task) = server(vec![
        (
            "result?format=lst&el1=Cu&nel1=O".into(),
            200,
            format!("{}\n1000003\n", ids.join("\r\n")),
        ),
        (
            metadata_path(&ids[..2]),
            200,
            serde_json::json!([record(&ids[0], "Cu", false), record(&ids[1], "Cu O", true)])
                .to_string(),
        ),
        (
            metadata_path(&ids[2..]),
            200,
            serde_json::json!([
                record(&ids[3], "Cu", true),
                record("9999999", "Cu", true),
                record(&ids[2], "Cu", true)
            ])
            .to_string(),
        ),
    ]);
    let query = StructureQuery {
        elements: vec!["Cu".into()],
        exclude: vec!["O".into()],
        limit: 2,
        ..Default::default()
    };
    let hits = cod.search(&query).unwrap();
    task.join().unwrap();
    assert_eq!(
        hits.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
        ["1000003", "1000004"]
    );
}

#[test]
fn failed_later_batch_does_not_return_partial_success() {
    let ids: Vec<_> = (1_000_001..=1_000_003).map(|id| id.to_string()).collect();
    let (cod, task) = server(vec![
        ("result?format=lst&el1=Cu".into(), 200, ids.join("\n")),
        (
            metadata_path(&ids[..2]),
            200,
            serde_json::json!([record(&ids[0], "Cu", true), record(&ids[1], "Cu", false)])
                .to_string(),
        ),
        (metadata_path(&ids[2..]), 503, "Unavailable".into()),
    ]);
    let query = StructureQuery {
        elements: vec!["Cu".into()],
        limit: 2,
        ..Default::default()
    };
    let error = cod.search(&query).unwrap_err();
    task.join().unwrap();
    assert!(error.to_string().contains("503"));
}

#[test]
fn empty_and_invalid_id_lists_do_not_request_metadata() {
    for (body, valid) in [
        ("\n", true),
        ("<html>Unavailable</html>", false),
        ("1000001\nbad", false),
    ] {
        let (cod, task) = server(vec![(
            "result?format=lst&formula=Cu".into(),
            200,
            body.into(),
        )]);
        let result = cod.search(&StructureQuery::text("Cu"));
        task.join().unwrap();
        if valid {
            assert!(result.unwrap().is_empty());
        } else {
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("invalid search ID list"));
        }
    }
}

/// Manual public-server check for the previously oversized Cu search.
#[test]
#[ignore = "requires the public COD server"]
fn live_search_copper() {
    let hits = Cod::default()
        .search(&StructureQuery::default().with_elements(["Cu"]))
        .unwrap();
    assert_eq!(hits.len(), DEFAULT_LIMIT);
    assert!(hits
        .iter()
        .all(|hit| hit.elements.iter().any(|el| el == "Cu")));
}
