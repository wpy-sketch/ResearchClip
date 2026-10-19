"""A06 M1: reproduce schema5 sync prerequisites with disposable logical clients.

No networking, SyncEngine, credentials, production database or migration6.
Runs the existing migration SQL with Python's SQLite, not the Rust migration runner.
PASS means an audit assertion reproduced, not WebDAV synchronization passed.
"""
from pathlib import Path
import hashlib
import json
import sqlite3
import struct
import sys
import tempfile
import unittest
import zlib

ROOT = Path(__file__).resolve().parent.parent
MIGRATIONS = sorted((ROOT / "src-tauri/migrations").glob("[0-9][0-9][0-9]_*.sql"))
assert len(MIGRATIONS) == 5, "Audit targets schema5; review before running against a new schema."
SHOT = "1" * 32
TAG = "2" * 32
PROJECT = "3" * 32
RESULTS = []


def annotation(text):
    return json.dumps({"schemaVersion": 1, "image": {"widthPx": 8, "heightPx": 6},
                       "objects": [{"id": "text", "type": "text", "x": 1, "y": 1,
                                    "text": text, "fontSize": 12,
                                    "color": "#123456", "strokeWidth": 2}]}, ensure_ascii=False)


def original_png(tint=10):
    def chunk(kind, value):
        return struct.pack(">I", len(value)) + kind + value + struct.pack(">I", zlib.crc32(kind + value))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 8, 6, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress((b"\0" + bytes([tint, 20, 30, 128]) * 8) * 6))
            + chunk(b"IEND", b""))


