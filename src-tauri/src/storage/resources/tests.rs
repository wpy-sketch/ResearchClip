use super::*;
use crate::capture::RawFrame;
use crate::services::markdown_document::DocumentFiles;
use crate::services::screen_capture::encode_png;
use rusqlite::OpenFlags;
use sha2::{Digest, Sha256};
fn hash() -> String {
    "b".repeat(64)
}
fn png() -> Vec<u8> {
    encode_png(RawFrame {
        width_px: 2,
        height_px: 2,
        rgba: [20, 40, 80, 255].repeat(4),
    })
    .unwrap()
}
fn shot(s: &mut Storage) -> ResourceKey {
    let v = s
        .create_screenshot(
            "旧截图",
            serde_json::json!({"schemaVersion":1,"image":{"widthPx":2,"heightPx":2},"objects":[]}),
            &png(),
        )
        .unwrap();
    ResourceKey {
        kind: ResourceKind::Screenshot,
        id: v.id,
    }
}
fn pdf(s: &mut Storage) -> (ResourceSnapshot, ResourceSnapshot) {
    let document = s
        .create_research_resource(
            "PDF文档",
            &ResearchData::Document(DocumentResource {
                document_type: DocumentType::Pdf,
                project_id: None,
                relative_path: None,
                code_workspace_id: None,
                content_hash: Some(hash()),
            }),
        )
        .unwrap();
    let attachment = s
        .create_research_resource(
            "PDF附件",
            &ResearchData::PdfAttachment(PdfAttachment {
                document_id: document.key.id.clone(),
                reference_id: None,
                content_hash: hash(),
                byte_size: 100,
            }),
        )
        .unwrap();
    (document, attachment)
}
fn location(id: &str) -> ResearchData {
    ResearchData::PdfLocation(PdfLocation {
        attachment_id: id.into(),
        content_hash: hash(),
        page_index: 0,
        page_label: Some("i".into()),
        page_width: 600.,
        page_height: 800.,
        rectangles: vec![PageRectangle {
            x: 10.,
            y: 20.,
            width: 30.,
            height: 40.,
        }],
        text_anchor: Some(TextAnchor {
            exact: "研究".into(),
            prefix: "前".into(),
            suffix: "后".into(),
        }),
    })
}
fn layer(id: &str) -> ResearchData {
    ResearchData::HandwritingLayer(HandwritingLayer {
        attachment_id: id.into(),
        content_hash: hash(),
        page_index: 0,
        page_width: 600.,
        page_height: 800.,
    })
}
fn stroke(id: &str) -> ResearchData {
    ResearchData::Stroke(Stroke {
        layer_id: id.into(),
        tool: StrokeTool::Pen,
        color: "#123456".into(),
        width: 2.,
        points: vec![
            StrokePoint {
                x: 1.,
                y: 2.,
                elapsed_ms: 0,
                pressure: Some(0.2),
                tilt_x: Some(-30.),
                tilt_y: None,
            },
            StrokePoint {
                x: 2.,
                y: 3.,
                elapsed_ms: 1,
                pressure: None,
                tilt_x: None,
                tilt_y: None,
            },
        ],
    })
}
fn legacy_v11(root: &Path) -> (String, String, Vec<u8>, String) {
    std::fs::create_dir_all(root).unwrap();
    let conn = Connection::open(root.join("researchclip.sqlite3")).unwrap();
    let migrations = [
        include_str!("../../../migrations/001_workspace.sql"),
        include_str!("../../../migrations/002_library.sql"),
        include_str!("../../../migrations/003_projects.sql"),
        include_str!("../../../migrations/004_ocr_index.sql"),
        include_str!("../../../migrations/005_annotation_revisions.sql"),
        include_str!("../../../migrations/006_sync.sql"),
        include_str!("../../../migrations/007_image_features.sql"),
        include_str!("../../../migrations/008_references.sql"),
        include_str!("../../../migrations/009_internal_links.sql"),
        include_str!("../../../migrations/010_annotation_anchors.sql"),
        include_str!("../../../migrations/011_semantic_embeddings.sql"),
    ];
    for (i, sql) in migrations.iter().enumerate() {
        conn.execute_batch(sql).unwrap();
        conn.pragma_update(None, "user_version", i + 1).unwrap();
    }
    let id = "a".repeat(32);
    let doc_id = "c".repeat(32);
    let bytes = png();
    std::fs::create_dir(root.join("originals")).unwrap();
    std::fs::create_dir(root.join("thumbnails")).unwrap();
    std::fs::write(root.join(format!("originals/{id}.png")), &bytes).unwrap();
    std::fs::write(root.join(format!("thumbnails/{id}.png")), &bytes).unwrap();
    let annotation =
        "{\"schemaVersion\":1, \"image\":{\"widthPx\":2,\"heightPx\":2}, \"objects\":[]}";
    conn.execute("INSERT INTO screenshots(id,title,original_image_path,thumbnail_path,annotation_data,width_px,height_px,created_at,updated_at) VALUES(?1,'旧图',?2,?3,?4,2,2,1,1)",params![id,format!("originals/{id}.png"),format!("thumbnails/{id}.png"),annotation]).unwrap();
    conn.execute(
        "INSERT INTO notes VALUES(?1,?2,'原始笔记','https://example.org',1,1,1)",
        params!["d".repeat(32), id],
    )
    .unwrap();
    let text = format!("# 中文记录\n\n[旧图](researchclip://screenshot/{id})\n");
    let path = root.join("旧记录.md");
    std::fs::write(&path, &text).unwrap();
    conn.execute(
        "INSERT INTO markdown_documents VALUES(?1,?2,'旧记录.md',NULL,1,1)",
        params![doc_id, path.to_str().unwrap()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO link_index VALUES(?1,?2,'screenshot',8,40,3,'旧图',1,NULL)",
        params![doc_id, id],
    )
    .unwrap();
    (id, doc_id, bytes, text)
}
#[test]
fn migration_v11_preserves_ids_bytes_notes_links_and_verified_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let (id, doc_id, bytes, text) = legacy_v11(tmp.path());
    let s = Storage::open(tmp.path().into()).unwrap();
    assert_eq!(s.status().unwrap().schema_version, 13);
    assert_eq!(s.screenshot_image(&id, false).unwrap(), bytes);
    assert_eq!(s.get_screenshot(&id).unwrap().title, "旧图");
    assert_eq!(s.list_notes(&id).unwrap()[0].content, "原始笔记");
    assert_eq!(
        s.research_resource(&ResourceKey {
            kind: ResourceKind::Screenshot,
            id: id.clone()
        })
        .unwrap()
        .key
        .id,
        id
    );
    assert_eq!(
        s.research_resource(&ResourceKey {
            kind: ResourceKind::Document,
            id: doc_id.clone()
        })
        .unwrap()
        .metadata["documentType"],
        "markdown"
    );
    assert_eq!(
        std::fs::read_to_string(s.indexed_document_path(&doc_id).unwrap()).unwrap(),
        text
    );
    assert_eq!(
        s.research_relations(
            &ResourceKey {
                kind: ResourceKind::Document,
                id: doc_id.clone()
            },
            RelationDirection::Outgoing,
            0,
            20
        )
        .unwrap()[0]
            .target
            .id,
        id
    );
    let backup = std::fs::read_dir(tmp.path().join("backups"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let b = Connection::open_with_flags(backup, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        b.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        11
    );
    let before: String = b
        .query_row("SELECT annotation_data FROM screenshots", [], |r| r.get(0))
        .unwrap();
    let after: String = s
        .connection
        .query_row("SELECT annotation_data FROM screenshots", [], |r| r.get(0))
        .unwrap();
    assert_eq!(before, after);
    drop(s);
    let s = Storage::open(tmp.path().into()).unwrap();
    assert_eq!(s.screenshot_image(&id, false).unwrap(), bytes);
    assert_eq!(
        s.indexed_document_path(&doc_id).unwrap(),
        tmp.path().join("旧记录.md")
    );
}
#[test]
fn migration_conflict_rolls_back_new_ddl_and_preserves_v11() {
    let tmp = tempfile::tempdir().unwrap();
    let (id, _, bytes, _) = legacy_v11(tmp.path());
    let c = Connection::open(tmp.path().join("researchclip.sqlite3")).unwrap();
    c.execute_batch("CREATE TABLE research_relations(keep TEXT); INSERT INTO research_relations VALUES('keep');").unwrap();
    drop(c);
    assert!(Storage::open(tmp.path().into()).is_err());
    let c = Connection::open(tmp.path().join("researchclip.sqlite3")).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        11
    );
    assert!(c.prepare("SELECT * FROM research_entities").is_err());
    assert_eq!(
        c.query_row("SELECT keep FROM research_relations", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert_eq!(
        std::fs::read(tmp.path().join(format!("originals/{id}.png"))).unwrap(),
        bytes
    );
}
#[test]
fn future_database_is_refused_without_reset() {
    let future = crate::storage::migrations::SCHEMA_VERSION + 1;
    let t = tempfile::tempdir().unwrap();
    let s = Storage::open(t.path().into()).unwrap();
    s.connection
        .pragma_update(None, "user_version", future)
        .unwrap();
    drop(s);
    assert!(matches!(
        Storage::open(t.path().into()),
        Err(StorageError::NewerSchema(v)) if v==future
    ));
    let c = Connection::open(t.path().join("researchclip.sqlite3")).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        future
    );
}
#[test]
fn multiple_screenshots_share_one_pdf_and_query_both_directions_after_restart() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    let a = shot(&mut s);
    let b = shot(&mut s);
    let (_, p) = pdf(&mut s);
    for key in [&a, &b] {
        s.relate_research_resources(key, &p.key, RelationType::Source)
            .unwrap();
        s.relate_research_resources(key, &p.key, RelationType::Source)
            .unwrap();
    }
    drop(s);
    let s = Storage::open(t.path().into()).unwrap();
    let incoming = s
        .research_relations(&p.key, RelationDirection::Incoming, 0, 20)
        .unwrap();
    assert_eq!(incoming.len(), 2);
    assert!(incoming.iter().any(|e| e.source == a));
    assert!(incoming.iter().any(|e| e.source == b));
    assert_eq!(
        s.research_relations(&a, RelationDirection::Outgoing, 0, 20)
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn all_new_domains_persist_and_private_strokes_cannot_bind_standard_annotations() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    let (_, p) = pdf(&mut s);
    let loc = s
        .create_research_resource("位置", &location(&p.key.id))
        .unwrap();
    let ann = s
        .create_research_resource(
            "标准批注",
            &ResearchData::Annotation(Annotation {
                location_id: loc.key.id.clone(),
                annotation_type: StandardAnnotationType::Ink,
                text: "".into(),
                comment: "标准".into(),
                source: AnnotationSource::Zotero,
            }),
        )
        .unwrap();
    let ink = s
        .create_research_resource("私有层", &layer(&p.key.id))
        .unwrap();
    let pen = s
        .create_research_resource("笔画", &stroke(&ink.key.id))
        .unwrap();
    assert!(
        s.create_research_resource("错误", &stroke(&ann.key.id))
            .is_err()
    );
    let workspace = s
        .create_research_resource(
            "代码",
            &ResearchData::CodeWorkspace(CodeWorkspace {
                project_id: None,
                root_path: t.path().to_string_lossy().into(),
            }),
        )
        .unwrap();
    let code = s
        .create_research_resource(
            "源码",
            &ResearchData::Document(DocumentResource {
                document_type: DocumentType::Code,
                project_id: None,
                relative_path: Some("src/研究.py".into()),
                code_workspace_id: Some(workspace.key.id.clone()),
                content_hash: None,
            }),
        )
        .unwrap();
    let agent = s
        .create_research_resource(
            "会话",
            &ResearchData::AgentSession(AgentSession {
                workspace_id: workspace.key.id.clone(),
                provider: AgentProvider::Codex,
                provider_session_id: Some("provider-session".into()),
                phase: AgentPhase::Idle,
            }),
        )
        .unwrap();
    let evidence = s
        .create_research_resource(
            "证据",
            &ResearchData::Evidence(ResearchEvidence {
                claim: "待人工验证的研究结论".into(),
                document_id: Some(code.key.id.clone()),
            }),
        )
        .unwrap();
    s.relate_research_resources(&evidence.key, &p.key, RelationType::Supports)
        .unwrap();
    drop(s);
    let s = Storage::open(t.path().into()).unwrap();
    for key in [
        &loc.key,
        &ann.key,
        &ink.key,
        &pen.key,
        &workspace.key,
        &code.key,
        &agent.key,
        &evidence.key,
    ] {
        assert_eq!(s.research_resource(key).unwrap().format_version, 1);
    }
    let ResearchData::Stroke(data) = s.research_resource(&pen.key).unwrap().data().unwrap() else {
        panic!()
    };
    assert_eq!(data.points[0].pressure, Some(0.2));
    assert_eq!(data.points[1].pressure, None);
}
#[test]
fn zotero_keys_are_scoped_by_instance_and_library_and_cannot_be_reassigned() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    let (_, a) = pdf(&mut s);
    let (_, b) = pdf(&mut s);
    let x = ZoteroKey {
        instance_id: "a".repeat(32),
        library_id: 1,
        item_key: "ABCDEFG1".into(),
    };
    let y = ZoteroKey {
        library_id: 2,
        ..x.clone()
    };
    let z = ZoteroKey {
        instance_id: "c".repeat(32),
        ..x.clone()
    };
    s.bind_zotero_resource(&x, &a.key).unwrap();
    s.bind_zotero_resource(&x, &a.key).unwrap();
    s.bind_zotero_resource(&y, &b.key).unwrap();
    s.bind_zotero_resource(&z, &b.key).unwrap();
    assert!(matches!(
        s.bind_zotero_resource(&x, &b.key),
        Err(StorageError::Conflict)
    ));
    drop(s);
    let s = Storage::open(t.path().into()).unwrap();
    assert_eq!(s.zotero_resource(&x).unwrap(), Some(a.key));
    assert_eq!(s.zotero_resource(&y).unwrap(), Some(b.key.clone()));
    assert_eq!(s.zotero_resource(&z).unwrap(), Some(b.key));
}
#[test]
fn stale_revision_unknown_metadata_and_immutable_pdf_fail_without_overwrite() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    let (_, p) = pdf(&mut s);
    let data = p.data().unwrap();
    let v = s
        .update_research_resource(&p.key, 1, "改名", &data)
        .unwrap();
    assert_eq!(v.revision, Some(2));
    assert!(matches!(
        s.update_research_resource(&p.key, 1, "覆盖", &data),
        Err(StorageError::Conflict)
    ));
    let ResearchData::PdfAttachment(mut changed) = data else {
        panic!()
    };
    changed.content_hash = "c".repeat(64);
    assert!(
        s.update_research_resource(&p.key, 2, "错误", &ResearchData::PdfAttachment(changed))
            .is_err()
    );
    assert_eq!(s.research_resource(&p.key).unwrap().title, "改名");
    s.connection.execute("UPDATE research_entities SET format_version=2 WHERE resource_type='pdfAttachment' AND id=?1",[&p.key.id]).unwrap();
    assert!(s.research_resource(&p.key).is_err());
    assert!(
        s.audit_research_resources(100)
            .unwrap()
            .issues
            .iter()
            .any(|i| i.code == "unsupportedVersion")
    );
}
#[test]
fn bad_geometry_pressure_paths_and_unknown_fields_are_rejected() {
    let mut d = location(&"a".repeat(32));
    let ResearchData::PdfLocation(ref mut loc) = d else {
        panic!()
    };
    loc.rectangles[0].width = 700.;
    assert!(d.validate().is_err());
    let mut d = stroke(&"a".repeat(32));
    let ResearchData::Stroke(ref mut pen) = d else {
        panic!()
    };
    pen.points[0].pressure = Some(1.1);
    assert!(d.validate().is_err());
    for path in [
        "../out.pdf",
        "/abs.pdf",
        "a//b",
        "a\\b",
        "C:/a",
        "a/CON.txt",
        "a/..",
        "a/end. ",
    ] {
        assert!(!relative_path_valid(path), "{path}");
    }
    assert!(relative_path_valid("pdfs/中文/a.pdf"));
    assert!(
        serde_json::from_str::<ResearchData>(
            r#"{"kind":"evidence","value":{"claim":"x","documentId":null,"command":"rm"}}"#
        )
        .is_err()
    );
}
#[test]
fn missing_dependencies_rollback_and_database_relation_guard_rejects_missing_endpoints() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    assert!(
        s.create_research_resource("错误", &location(&"a".repeat(32)))
            .is_err()
    );
    assert!(s.research_resources(None, 0, 100).unwrap().is_empty());
    let a = shot(&mut s);
    let missing = ResourceKey {
        kind: ResourceKind::Document,
        id: "a".repeat(32),
    };
    assert!(
        s.relate_research_resources(&a, &missing, RelationType::Source)
            .is_err()
    );
    assert!(s.connection.execute("INSERT INTO research_relations VALUES('screenshot',?1,'document',?2,'source','manual')",params![a.id,missing.id]).is_err());
}
#[test]
fn pdf_hash_mismatch_and_strokes_outside_page_are_rejected() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    let (_, p) = pdf(&mut s);
    let mut d = location(&p.key.id);
    let ResearchData::PdfLocation(ref mut loc) = d else {
        panic!()
    };
    loc.content_hash = "e".repeat(64);
    assert!(s.create_research_resource("错页", &d).is_err());
    let ink = s.create_research_resource("层", &layer(&p.key.id)).unwrap();
    let mut d = stroke(&ink.key.id);
    let ResearchData::Stroke(ref mut pen) = d else {
        panic!()
    };
    pen.points[0].x = 601.;
    assert!(s.create_research_resource("越界", &d).is_err());
}
#[test]
fn legacy_rename_and_external_move_keep_id_links_and_assets_and_revoke_path_grant() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("workspace");
    let docs = t.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    let mut s = Storage::open(root).unwrap();
    let a = shot(&mut s);
    let name = format!("researchclip-{}.png", "e".repeat(32));
    std::fs::create_dir(docs.join("assets")).unwrap();
    std::fs::write(docs.join("assets").join(&name), png()).unwrap();
    let body = format!(
        "# 中文\n[图](researchclip://screenshot/{})\n![图](assets/{name})",
        a.id
    );
    let old = docs.join("旧.md");
    std::fs::write(&old, &body).unwrap();
    let mut files = DocumentFiles::default();
    let disk = files.open(&old).unwrap();
    let n = u32::from_le_bytes(disk[..4].try_into().unwrap()) as usize;
    let meta: serde_json::Value = serde_json::from_slice(&disk[4..4 + n]).unwrap();
    let token = meta["token"].as_str().unwrap();
    let registered = s.index_document(&old, None).unwrap();
    files.mark_indexed(token, old.canonicalize().unwrap());
    let key = ResourceKey {
        kind: ResourceKind::Document,
        id: registered.id.clone(),
    };
    s.relate_research_resources(&a, &key, RelationType::Related)
        .unwrap();
    files.rename(token, "改名.md").unwrap();
    let (path, previous) = files.index_paths(token).unwrap();
    assert_eq!(
        s.index_document(&path, previous.as_deref()).unwrap().id,
        key.id
    );
    files.release(token);
    s.connection
        .execute(
            "INSERT INTO sync_document_grants VALUES(?1,?2)",
            params![key.id, path.to_string_lossy()],
        )
        .unwrap();
    let hash = format!("{:x}", Sha256::digest(body.as_bytes()));
    let moved = t.path().join("moved");
    std::fs::create_dir(&moved).unwrap();
    let destination = moved.join("改名.md");
    std::fs::rename(&path, &destination).unwrap();
    std::fs::rename(docs.join("assets"), moved.join("assets")).unwrap();
    s.relink_markdown_resource(&key.id, &hash, &destination)
        .unwrap();
    assert_eq!(
        s.research_relations(&a, RelationDirection::Outgoing, 0, 10)
            .unwrap()[0]
            .target,
        key
    );
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM sync_document_grants", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(std::fs::read_to_string(&destination).unwrap(), body);
    assert_eq!(
        std::fs::read(moved.join("assets").join(name)).unwrap(),
        png()
    );
    files.open(&destination).unwrap();
    drop(s);
    let s = Storage::open(t.path().join("workspace")).unwrap();
    assert_eq!(
        s.indexed_document_path(&key.id).unwrap(),
        destination.canonicalize().unwrap()
    );
}
#[test]
fn relink_rejects_wrong_content_unverified_legacy_and_other_registered_target() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().join("db")).unwrap();
    let old = t.path().join("a.md");
    let wrong = t.path().join("b.md");
    std::fs::write(&old, "keep").unwrap();
    std::fs::write(&wrong, "wrong").unwrap();
    let doc = s.index_document(&old, None).unwrap();
    let hash = format!("{:x}", Sha256::digest(b"keep"));
    assert!(matches!(
        s.relink_markdown_resource(&doc.id, &hash, &wrong),
        Err(StorageError::Conflict)
    ));
    assert_eq!(
        s.indexed_document_path(&doc.id).unwrap(),
        old.canonicalize().unwrap()
    );
    std::fs::write(&wrong, "keep").unwrap();
    s.index_document(&wrong, None).unwrap();
    assert!(s.relink_markdown_resource(&doc.id, &hash, &wrong).is_err());
    assert_eq!(std::fs::read_to_string(&old).unwrap(), "keep");
    s.connection
        .execute(
            "DELETE FROM research_document_fingerprints WHERE document_id=?1",
            [&doc.id],
        )
        .unwrap();
    assert!(s.relink_markdown_resource(&doc.id, &hash, &old).is_err());
}
#[test]
fn diagnostics_report_orphans_missing_files_and_links_without_cleanup() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    let a = shot(&mut s);
    let (_, p) = pdf(&mut s);
    s.relate_research_resources(&a, &p.key, RelationType::Source)
        .unwrap();
    let item = s.get_screenshot(&a.id).unwrap();
    s.delete_screenshot(&a.id, item.revision).unwrap();
    let sentinel = t.path().join("unknown.keep");
    std::fs::write(&sentinel, b"do not delete").unwrap();
    let audit = s.audit_research_resources(100).unwrap();
    assert!(
        audit
            .issues
            .iter()
            .any(|i| i.code == "missingRelationEndpoint")
    );
    assert!(audit.issues.iter().any(|i| i.code == "fileNotRegistered"));
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"do not delete");
    assert!(s.audit_research_resources(1).unwrap().truncated);
    assert!(s.audit_research_resources(0).is_err());
}
#[test]
fn listing_and_direction_limits_are_explicit_and_type_identity_does_not_collide() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    let a = shot(&mut s);
    let (d, p) = pdf(&mut s);
    assert!(s.research_resources(None, 0, 101).is_err());
    assert!(
        s.research_relations(&a, RelationDirection::Both, 0, 201)
            .is_err()
    );
    assert_eq!(
        s.research_resources(Some(ResourceKind::PdfAttachment), 0, 100)
            .unwrap()
            .len(),
        1
    );
    assert!(
        s.research_resource(&ResourceKey {
            kind: ResourceKind::Screenshot,
            id: d.key.id
        })
        .is_err()
    );
    assert_eq!(
        s.research_relations(&p.key, RelationDirection::Outgoing, 0, 100)
            .unwrap()[0]
            .origin,
        "model"
    );
}

