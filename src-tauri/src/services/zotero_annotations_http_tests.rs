use super::*;
use crate::services::zotero_annotations::{AnnotationSource, SOURCE_MISMATCH};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};
struct Fixture {
    client: ZoteroClient,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    _tmp: tempfile::TempDir,
    path: std::path::PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Err(error) = self.worker.take().unwrap().join()
            && !thread::panicking()
        {
            std::panic::resume_unwind(error);
        }
    }
}
fn source() -> AnnotationSource {
    AnnotationSource {
        instance_id: instance_identity("FixtureAnnotations425").unwrap(),
        library_type: "user".into(),
        library_id: 42,
        item_key: "PARENT01".into(),
        attachment_key: "ATTACH01".into(),
    }
}
fn item(n: usize, version: u64) -> Value {
    json!({"key":format!("N{n:07}"),"version":version,"library":{"type":"user","id":42},"data":{"itemType":"annotation","parentItem":"ATTACH01","annotationType":"highlight","annotationColor":"#ffd400","annotationText":"fictional protocol fixture","annotationComment":"","annotationPosition":"{\"pageIndex\":0,\"rects\":[[10,20,100,40]]}"}})
}
fn fixture(changed_version: bool, wrong_library: bool) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("中文 PDF & 练习.pdf");
    std::fs::write(&path, b"%PDF-1.7\nowned fixture").unwrap();
    let file_url = Url::from_file_path(&path).unwrap().to_string();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/api/", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let seen = requests.clone();
    let running = stop.clone();
    let worker = thread::spawn(move || {
        while !running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut socket, _)) => {
                    // Windows accept inherits the listener's nonblocking mode.
                    // A WouldBlock before request bytes is not an empty HTTP request.
                    socket.set_nonblocking(false).unwrap();
                    socket
                        .set_read_timeout(Some(Duration::from_secs(1)))
                        .unwrap();
                    let mut raw = Vec::new();
                    let mut chunk = [0; 4096];
                    while !raw.windows(4).any(|bytes| bytes == b"\r\n\r\n") && raw.len() < 16384 {
                        match socket.read(&mut chunk) {
                            Ok(n) if n > 0 => raw.extend_from_slice(&chunk[..n]),
                            _ => break,
                        }
                    }
                    if raw.is_empty() {
                        continue;
                    }
                    let text = String::from_utf8(raw).unwrap();
                    let first = text.lines().next().unwrap_or("");
                    seen.lock().unwrap().push(text.clone());
                    assert!(first.starts_with("GET "));
                    let request = first.split_whitespace().nth(1).unwrap();
                    if request.split('?').next() != Some("/api/") {
                        assert!(
                            text.to_lowercase()
                                .contains("zotero-server-id: fixtureannotations425")
                        );
                    }
                    let mut version = 10;
                    let body = if request.split('?').next() == Some("/api/") {
                        "{}".into()
                    } else if request.contains("/file/view/url") {
                        file_url.clone()
                    } else if request.contains("/children") {
                        let second = request.contains("start=20");
                        if second && changed_version {
                            version = 11;
                        }
                        let mut batch: Vec<Value> = if second {
                            vec![item(19, 2), item(20, 1)]
                        } else {
                            (0..20).map(|n| item(n, 1)).collect()
                        };
                        if wrong_library {
                            batch[0]["library"]["id"] = json!(43);
                        }
                        serde_json::to_string(&batch).unwrap()
                    } else {
                        json!({"key":"ATTACH01","version":1,"library":{"type":"user","id":42},"data":{"itemType":"attachment","parentItem":"PARENT01"}}).to_string()
                    };
                    let reply = format!(
                        "HTTP/1.1 200 OK\r\nZotero-API-Version: 3\r\nZotero-Schema-Version: 44\r\nZotero-Server-ID: FixtureAnnotations425\r\nX-Zotero-Version: 10.0.6-fixture\r\nLast-Modified-Version: {version}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(reply.as_bytes());
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2))
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    Fixture {
        client: ZoteroClient::at(&base).unwrap(),
        requests,
        stop,
        worker: Some(worker),
        _tmp: tmp,
        path,
    }
}
#[test]
fn children_reader_is_get_only_scoped_paginated_deduplicated_and_preserves_local_versions() {
    let f = fixture(false, false);
    let cache = f
        .client
        .read_annotations(&source(), &f.path, &"a".repeat(64), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(cache.annotations.len(), 21);
    assert_eq!(cache.library_version, Some(10));
    assert_eq!(cache.product_version.as_deref(), Some("10.0.6-fixture"));
    assert_eq!(
        cache
            .annotations
            .iter()
            .find(|a| a.identity.annotation_key == "N0000019")
            .unwrap()
            .version,
        2
    );
    assert!(!cache.truncated);
    let requests = f.requests.lock().unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests.iter().all(|r| r.starts_with("GET ")));
    assert!(
        requests
            .iter()
            .any(|r| r.contains("limit=20") && r.contains("start=20"))
    );
}
#[test]
fn changing_library_version_or_foreign_annotation_refuses_a_mixed_snapshot() {
    let f = fixture(true, false);
    assert!(
        f.client
            .read_annotations(&source(), &f.path, &"a".repeat(64), &AtomicBool::new(false))
            .unwrap_err()
            .contains("版本")
    );
    let f = fixture(false, true);
    assert_eq!(
        f.client
            .read_annotations(&source(), &f.path, &"a".repeat(64), &AtomicBool::new(false))
            .unwrap_err(),
        SOURCE_MISMATCH
    );
}
#[test]
fn cancel_before_network_and_wrong_instance_never_return_successful_data() {
    let f = fixture(false, false);
    assert!(
        f.client
            .read_annotations(&source(), &f.path, &"a".repeat(64), &AtomicBool::new(true))
            .is_err()
    );
    assert!(f.requests.lock().unwrap().is_empty());
    let mut other = source();
    other.instance_id = "b".repeat(32);
    assert_eq!(
        f.client
            .read_annotations(&other, &f.path, &"a".repeat(64), &AtomicBool::new(false))
            .unwrap_err(),
        SOURCE_MISMATCH
    );
}
