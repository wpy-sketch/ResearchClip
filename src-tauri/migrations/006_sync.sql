-- Resource sync metadata is local; no SQLite file is ever uploaded.
CREATE TABLE annotation_heads (
 screenshot_id TEXT PRIMARY KEY REFERENCES screenshots(id) ON DELETE CASCADE,
 revision_id TEXT REFERENCES annotation_revisions(revision_id),
 needs_review INTEGER NOT NULL DEFAULT 0 CHECK(needs_review IN (0,1))
) STRICT;
INSERT INTO annotation_heads
SELECT s.id,(SELECT r.revision_id FROM annotation_revisions r WHERE r.screenshot_id=s.id AND r.annotation_json=s.annotation_data ORDER BY r.version_number DESC LIMIT 1),
 NOT EXISTS(SELECT 1 FROM annotation_revisions r WHERE r.screenshot_id=s.id AND r.annotation_json=s.annotation_data)
FROM screenshots s;
CREATE TRIGGER annotation_head_created AFTER INSERT ON annotation_revisions BEGIN
 INSERT INTO annotation_heads VALUES(NEW.screenshot_id,NEW.revision_id,0)
 ON CONFLICT(screenshot_id) DO UPDATE SET revision_id=NEW.revision_id,needs_review=0;
END;
CREATE TABLE sync_identity (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1),device_id TEXT NOT NULL,counter INTEGER NOT NULL DEFAULT 0
) STRICT;
INSERT INTO sync_identity VALUES(1,lower(hex(randomblob(16))),0);
CREATE TABLE sync_config (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1),config_json TEXT NOT NULL CHECK(json_valid(config_json))
) STRICT;
CREATE TABLE sync_state (
 connection_id TEXT NOT NULL,resource_key TEXT NOT NULL,
 content_hash TEXT NOT NULL,manifest_hash TEXT NOT NULL,remote_etag TEXT NOT NULL,
 PRIMARY KEY(connection_id,resource_key)
) STRICT;
CREATE TABLE sync_queue (
 operation_id TEXT PRIMARY KEY,connection_id TEXT NOT NULL,resource_key TEXT NOT NULL,
 manifest_hash TEXT NOT NULL,expected_remote TEXT,
 phase TEXT NOT NULL CHECK(phase IN ('prepared','uncertain','acknowledged')),
 UNIQUE(connection_id,resource_key)
) STRICT;
CREATE TABLE sync_conflicts (
 id TEXT PRIMARY KEY,connection_id TEXT NOT NULL,resource_key TEXT NOT NULL,
 local_manifest TEXT,remote_manifest TEXT,reason TEXT NOT NULL,resolved INTEGER NOT NULL DEFAULT 0,
 UNIQUE(connection_id,resource_key)
) STRICT;
CREATE TABLE sync_tombstones (
 resource_key TEXT PRIMARY KEY,project_id TEXT,deleted_at INTEGER NOT NULL
) STRICT;
CREATE TRIGGER sync_screenshot_deleted BEFORE DELETE ON screenshots BEGIN
 INSERT INTO sync_tombstones VALUES('screenshot/'||OLD.id,(SELECT project_id FROM screenshot_projects WHERE screenshot_id=OLD.id),unixepoch()*1000)
 ON CONFLICT(resource_key) DO UPDATE SET project_id=excluded.project_id,deleted_at=excluded.deleted_at;
END;
CREATE TRIGGER sync_project_deleted BEFORE DELETE ON projects BEGIN
 INSERT INTO sync_tombstones VALUES('project/'||OLD.id,OLD.id,unixepoch()*1000)
 ON CONFLICT(resource_key) DO UPDATE SET deleted_at=excluded.deleted_at;
END;
CREATE TRIGGER sync_document_deleted BEFORE DELETE ON markdown_documents BEGIN
 INSERT INTO sync_tombstones VALUES('document/'||OLD.id,OLD.project_id,unixepoch()*1000)
 ON CONFLICT(resource_key) DO UPDATE SET project_id=excluded.project_id,deleted_at=excluded.deleted_at;
END;
-- Local grants are explicit, and bound to canonical paths rather than remote paths.
CREATE TABLE sync_document_grants (
 document_id TEXT PRIMARY KEY REFERENCES markdown_documents(id) ON DELETE CASCADE,
 authorized_path TEXT NOT NULL
) STRICT;