#[test]
fn code_metadata_retarget_keeps_manual_edges_and_uses_one_markdown_service() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    let workspace_data = ResearchData::CodeWorkspace(CodeWorkspace {
        project_id: None,
        root_path: t.path().to_string_lossy().into(),
    });
    let a = s.create_research_resource("A", &workspace_data).unwrap();
    let b = s.create_research_resource("B", &workspace_data).unwrap();
    let mut data = DocumentResource {
        document_type: DocumentType::Code,
        project_id: None,
        relative_path: Some("src/main.rs".into()),
        code_workspace_id: Some(a.key.id.clone()),
        content_hash: None,
    };
    let doc = s
        .create_research_resource("main", &ResearchData::Document(data.clone()))
        .unwrap();
    s.relate_research_resources(&doc.key, &a.key, RelationType::Related)
        .unwrap();
    data.code_workspace_id = Some(b.key.id.clone());
    s.update_research_resource(
        &doc.key,
        1,
        "main moved",
        &ResearchData::Document(data.clone()),
    )
    .unwrap();
    let edges = s
        .research_relations(&doc.key, RelationDirection::Outgoing, 0, 20)
        .unwrap();
    assert_eq!(edges.len(), 2);
    assert!(
        edges
            .iter()
            .any(|e| e.origin == "model" && e.target == b.key)
    );
    assert!(
        edges
            .iter()
            .any(|e| e.origin == "manual" && e.target == a.key)
    );
    data.relative_path = Some("research.MD".into());
    assert!(
        s.create_research_resource("duplicate markdown pipeline", &ResearchData::Document(data))
            .is_err()
    );
}
#[test]
fn stroke_layer_budget_finite_coordinates_and_sample_order_are_enforced() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    let (_, pdf) = pdf(&mut s);
    let ink = s
        .create_research_resource("ink", &layer(&pdf.key.id))
        .unwrap();
    let ResearchData::Stroke(mut data) = stroke(&ink.key.id) else {
        panic!()
    };
    data.points[0].x = f64::NAN;
    assert!(
        s.create_research_resource("NaN", &ResearchData::Stroke(data.clone()))
            .is_err()
    );
    data.points[0].x = 1.;
    data.points[0].elapsed_ms = 2;
    assert!(
        s.create_research_resource("time", &ResearchData::Stroke(data.clone()))
            .is_err()
    );
    data.points = vec![
        StrokePoint {
            x: 1.,
            y: 1.,
            elapsed_ms: 0,
            pressure: None,
            tilt_x: None,
            tilt_y: None
        };
        10_000
    ];
    for _ in 0..10 {
        s.create_research_resource("bounded stroke", &ResearchData::Stroke(data.clone()))
            .unwrap();
    }
    assert!(
        s.create_research_resource("overflow", &ResearchData::Stroke(data))
            .is_err()
    );
    assert_eq!(
        s.research_resources(Some(ResourceKind::Stroke), 0, 100)
            .unwrap()
            .len(),
        10
    );
}
#[test]
fn missing_file_anchor_and_legacy_identity_collision_are_diagnosed_without_repair() {
    let t = tempfile::tempdir().unwrap();
    let (_, id, _, _) = legacy_v11(t.path());
    let mut s = Storage::open(t.path().into()).unwrap();
    s.connection
        .execute("UPDATE link_index SET anchor_id=?1", ["f".repeat(32)])
        .unwrap();
    let path = s.indexed_document_path(&id).unwrap();
    let moved = t.path().join("moved.md");
    std::fs::rename(&path, &moved).unwrap();
    let audit = s.audit_research_resources(100).unwrap();
    assert!(
        audit
            .issues
            .iter()
            .any(|i| i.code == "missingAnnotationAnchor")
    );
    assert!(
        audit
            .issues
            .iter()
            .any(|i| i.code == "missingOrUnsafeFile" && i.resource.id == id)
    );
    assert!(audit.issues.iter().any(|i| i.code == "missingFingerprint"));
    let (doc, _) = pdf(&mut s);
    assert!(s.connection.execute("INSERT INTO research_entities SELECT resource_type,?1,title,format_version,revision,data_json,created_at,updated_at FROM research_entities WHERE resource_type='document' AND id=?2",params![id,doc.key.id]).is_err());
    assert!(moved.is_file());
    assert!(s.indexed_document_path(&id).unwrap() == path);
}

