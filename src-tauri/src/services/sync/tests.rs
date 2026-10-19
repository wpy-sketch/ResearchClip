use super::{
    engine::{self, Store},
    model::*,
    transport::Dav,
};
use crate::{
    capture::RawFrame,
    services::screen_capture::encode_png,
    storage::{Storage, library::ScreenshotPatch},
};
use serde_json::json;
use std::{
    fs,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
fn doc(text: &str) -> serde_json::Value {
    json!({"schemaVersion":1,"image":{"widthPx":32,"heightPx":24},"objects":[{"id":"text","type":"text","x":1,"y":1,"text":text,"fontSize":12,"color":"#abcdef","strokeWidth":2}]})
}
fn png() -> Vec<u8> {
    encode_png(RawFrame {
        width_px: 32,
        height_px: 24,
        rgba: [10, 20, 30, 128].repeat(768),
    })
    .unwrap()
}
fn config() -> Config {
    Config {
        server: "https://test.invalid/".into(),
        folder: "researchclip-sync-v2".into(),
        projects: vec![],
        unclassified: true,
    }
}
fn server(store: &impl Store) -> (Dav, String) {
    let base =
        std::env::var("RESEARCHCLIP_QA_DAV_BASE").expect("start independent loopback test server");
    assert!(base.starts_with("http://127.0.0.1:"));
    let id = store.access(|s| s.sync_new_id()).unwrap();
    let dav = Dav::local(
        &format!("{base}researchclip-{id}/"),
        Arc::new(AtomicBool::new(false)),
    );
    dav.probe(&id).unwrap();
    let w = dav.initialize(&id).unwrap();
    (dav, w)
}
fn run(store: &impl Store, dav: &Dav, w: &str, device: &str) -> engine::Progress {
    engine::run(store, dav, &config(), w, device, |_| {}).unwrap()
}
fn rename(store: &impl Store, id: &str, title: &str) {
    store
        .access(|s| {
            let old = s.get_screenshot(id)?;
            s.update_screenshot(
                id,
                old.revision,
                ScreenshotPatch {
                    title: Some(title.into()),
                    is_favorite: None,
                    annotation_data: None,
                },
            )?;
            Ok(())
        })
        .unwrap();
}

#[test]
fn cache_authorization_paths_and_future_annotations_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let mut s = Storage::open(root.path().into()).unwrap();
    let shot = s.create_screenshot("测试", doc("数学"), &png()).unwrap();
    let snap = s
        .sync_snapshot(&format!("screenshot/{}", shot.id))
        .unwrap()
        .unwrap();
    snap.validate().unwrap();
    let b = s.sync_cache(b"immutable").unwrap();
    assert_eq!(s.sync_blob(&b.hash, 100).unwrap(), b"immutable");
    assert!(s.sync_blob("../outside", 100).is_err());
    let mut bad = snap.clone();
    if let Body::Screenshot(ref mut body) = bad.body {
        body.history[0].schema = 99;
    }
    assert!(bad.validate().is_err());
    let path = root.path().join("科研.md");
    fs::write(&path, "# 未授权").unwrap();
    let indexed = s.index_document(&path, None).unwrap();
    assert!(
        s.sync_snapshot(&format!("document/{}", indexed.id))
            .is_err()
    );
    assert_eq!(s.sync_authorize_documents(root.path()).unwrap(), 1);
    assert!(
        s.sync_snapshot(&format!("document/{}", indexed.id))
            .unwrap()
            .is_some()
    );
    let mut document = s
        .sync_snapshot(&format!("document/{}", indexed.id))
        .unwrap()
        .unwrap();
    if let Body::Document(ref mut d) = document.body {
        d.name = "no-extension".into();
    }
    assert!(document.validate().is_err());
    assert!(crate::storage::sync::document_assets("![](assets/../../secret.png)").is_err());
    assert_eq!(s.screenshot_image(&shot.id, false).unwrap(), png());
}