class ModelAudit(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="model-", dir=RUN_ROOT)
        self.root = Path(self.temporary.name)
        self.assertTrue(self.root.resolve().is_relative_to(RUN_ROOT.resolve()))
        self.a = sqlite3.connect(self.root / "A.sqlite3")
        for number, path in enumerate(MIGRATIONS, 1):
            self.a.executescript("BEGIN IMMEDIATE;\n" + path.read_text(encoding="utf-8")
                                 + f"\nPRAGMA user_version={number}; COMMIT;")
        self.a.execute("PRAGMA foreign_keys=ON")
        self.a.execute("INSERT INTO screenshots(id,title,original_image_path,thumbnail_path,annotation_data,"
                       "width_px,height_px,created_at,updated_at) VALUES(?,?,?,?,?,8,6,1,1)",
                       (SHOT, "owned fixture", f"originals/{SHOT}.png", f"thumbnails/{SHOT}.png", annotation("base")))
        self.a.execute("INSERT INTO projects VALUES(?,?,?,1,1)", (PROJECT, "Project", "project"))
        self.a.execute("INSERT INTO screenshot_projects VALUES(?,?)", (SHOT, PROJECT))
        self.a.execute("INSERT INTO tags VALUES(?,?,?)", (TAG, "Research", "research"))
        self.a.execute("INSERT INTO screenshot_tags VALUES(?,?)", (SHOT, TAG))
        self.a.execute("INSERT INTO notes VALUES(?,?,?,?,1,1,1)", ("4" * 32, SHOT, "owned note", None))
        self.a.commit()
        self.b = sqlite3.connect(self.root / "B.sqlite3")
        self.a.backup(self.b)
        self.b.execute("PRAGMA foreign_keys=ON")
        for client in ("A", "B"):
            folder = self.root / client / "originals"
            folder.mkdir(parents=True)
            (folder / f"{SHOT}.png").write_bytes(original_png())

    def tearDown(self):
        self.a.close()
        self.b.close()
        self.assertTrue(self.root.resolve().is_relative_to(RUN_ROOT.resolve()))
        self.temporary.cleanup()

    def edit(self, connection, text, rid):
        parent = connection.execute("SELECT revision_id FROM annotation_revisions WHERE screenshot_id=? "
                                    "ORDER BY version_number DESC LIMIT 1", (SHOT,)).fetchone()[0]
        connection.execute("INSERT INTO annotation_revisions VALUES(?,?,2,?,?,2,'owned edit',1)",
                           (rid, SHOT, parent, annotation(text)))
        connection.execute("UPDATE screenshots SET annotation_data=?,revision=revision+1,updated_at=2 WHERE id=?",
                           (annotation(text), SHOT))
        connection.commit()

    def test_01_existing_schema_and_immutable_original_are_usable_foundations(self):
        self.assertEqual(self.a.execute("PRAGMA user_version").fetchone()[0], 5)
        self.edit(self.a, "A", "a" * 32)
        self.assertEqual((self.root / "A/originals" / f"{SHOT}.png").read_bytes(), original_png())
        self.assertEqual(self.a.execute("PRAGMA foreign_key_check").fetchall(), [])

    def test_02_single_end_change_can_be_seen_but_counter_is_only_local(self):
        self.edit(self.a, "A", "a" * 32)
        self.assertEqual(self.a.execute("SELECT revision FROM screenshots").fetchone()[0], 2)
        self.assertEqual(self.b.execute("SELECT revision FROM screenshots").fetchone()[0], 1)

    def test_03_two_clients_fork_same_ordinal_and_naive_history_import_fails(self):
        self.edit(self.a, "A", "a" * 32)
        self.edit(self.b, "B", "b" * 32)
        self.assertEqual(self.a.execute("SELECT revision FROM screenshots").fetchone(),
                         self.b.execute("SELECT revision FROM screenshots").fetchone())
        self.assertNotEqual(self.a.execute("SELECT annotation_data FROM screenshots").fetchone(),
                            self.b.execute("SELECT annotation_data FROM screenshots").fetchone())
        branch = self.b.execute("SELECT * FROM annotation_revisions WHERE version_number=2").fetchone()
        with self.assertRaises(sqlite3.IntegrityError):
            self.a.execute("INSERT INTO annotation_revisions VALUES(?,?,?,?,?,?,?,?)", branch)
        self.a.rollback()

    def test_04_completed_physical_delete_has_no_durable_sync_tombstone(self):
        # Mirrors the existing DELETE + pending file cleanup acknowledgement at SQL level.
        self.a.execute("DELETE FROM screenshots WHERE id=?", (SHOT,))
        self.a.execute("INSERT INTO pending_file_deletions VALUES(?,?)", (f"originals/{SHOT}.png", SHOT))
        self.a.commit()
        (self.root / "A/originals" / f"{SHOT}.png").unlink()
        self.a.execute("DELETE FROM pending_file_deletions")
        self.a.commit()
        self.assertEqual(self.a.execute("SELECT count(*) FROM annotation_revisions").fetchone()[0], 0)
        self.assertEqual(self.a.execute("SELECT count(*) FROM notes").fetchone()[0], 0)
        self.assertEqual(self.a.execute("SELECT count(*) FROM pending_file_deletions").fetchone()[0], 0)
        self.assertEqual(self.b.execute("SELECT count(*) FROM screenshots WHERE id=?", (SHOT,)).fetchone()[0], 1)
        self.assertFalse(any("tombstone" in name or "sync" in name for (name,) in
                             self.a.execute("SELECT name FROM sqlite_master WHERE type='table'")))

    def test_05_same_named_documents_have_no_cross_device_path_identity(self):
        for connection, client, docid in ((self.a, "A", "c" * 32), (self.b, "B", "d" * 32)):
            document = self.root / client / "研究.md"
            document.write_text("# Same title\n", encoding="utf-8")
            connection.execute("INSERT INTO markdown_documents VALUES(?,?,?,?,1,1)",
                               (docid, str(document.resolve()), "研究.md", PROJECT))
            connection.commit()
        a = self.a.execute("SELECT id,path,name FROM markdown_documents").fetchone()
        b = self.b.execute("SELECT id,path,name FROM markdown_documents").fetchone()
        self.assertEqual(a[2], b[2])
        self.assertNotEqual(a[0], b[0])
        self.assertNotEqual(a[1], b[1])

    def test_06_external_markdown_and_assets_changes_are_not_index_revisions(self):
        doc = self.root / "A/研究.md"
        assets = doc.parent / "assets"
        assets.mkdir()
        asset = assets / "figure.png"
        asset.write_bytes(original_png())
        doc.write_text("![figure](assets/figure.png)\nbase", encoding="utf-8")
        self.a.execute("INSERT INTO markdown_documents VALUES(?,?,?,?,1,1)",
                       ("c" * 32, str(doc.resolve()), doc.name, PROJECT))
        self.a.commit()
        before = self.a.execute("SELECT * FROM markdown_documents").fetchone()
        doc.write_text("![figure](assets/figure.png)\nexternal edit", encoding="utf-8")
        asset.write_bytes(original_png(11))
        self.assertEqual(before, self.a.execute("SELECT * FROM markdown_documents").fetchone())
        columns = {r[1] for r in self.a.execute("PRAGMA table_info(markdown_documents)")}
        self.assertFalse({"content_hash", "revision", "deleted_at"} & columns)

    def test_07_distinct_tag_and_project_ids_with_same_names_cannot_be_blindly_merged(self):
        for sql, args in [("INSERT INTO tags VALUES(?,?,?)", ("e" * 32, "Research", "research")),
                          ("INSERT INTO projects VALUES(?,?,?,2,2)", ("f" * 32, "Project", "project"))]:
            with self.assertRaises(sqlite3.IntegrityError):
                self.a.execute(sql, args)
            self.a.rollback()

    def test_08_shared_tag_rename_also_changes_unselected_project_display(self):
        other = "5" * 32
        self.a.execute("INSERT INTO screenshots(id,title,original_image_path,thumbnail_path,annotation_data,"
                       "width_px,height_px,created_at,updated_at) VALUES(?,?,?,?,?,8,6,1,1)",
                       (other, "unclassified", f"originals/{other}.png", f"thumbnails/{other}.png", annotation("other")))
        self.a.execute("INSERT INTO screenshot_tags VALUES(?,?)", (other, TAG))
        self.a.execute("UPDATE tags SET name='Renamed',normalized_name='renamed' WHERE id=?", (TAG,))
        self.a.commit()
        self.assertEqual(self.a.execute("SELECT name FROM tags JOIN screenshot_tags ON tags.id=tag_id "
                                        "WHERE screenshot_id=?", (other,)).fetchone()[0], "Renamed")
        self.assertIsNone(self.a.execute("SELECT project_id FROM screenshot_projects WHERE screenshot_id=?", (other,)).fetchone())