#[test]
fn required_dependencies_protect_legacy_project_document_and_pdf_deletion() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().into()).unwrap();
    s.save_project(None, "research").unwrap();
    let project = s.projects().unwrap().remove(0);
    let data = ResearchData::Document(DocumentResource {
        document_type: DocumentType::Pdf,
        project_id: Some(project.id.clone()),
        relative_path: None,
        code_workspace_id: None,
        content_hash: Some(hash()),
    });
    let doc = s.create_research_resource("research PDF", &data).unwrap();
    assert!(s.delete_empty_project(&project.id).is_err());
    assert_eq!(s.projects().unwrap().len(), 1);
    let attachment = s
        .create_research_resource(
            "attachment",
            &ResearchData::PdfAttachment(PdfAttachment {
                document_id: doc.key.id.clone(),
                reference_id: None,
                content_hash: hash(),
                byte_size: 1,
            }),
        )
        .unwrap();
    assert!(
        s.connection
            .execute(
                "DELETE FROM research_entities WHERE resource_type='document' AND id=?1",
                [&doc.key.id]
            )
            .is_err()
    );
    assert!(s.research_resource(&attachment.key).is_ok());
    let path = t.path().join("note.md");
    std::fs::write(&path, "keep").unwrap();
    let md = s.index_document(&path, None).unwrap();
    s.create_research_resource(
        "claim",
        &ResearchData::Evidence(ResearchEvidence {
            claim: "claim".into(),
            document_id: Some(md.id.clone()),
        }),
    )
    .unwrap();
    assert!(
        s.connection
            .execute("DELETE FROM markdown_documents WHERE id=?1", [md.id])
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), "keep");
}