#[test]
#[ignore = "requires independent loopback WebDAV with content ETags and atomic conditional writes"]
fn webdav_initial_single_side_retry_reopen_and_scope() {
    let ar = tempfile::tempdir().unwrap();
    let br = tempfile::tempdir().unwrap();
    let a = Mutex::new(Storage::open(ar.path().into()).unwrap());
    let b = Mutex::new(Storage::open(br.path().into()).unwrap());
    let (dav, w) = server(&a);
    let shot = a
        .access(|s| {
            let shot = s.create_screenshot("中文 English", doc("A"), &png())?;
            s.save_note(
                &shot.id,
                None,
                None,
                "研究笔记 $V=x^TPx$",
                Some("https://example.org/paper"),
            )?;
            let tag = s.save_tag(None, "控制")?;
            s.set_screenshot_tags(&shot.id, shot.revision, &[tag.id])?;
            s.get_screenshot(&shot.id)
        })
        .unwrap();
    assert_eq!(run(&a, &dav, &w, &"a".repeat(32)).uploaded, 1);
    assert_eq!(run(&b, &dav, &w, &"b".repeat(32)).downloaded, 1);
    b.access(|s| {
        assert_eq!(s.get_screenshot(&shot.id)?.title, "中文 English");
        assert_eq!(s.screenshot_image(&shot.id, false)?, png());
        assert_eq!(s.list_notes(&shot.id)?[0].content, "研究笔记 $V=x^TPx$");
        assert_eq!(s.screenshot_tags(&shot.id)?.len(), 1);
        Ok(())
    })
    .unwrap();
    rename(&a, &shot.id, "单端修改");
    run(&a, &dav, &w, &"a".repeat(32));
    run(&b, &dav, &w, &"b".repeat(32));
    assert_eq!(
        b.access(|s| Ok(s.get_screenshot(&shot.id)?.title)).unwrap(),
        "单端修改"
    );
    assert_eq!(run(&b, &dav, &w, &"b".repeat(32)).uploaded, 0);
    let mut none = config();
    none.unclassified = false;
    assert_eq!(
        engine::run(&b, &dav, &none, &w, &"b".repeat(32), |_| {})
            .unwrap()
            .total,
        0
    );
    a.access(|s| {
        s.save_project(None, "只同步此项目")?;
        let project = s.projects()?.remove(0);
        let shot = s.create_screenshot("项目图", doc("项目"), &png())?;
        s.batch_screenshots(
            &[crate::storage::projects::SelectedScreenshot {
                id: shot.id,
                revision: shot.revision,
            }],
            crate::storage::projects::BatchAction::Move {
                project_id: Some(project.id.clone()),
            },
        )?;
        let path = ar.path().join("project.md");
        fs::write(&path, "# 项目文档")?;
        let d = s.index_document(&path, None)?;
        s.assign_document(&d.id, Some(&project.id))?;
        s.sync_authorize_documents(ar.path())?;
        Ok(())
    })
    .unwrap();
    let project = a.access(|s| Ok(s.projects()?.remove(0))).unwrap();
    let selected = Config {
        projects: vec![project.id.clone()],
        unclassified: false,
        ..config()
    };
    assert_eq!(
        engine::run(&a, &dav, &selected, &w, &"a".repeat(32), |_| {})
            .unwrap()
            .uploaded,
        3
    );
    let received = engine::run(&b, &dav, &selected, &w, &"b".repeat(32), |_| {}).unwrap();
    assert_eq!(received.total, 3);
    // The document installs its project dependency first; the identical project
    // resource then needs only a baseline ACK, not a second apply.
    assert_eq!(received.downloaded, 2);
    assert_eq!(received.conflicts, 0);
    b.access(|s| {
        let p = s.projects()?.remove(0);
        assert_eq!(p.id, project.id);
        assert_eq!((p.image_count, p.document_count), (1, 1));
        assert_eq!(s.list_notes(&shot.id)?[0].content, "研究笔记 $V=x^TPx$");
        Ok(())
    })
    .unwrap();
    drop(b);
    let b = Mutex::new(Storage::open(br.path().into()).unwrap());
    assert_eq!(run(&b, &dav, &w, &"b".repeat(32)).downloaded, 0);
    assert_eq!(
        b.access(|s| s.screenshot_image(&shot.id, false)).unwrap(),
        png()
    );
}

