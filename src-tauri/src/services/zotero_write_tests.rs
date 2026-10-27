use super::*;
use crate::services::zotero_write_model::WriteGeometry;
#[test]
fn unsupported_clients_and_cancelled_reads_never_become_success() {
    assert_eq!(
        read_failure("Zotero查询已取消。".into()).code,
        "ZOTERO_WRITE_CANCELLED"
    );
    assert_eq!(
        read_failure("Zotero实例已经切换。".into()).code,
        "ZOTERO_WRITE_SOURCE_MISMATCH"
    );
    assert!(!unknown("ANN00001").verified);
    let value = serde_json::to_value(unknown("ANN00001")).unwrap();
    assert!(value.get("key").is_none());
    assert_eq!(value["annotationKey"], "ANN00001");
}
#[test]
#[ignore = "requires explicit RESEARCHCLIP_REAL_ZOTERO_TEST_ROOT for a marked independent live Zotero profile"]
fn real_marked_zotero_official_http_roundtrip_and_permissions() {
    // No default profile, SQLite access, authentication UI automation, or product key injection.
    let root = std::path::PathBuf::from(
        std::env::var_os("RESEARCHCLIP_REAL_ZOTERO_TEST_ROOT")
            .expect("explicit owned profile required"),
    )
    .canonicalize()
    .unwrap();
    let artifacts = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("artifacts")
        .canonicalize()
        .unwrap();
    assert_eq!(root.parent(), Some(artifacts.as_path()));
    assert_eq!(root.file_name().unwrap(), "zotero-write-v426-b");
    let marker: Value =
        serde_json::from_slice(&std::fs::read(root.join("fixture-marker.json")).unwrap()).unwrap();
    assert_eq!(
        std::path::PathBuf::from(marker["profile"].as_str().unwrap())
            .canonicalize()
            .unwrap(),
        root.join("profile").canonicalize().unwrap()
    );
    assert_eq!(
        std::path::PathBuf::from(marker["data"].as_str().unwrap())
            .canonicalize()
            .unwrap(),
        root.join("data").canonicalize().unwrap()
    );
    let original = std::path::PathBuf::from(marker["pdf"].as_str().unwrap())
        .canonicalize()
        .unwrap();
    assert_eq!(original.parent(), Some(root.as_path()));
    let bytes = std::fs::read(&original).unwrap();
    let before = Sha256::digest(&bytes);
    let client = ZoteroClient::at("http://127.0.0.1:32122/api/").unwrap();
    let cancel = AtomicBool::new(false);
    let health = client.health(&cancel, Instant::now()).unwrap();
    let expected: Value =
        serde_json::from_slice(&std::fs::read(root.join("api-instance.json")).unwrap()).unwrap();
    assert_eq!(expected["serverID"].as_str().unwrap(), health.server_id);
    assert_eq!(health.product_version.as_deref(), Some("10.0.6"));
    let source = AnnotationSource {
        instance_id: health.instance_id.clone(),
        library_type: "user".into(),
        library_id: 0,
        item_key: "ABCDEFGH".into(),
        attachment_key: "BCDEFGHJ".into(),
    };
    let capabilities = client
        .annotation_write_capabilities(&source, &original, &cancel)
        .unwrap();
    assert_eq!(
        capabilities.supported_types,
        vec!["highlight", "underline", "note"]
    );
    let keys: Value =
        serde_json::from_slice(&std::fs::read(root.join("profile/localAPIKeys.json")).unwrap())
            .unwrap();
    let secret = keys["keys"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["appName"] == "ResearchClip isolated QA key fixture")
        .unwrap()["key"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(secret.len(), 32);
    let grant = LocalWriteGrant {
        source: source.clone(),
        server_id: health.server_id.clone(),
        key: secret,
        remembered: true,
    };
    let mut bad = grant.clone();
    bad.key = "x".repeat(32);
    for kind in WRITE_TYPES {
        let geometry = WriteGeometry {
            page_index: 0,
            view: [0.0, 0.0, 600.0, 800.0],
            user_unit: 1.0,
            rectangles: vec![[100.0, 150.0, 200.0, 100.0]],
        };
        let text = if kind == "note" {
            ""
        } else {
            "Rust真实校准选文（虚构）"
        };
        let intent = client
            .prepare_annotation_write(
                &source,
                &original,
                &AnnotationWriteInput::Create {
                    annotation_type: kind.into(),
                    color: "#FFD400".into(),
                    text: text.into(),
                    comment: "Rust独立创建".into(),
                    geometry,
                },
                &cancel,
            )
            .unwrap();
        assert_eq!(
            client
                .perform_annotation_write(&intent, &bad, &original, &cancel)
                .err()
                .unwrap()
                .code,
            "ZOTERO_WRITE_AUTH_REQUIRED"
        );
        let created = client
            .perform_annotation_write(&intent, &grant, &original, &cancel)
            .unwrap();
        assert!(created.verified);
        let annotation = created.annotation.unwrap();
        assert_eq!(
            annotation.position.as_ref().unwrap().rects,
            vec![[100.0, 550.0, 300.0, 650.0]]
        );
        let update = client
            .prepare_annotation_write(
                &source,
                &original,
                &AnnotationWriteInput::Update {
                    annotation_key: annotation.identity.annotation_key.clone(),
                    expected_version: annotation.version,
                    annotation_type: kind.into(),
                    color: "#2ea8e5".into(),
                    text: text.into(),
                    comment: "Rust已改文字颜色".into(),
                },
                &cancel,
            )
            .unwrap();
        let updated = client
            .perform_annotation_write(&update, &grant, &original, &cancel)
            .unwrap();
        assert!(updated.verified);
        let after = updated.annotation.unwrap();
        assert!(after.version > annotation.version);
        assert_eq!(
            client
                .perform_annotation_write(&update, &grant, &original, &cancel)
                .err()
                .unwrap()
                .code,
            "ZOTERO_WRITE_CONFLICT"
        );
        let delete = client
            .prepare_annotation_write(
                &source,
                &original,
                &AnnotationWriteInput::Delete {
                    annotation_key: after.identity.annotation_key.clone(),
                    expected_version: after.version,
                },
                &cancel,
            )
            .unwrap();
        let removed = client
            .perform_annotation_write(&delete, &grant, &original, &cancel)
            .unwrap();
        assert!(removed.verified && removed.annotation.is_none());
        assert!(
            client
                .verify_annotation_write(&delete, &original, &cancel)
                .unwrap()
                .verified
        );
    }
    // Two independent native clients race from the SAME base. All data belongs to this marked fixture.
    let client_b = ZoteroClient::at("http://127.0.0.1:32122/api/").unwrap();
    let fixed_key = crate::services::zotero_write_model::new_foreign_annotation_key().unwrap();
    let create = AnnotationWriteInput::Create {
        annotation_type: "highlight".into(),
        color: "#ffd400".into(),
        text: "两个客户端的虚构选文".into(),
        comment: "固定key的幂等创建".into(),
        geometry: WriteGeometry {
            page_index: 0,
            view: [0., 0., 600., 800.],
            user_unit: 1.,
            rectangles: vec![[10., 20., 100., 20.]],
        },
    };
    let a = client
        .prepare_annotation_write_reserved(&source, &original, &create, Some(&fixed_key), &cancel)
        .unwrap();
    let initial = client
        .perform_annotation_write(&a, &grant, &original, &cancel)
        .unwrap()
        .annotation
        .unwrap();
    assert_eq!(initial.identity.annotation_key, fixed_key);
    // Restart/retry must READ the same key, rather than create a fresh one.
    assert!(
        client_b
            .verify_annotation_write(&a, &original, &cancel)
            .unwrap()
            .verified
    );
    assert_eq!(
        client_b
            .prepare_annotation_write_reserved(
                &source,
                &original,
                &create,
                Some(&fixed_key),
                &cancel
            )
            .err()
            .unwrap()
            .code,
        "ZOTERO_WRITE_CONFLICT"
    );
    let edit = |version, comment: &str| AnnotationWriteInput::Update {
        annotation_key: fixed_key.clone(),
        expected_version: version,
        annotation_type: "highlight".into(),
        color: "#34a853".into(),
        text: "两个客户端的虚构选文".into(),
        comment: comment.into(),
    };
    let a_edit = client
        .prepare_annotation_write(
            &source,
            &original,
            &edit(initial.version, "客户端A"),
            &cancel,
        )
        .unwrap();
    let b_edit = client_b
        .prepare_annotation_write(
            &source,
            &original,
            &edit(initial.version, "客户端B待处理"),
            &cancel,
        )
        .unwrap();
    let after_a = client
        .perform_annotation_write(&a_edit, &grant, &original, &cancel)
        .unwrap()
        .annotation
        .unwrap();
    assert_eq!(
        client_b
            .perform_annotation_write(&b_edit, &grant, &original, &cancel)
            .err()
            .unwrap()
            .code,
        "ZOTERO_WRITE_CONFLICT"
    );
    assert_eq!(
        client_b
            .read_annotation_for_sync(&source, &original, &fixed_key, &cancel)
            .unwrap()
            .unwrap()
            .comment,
        "客户端A"
    );
    // Delete-versus-update race: update wins, stale deletion must not erase it.
    let stale_delete = client_b
        .prepare_annotation_write(
            &source,
            &original,
            &AnnotationWriteInput::Delete {
                annotation_key: fixed_key.clone(),
                expected_version: after_a.version,
            },
            &cancel,
        )
        .unwrap();
    let newer = client
        .prepare_annotation_write(
            &source,
            &original,
            &edit(after_a.version, "保留最新修改"),
            &cancel,
        )
        .unwrap();
    let after_new = client
        .perform_annotation_write(&newer, &grant, &original, &cancel)
        .unwrap()
        .annotation
        .unwrap();
    assert_eq!(
        client_b
            .perform_annotation_write(&stale_delete, &grant, &original, &cancel)
            .err()
            .unwrap()
            .code,
        "ZOTERO_WRITE_CONFLICT"
    );
    // Deletion wins the next race; stale update must not resurrect a deleted annotation.
    let stale_update = client_b
        .prepare_annotation_write(
            &source,
            &original,
            &edit(after_new.version, "删除后的过期修改"),
            &cancel,
        )
        .unwrap();
    let cleanup = client
        .prepare_annotation_write(
            &source,
            &original,
            &AnnotationWriteInput::Delete {
                annotation_key: fixed_key.clone(),
                expected_version: after_new.version,
            },
            &cancel,
        )
        .unwrap();
    assert!(
        client
            .perform_annotation_write(&cleanup, &grant, &original, &cancel)
            .unwrap()
            .verified
    );
    assert_eq!(
        client_b
            .perform_annotation_write(&stale_update, &grant, &original, &cancel)
            .err()
            .unwrap()
            .code,
        "ZOTERO_WRITE_NOT_FOUND"
    );
    assert!(
        client_b
            .read_annotation_for_sync(&source, &original, &fixed_key, &cancel)
            .unwrap()
            .is_none()
    );
    assert!(
        client_b
            .verify_annotation_write(&cleanup, &original, &cancel)
            .unwrap()
            .verified
    );
    assert_eq!(Sha256::digest(std::fs::read(original).unwrap()), before);
}

// HTTP fixtures below test protocol/error boundaries only, never a real authorization GUI.
fn wire_fixture(
    status: u16,
    body: &'static str,
    headers: &'static str,
) -> (ZoteroClient, std::thread::JoinHandle<Vec<u8>>) {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client =
        ZoteroClient::at(&format!("http://{}/api/", listener.local_addr().unwrap())).unwrap();
    let worker = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut raw = vec![];
        let mut chunk = [0; 4096];
        loop {
            let n = socket.read(&mut chunk).unwrap();
            assert!(n > 0);
            raw.extend_from_slice(&chunk[..n]);
            assert!(raw.len() < 600_000);
            if let Some(split) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                let h = String::from_utf8_lossy(&raw[..split]);
                let size = h
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if raw.len() >= split + 4 + size {
                    break;
                }
            }
        }
        socket.write_all(format!("HTTP/1.1 {status} QA\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).unwrap();
        raw
    });
    (client, worker)
}
#[test]
fn mutation_http_errors_are_explicit_even_when_zotero_early_denials_omit_identity_headers() {
    for (status, expected) in [
        (401, "AUTH_REQUIRED"),
        (403, "FORBIDDEN"),
        (412, "CONFLICT"),
        (428, "PRECONDITION"),
        (429, "RATE_LIMIT"),
    ] {
        let (c, t) = wire_fixture(status, "denied", "");
        let e = c
            .annotation_write_request(
                "DELETE",
                "users/0/items/ANN00001",
                None,
                Some(3),
                Some(&"k".repeat(32)),
                "FixtureAnnotations426",
                false,
            )
            .err()
            .unwrap();
        assert_eq!(e.code, format!("ZOTERO_WRITE_{expected}"));
        let raw = t.join().unwrap();
        let text = String::from_utf8(raw).unwrap();
        assert!(text.starts_with("DELETE /api/users/0/items/ANN00001 HTTP/1.1"));
        assert!(
            text.to_lowercase()
                .contains("if-unmodified-since-version: 3")
        );
        assert!(!text.lines().next().unwrap().contains("key="));
    }
}
#[test]
fn successful_mutation_requires_identity_headers_and_json_orders_annotation_type_first() {
    let (c, t) = wire_fixture(
        200,
        "{}",
        "Zotero-Server-ID: FixtureAnnotations426\r\nZotero-API-Version: 3\r\n",
    );
    let data = json!([{"annotationType":"highlight","annotationColor":"#ffd400","itemType":"annotation","key":"ANN00001","version":0}]);
    c.annotation_write_request(
        "POST",
        "users/0/items",
        Some(&data),
        None,
        Some(&"k".repeat(32)),
        "FixtureAnnotations426",
        false,
    )
    .unwrap();
    let raw = t.join().unwrap();
    let text = String::from_utf8(raw).unwrap();
    assert!(
        text.split("\r\n\r\n")
            .nth(1)
            .unwrap()
            .starts_with("[{\"annotationType\":\"highlight\",")
    );
    let (c, t) = wire_fixture(204, "", "");
    assert_eq!(
        c.annotation_write_request(
            "DELETE",
            "users/0/items/ANN00001",
            None,
            Some(1),
            None,
            "FixtureAnnotations426",
            false
        )
        .err()
        .unwrap()
        .code,
        "ZOTERO_WRITE_SOURCE_MISMATCH"
    );
    t.join().unwrap();
}
#[test]
fn authorization_request_is_named_server_bound_and_never_contains_a_stored_key() {
    let (c, t) = wire_fixture(
        200,
        "{\"key\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"remember\":false}",
        "Zotero-Server-ID: FixtureAnnotations426\r\nZotero-API-Version: 3\r\n",
    );
    let (_, value) = c
        .annotation_write_request(
            "POST",
            "local/authorize",
            Some(&json!({"appName":"ResearchClip"})),
            None,
            None,
            "FixtureAnnotations426",
            true,
        )
        .unwrap();
    assert_eq!(value.unwrap()["remember"], false);
    let text = String::from_utf8(t.join().unwrap()).unwrap();
    assert!(text.starts_with("POST /api/local/authorize "));
    assert!(!text.to_lowercase().contains("zotero-api-key"));
    assert!(text.contains("{\"appName\":\"ResearchClip\"}"));
}
#[test]
fn ambiguous_commit_and_per_item_failure_are_never_reported_as_verified() {
    let (c, t) = wire_fixture(
        503,
        "unavailable",
        "Zotero-Server-ID: FixtureAnnotations426\r\nZotero-API-Version: 3\r\n",
    );
    assert_eq!(
        c.annotation_write_request(
            "PATCH",
            "users/0/items/ANN00001",
            Some(&json!({"annotationComment":"draft"})),
            Some(1),
            None,
            "FixtureAnnotations426",
            false
        )
        .err()
        .unwrap()
        .code,
        "ZOTERO_WRITE_UNKNOWN"
    );
    t.join().unwrap();
    assert!(!unknown("ANN00001").verified);
}
#[test]
fn native_grants_are_source_bound_single_use_and_pending_blocks_replays() {
    use crate::services::zotero_session::ZoteroSessions;
    let source = AnnotationSource {
        instance_id: "a".repeat(32),
        library_type: "user".into(),
        library_id: 0,
        item_key: "PARENT01".into(),
        attachment_key: "ATTACH01".into(),
    };
    let s = ZoteroSessions::default();
    let cancel = AtomicBool::new(false);
    let grant = LocalWriteGrant {
        source: source.clone(),
        server_id: "FixtureAnnotations426".into(),
        key: "k".repeat(32),
        remembered: false,
    };
    s.save_write_grant("owner", &source, "hash", grant, &cancel)
        .unwrap();
    let intent = WriteIntent {
        source: source.clone(),
        key: "ANN00001".into(),
        method: "DELETE",
        body: None,
        version: Some(3),
        expected: None,
    };
    assert!(
        s.start_write("other", "hash", intent.clone(), &cancel)
            .is_err()
    );
    assert!(
        s.start_write("owner", "wrong", intent.clone(), &cancel)
            .is_err()
    );
    s.start_write("owner", "hash", intent.clone(), &cancel)
        .unwrap();
    assert_eq!(
        s.write_authorized("owner", &source, "hash").unwrap(),
        (false, false, true)
    );
    assert!(s.start_write("owner", "hash", intent, &cancel).is_err());
    let mut wrong = source.clone();
    wrong.library_type = "group".into();
    wrong.library_id = 42;
    assert!(s.pending_write("owner", &wrong, "hash").is_err());
    s.finish_write("owner", "OTHER001").unwrap();
    assert!(s.pending_write("owner", &source, "hash").is_ok());
    s.finish_write("owner", "ANN00001").unwrap();
    assert!(s.pending_write("owner", &source, "hash").is_err());
    s.cancel_pdf("owner").unwrap();
    assert_eq!(
        s.write_authorized("owner", &source, "hash").unwrap(),
        (false, false, false)
    );
}
