CREATE TABLE document_folders (
 id TEXT PRIMARY KEY,
 parent_id TEXT REFERENCES document_folders(id) ON DELETE RESTRICT,
 name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 80)
) STRICT;
CREATE UNIQUE INDEX document_folder_name ON document_folders(coalesce(parent_id,''),name COLLATE NOCASE);
CREATE TABLE document_organization (
 document_id TEXT PRIMARY KEY REFERENCES markdown_documents(id) ON DELETE CASCADE,
 folder_id TEXT REFERENCES document_folders(id) ON DELETE RESTRICT,
 is_favorite INTEGER NOT NULL DEFAULT 0 CHECK(is_favorite IN (0,1)),
 last_opened_at INTEGER NOT NULL DEFAULT 0 CHECK(last_opened_at>=0)
) STRICT;
INSERT INTO document_organization(document_id) SELECT id FROM markdown_documents;
CREATE TABLE markdown_revisions (
 revision_id TEXT PRIMARY KEY,
 document_id TEXT NOT NULL REFERENCES markdown_documents(id) ON DELETE RESTRICT,
 version_number INTEGER NOT NULL CHECK(version_number>0),
 parent_revision_id TEXT REFERENCES markdown_revisions(revision_id) ON DELETE RESTRICT,
 created_at INTEGER NOT NULL,
 description TEXT NOT NULL CHECK(length(description)<=500),
 schema_version INTEGER NOT NULL CHECK(schema_version=1),
 content_hash TEXT NOT NULL CHECK(length(content_hash)=64),
 markdown TEXT NOT NULL,
 assets_json TEXT NOT NULL,
 restored_from TEXT REFERENCES markdown_revisions(revision_id) ON DELETE RESTRICT,
 byte_size INTEGER NOT NULL CHECK(byte_size>=0),
 UNIQUE(document_id,version_number)
) STRICT;
CREATE TRIGGER markdown_revision_no_update BEFORE UPDATE ON markdown_revisions BEGIN SELECT RAISE(ABORT,'revisions are immutable'); END;
CREATE TRIGGER markdown_revision_no_delete BEFORE DELETE ON markdown_revisions BEGIN SELECT RAISE(ABORT,'revisions are immutable'); END;