#[test]
#[ignore = "requires independent loopback WebDAV"]
fn webdav_dual_edit_keep_both_and_delete_modify_confirmation() {
    let ar = tempfile::tempdir().unwrap();
    let br = tempfile::tempdir().unwrap();
    let a = Mutex::new(Storage::open(ar.path().into()).unwrap());
    let b = Mutex::new(Storage::open(br.path().into()).unwrap());
    let (dav, w) = server(&a);
    let shot = a
        .access(|s| s.create_screenshot("初始", doc("A"), &png()))
        .unwrap();
    run(&a, &dav, &w, &"a".repeat(32));
    run(&b, &dav, &w, &"b".repeat(32));
    rename(&a, &shot.id, "设备A");
    rename(&b, &shot.id, "设备B");
    run(&a, &dav, &w, &"a".repeat(32));
    assert_eq!(run(&b, &dav, &w, &"b".repeat(32)).conflicts, 1);
    assert_eq!(
        b.access(|s| Ok(s.get_screenshot(&shot.id)?.title)).unwrap(),
        "设备B"
    );
    let conflict = b
        .access(|s| s.sync_conflicts(&config().id()))
        .unwrap()
        .remove(0);
    engine::resolve(&b, &dav, &config(), &w, &"b".repeat(32), &conflict, "both").unwrap();
    run(&b, &dav, &w, &"b".repeat(32));
    run(&a, &dav, &w, &"a".repeat(32));
    assert_eq!(
        a.access(|s| Ok(s.list_screenshots(0, 50, false)?.len()))
            .unwrap(),
        2
    );
    assert_eq!(
        b.access(|s| Ok(s.list_screenshots(0, 50, false)?.len()))
            .unwrap(),
        2
    );
    a.access(|s| {
        let old = s.get_screenshot(&shot.id)?;
        s.delete_screenshot(&shot.id, old.revision)?;
        Ok(())
    })
    .unwrap();
    rename(&b, &shot.id, "删除时仍修改");
    run(&a, &dav, &w, &"a".repeat(32));
    assert_eq!(run(&b, &dav, &w, &"b".repeat(32)).conflicts, 1);
    assert!(b.access(|s| s.get_screenshot(&shot.id)).is_ok());
    let conflict = b
        .access(|s| s.sync_conflicts(&config().id()))
        .unwrap()
        .remove(0);
    engine::resolve(&b, &dav, &config(), &w, &"b".repeat(32), &conflict, "local").unwrap();
    run(&a, &dav, &w, &"a".repeat(32));
    assert_eq!(
        a.access(|s| s.screenshot_image(&shot.id, false)).unwrap(),
        png()
    );
}

#[test]
#[ignore = "requires independent loopback WebDAV"]
fn webdav_history_forks_current_head_and_continued_editing() {
    let ar = tempfile::tempdir().unwrap();
    let br = tempfile::tempdir().unwrap();
    let a = Mutex::new(Storage::open(ar.path().into()).unwrap());
    let b = Mutex::new(Storage::open(br.path().into()).unwrap());
    let (dav, w) = server(&a);
    let shot = a
        .access(|s| s.create_screenshot("标注", doc("initial"), &png()))
        .unwrap();
    run(&a, &dav, &w, &"a".repeat(32));
    run(&b, &dav, &w, &"b".repeat(32));
    for (store, text) in [(&a, "A分支"), (&b, "B分支")] {
        store
            .access(|s| {
                let old = s.get_screenshot(&shot.id)?;
                s.save_annotation_revision(&shot.id, old.revision, doc(text), text)?;
                Ok(())
            })
            .unwrap();
    }
    run(&a, &dav, &w, &"a".repeat(32));
    assert_eq!(run(&b, &dav, &w, &"b".repeat(32)).conflicts, 1);
    let conflict = b
        .access(|s| s.sync_conflicts(&config().id()))
        .unwrap()
        .remove(0);
    engine::resolve(
        &b,
        &dav,
        &config(),
        &w,
        &"b".repeat(32),
        &conflict,
        "remote",
    )
    .unwrap();
    b.access(|s| {
        assert_eq!(s.annotation_history(&shot.id, i64::MAX, 50)?.len(), 3);
        let old = s.get_screenshot(&shot.id)?;
        assert_eq!(old.annotation_data, doc("A分支"));
        let updated =
            s.save_annotation_revision(&shot.id, old.revision, doc("继续编辑"), "continued")?;
        assert_eq!(updated.annotation_version, 4);
        assert_eq!(s.screenshot_image(&shot.id, false)?, png());
        Ok(())
    })
    .unwrap();
    run(&b, &dav, &w, &"b".repeat(32));
    run(&a, &dav, &w, &"a".repeat(32));
    assert_eq!(
        a.access(|s| Ok(s.annotation_history(&shot.id, i64::MAX, 50)?.len()))
            .unwrap(),
        4
    );
    for _ in 0..3 {
        for (store, device) in [(&a, "a".repeat(32)), (&b, "b".repeat(32))] {
            let stats = run(store, &dav, &w, &device);
            assert_eq!(stats.uploaded, 0, "stable topology must not echo histories");
            assert_eq!(stats.conflicts, 0);
        }
    }
}