class AuditResult(unittest.TextTestResult):
    def addSuccess(self, test):
        super().addSuccess(test)
        RESULTS.append({"id": test.id().split(".")[-1], "status": "PASS", "meaning": "schema5 model audit assertion reproduced; not sync acceptance"})

    def addFailure(self, test, error):
        super().addFailure(test, error)
        RESULTS.append({"id": test.id().split(".")[-1], "status": "FAIL"})

    def addError(self, test, error):
        super().addError(test, error)
        RESULTS.append({"id": test.id().split(".")[-1], "status": "FAIL", "kind": "audit execution error"})


if __name__ == "__main__":
    # All destructive fixture operations remain inside this newly created directory.
    parent = ROOT / "artifacts/qa-a06"
    parent.mkdir(parents=True, exist_ok=True)
    assert parent.resolve().is_relative_to(ROOT), "Test data must remain inside this repository."
    RUN_ROOT = Path(tempfile.mkdtemp(prefix="m1-", dir=parent))
    assert RUN_ROOT.resolve().parent == parent.resolve()
    outcome = unittest.TextTestRunner(verbosity=2, resultclass=AuditResult).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(ModelAudit))
    report = {"auditResult": "PASS" if outcome.wasSuccessful() else "FAIL",
              "phaseGate": "BLOCKED", "syncImplemented": False,
              "reason": "schema5 needs explicit sync migration; actual test server information pending",
              "python": sys.version.split()[0], "sqlite": sqlite3.sqlite_version,
              "logicalClients": 2, "tests": outcome.testsRun, "checks": RESULTS,
              "originalFixtureSha256": hashlib.sha256(original_png()).hexdigest(),
              "migrationHashes": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in MIGRATIONS},
              "serverTests": "NOT_RUN", "productionDataAccess": False}
    (RUN_ROOT / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("Audit evidence: " + str((RUN_ROOT / "report.json").relative_to(ROOT)))
    sys.exit(0 if outcome.wasSuccessful() else 1)