#[test]
#[ignore = "requires independent loopback WebDAV"]
fn webdav_markdown_assets_same_names_dual_edit_and_safe_paths() {
    let ar = tempfile::tempdir().unwrap();
    let br = tempfile::tempdir().unwrap();
    let a = Mutex::new(Storage::open(ar.path().join("data")).unwrap());
    let b = Mutex::new(Storage::open(br.path().into()).unwrap());
    let (dav, w) = server(&a);
    let source = ar.path().join("notes");
    fs::create_dir_all(source.join("assets")).unwrap();
    let asset = format!("researchclip-{}.png", "f".repeat(32));
    fs::write(source.join("assets").join(&asset), png()).unwrap();
    let text = format!("# 中文\n\n![说明](assets/{asset})\n\n$$V(x)=x^T P x$$\n");
    let path = source.join("notes.md");
    fs::write(&path, &text).unwrap();
    let id = a
        .access(|s| {
            let item = s.index_document(&path, None)?;
            s.sync_authorize_documents(&source)?;
            Ok(item.id)
        })
        .unwrap();
    run(&a, &dav, &w, &"a".repeat(32));
    run(&b, &dav, &w, &"b".repeat(32));
    let bpath = b.access(|s| s.indexed_document_path(&id)).unwrap();
    assert!(bpath.starts_with(br.path().canonicalize().unwrap()));
    assert_eq!(fs::read_to_string(&bpath).unwrap(), text);
    assert_eq!(
        fs::read(bpath.parent().unwrap().join("assets").join(&asset)).unwrap(),
        png()
    );
    fs::write(&path, format!("{text}A change")).unwrap();
    fs::write(&bpath, format!("{text}B change")).unwrap();
    run(&a, &dav, &w, &"a".repeat(32));
    assert_eq!(run(&b, &dav, &w, &"b".repeat(32)).conflicts, 1);
    let conflict = b
        .access(|s| s.sync_conflicts(&config().id()))
        .unwrap()
        .remove(0);
    engine::resolve(&b, &dav, &config(), &w, &"b".repeat(32), &conflict, "both").unwrap();
    assert_eq!(
        b.access(|s| Ok(s.project_documents(None, true)?.len()))
            .unwrap(),
        2
    );
    assert!(fs::read_to_string(&path).unwrap().ends_with("A change"));
    assert!(fs::read_to_string(&bpath).unwrap().ends_with("B change"));
}

#[test]
#[ignore = "requires independent loopback WebDAV"]
fn webdav_lost_ack_restart_cancel_and_corrupt_download() {
    let ar = tempfile::tempdir().unwrap();
    let br = tempfile::tempdir().unwrap();
    let a = Mutex::new(Storage::open(ar.path().into()).unwrap());
    let b = Mutex::new(Storage::open(br.path().into()).unwrap());
    let (dav, w) = server(&a);
    let shot = a
        .access(|s| s.create_screenshot("restart", doc("持久"), &png()))
        .unwrap();
    let key = format!("screenshot/{}", shot.id);
    let digest = a
        .access(|s| {
            let snap = s.sync_snapshot(&key)?.unwrap();
            s.sync_prepare(&config().id(), &w, &"a".repeat(32), snap, None, None)
        })
        .unwrap();
    let m = a.access(|s| s.sync_manifest(&digest, &w)).unwrap();
    for blob in m.snapshot.blobs() {
        dav.upload_object(
            &blob.hash,
            a.access(|s| s.sync_blob(&blob.hash, MAX_BLOB)).unwrap(),
        )
        .unwrap();
    }
    dav.upload_object(
        &digest,
        a.access(|s| s.sync_blob(&digest, MAX_RESOURCE + 8192))
            .unwrap(),
    )
    .unwrap();
    dav.publish(&w, &key, None, &digest, None).unwrap();
    a.access(|s| s.sync_uncertain(&config().id(), &key))
        .unwrap();
    drop(a);
    let a = Mutex::new(Storage::open(ar.path().into()).unwrap());
    run(&a, &dav, &w, &"a".repeat(32));
    assert!(
        a.access(|s| s.sync_pending(&config().id()))
            .unwrap()
            .is_empty()
    );
    run(&b, &dav, &w, &"b".repeat(32));
    assert_eq!(
        b.access(|s| Ok(s.list_screenshots(0, 50, false)?.len()))
            .unwrap(),
        1
    );
    let cancelled = Arc::new(AtomicBool::new(true));
    let d = Dav::local("http://127.0.0.1:1461/unused/", Arc::clone(&cancelled));
    assert!(engine::run(&a, &d, &config(), &w, &"a".repeat(32), |_| {}).is_err());
    cancelled.store(false, Ordering::Release);
    let bad_hash = hash(b"expected valid content");
    dav.mkdir("objects/").unwrap();
    dav.mkdir(&format!("objects/{}/", &bad_hash[..2])).unwrap();
    assert_eq!(
        dav.put_new(
            &format!("objects/{}/{bad_hash}", &bad_hash[..2]),
            b"incomplete".to_vec()
        )
        .unwrap()
        .status,
        201
    );
    assert!(dav.object(&bad_hash, MAX_BLOB).is_err());
    assert!(dav.object(&m.snapshot.blobs()[0].hash, 4).is_err());
}

#[test]
fn interrupted_http_body_is_rejected_without_partial_cache() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        assert!(stream.read(&mut request).unwrap() > 0);
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nETag: \"valid\"\r\nConnection: close\r\n\r\nshort").unwrap();
    });
    let d = Dav::local(
        &format!("http://{address}/"),
        Arc::new(AtomicBool::new(false)),
    );
    assert!(d.object(&hash(b"expected"), MAX_BLOB).is_err());
    server.join().unwrap();
}

#[test]
#[ignore = "requires independent TLS fixture with untrusted self-signed certificate on 1462"]
fn webdav_rejects_untrusted_tls_without_disabling_certificate_validation() {
    std::net::TcpStream::connect("127.0.0.1:1462").expect("start independent TLS fixture first");
    let c = Config {
        server: "https://127.0.0.1:1462/".into(),
        ..config()
    };
    let d = Dav::new(
        &c,
        super::transport::Credentials {
            username: "isolated".into(),
            password: "test-only".into(),
            device: "a".repeat(32),
        },
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    let e = match d.get("workspace.json", 1024) {
        Ok(_) => panic!("self-signed certificate must be rejected"),
        Err(e) => e,
    };
    assert!(matches!(e, crate::storage::StorageError::Invalid(_)));
}

#[test]
fn cancel_during_actual_http_download_preserves_cancellation_reason() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let bytes = vec![123u8; 131072];
    let digest = hash(&bytes);
    let (started_tx, started_rx) = mpsc::channel();
    let (continue_tx, continue_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        assert!(stream.read(&mut request).unwrap() > 0);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        )
        .unwrap();
        stream.write_all(&bytes[..32768]).unwrap();
        started_tx.send(()).unwrap();
        continue_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let _ = stream.write_all(&bytes[32768..]);
    });
    let cancel = Arc::new(AtomicBool::new(false));
    let d = Dav::local(&format!("http://{address}/"), Arc::clone(&cancel));
    let worker = std::thread::spawn(move || d.object(&digest, MAX_BLOB));
    started_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    cancel.store(true, Ordering::Release);
    continue_tx.send(()).unwrap();
    let error = worker.join().unwrap().unwrap_err();
    assert!(matches!(error,crate::storage::StorageError::Invalid(m) if m.contains("停止")));
    server.join().unwrap();
}
