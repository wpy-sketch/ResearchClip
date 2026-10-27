-- Standard Zotero annotation sync sidecars; never a Zotero database table.
CREATE TABLE zotero_annotation_sync (
 id TEXT PRIMARY KEY CHECK(length(id)=32),
 document_type TEXT NOT NULL DEFAULT 'document' CHECK(document_type='document'),
 document_id TEXT NOT NULL,instance_id TEXT NOT NULL CHECK(length(instance_id)=32),
 library_type TEXT NOT NULL CHECK(library_type IN ('user','group')),library_id INTEGER NOT NULL CHECK(library_id>=0),
 item_key TEXT NOT NULL CHECK(length(item_key)=8),attachment_key TEXT NOT NULL CHECK(length(attachment_key)=8),
 content_hash TEXT NOT NULL CHECK(length(content_hash)=64),annotation_key TEXT NOT NULL CHECK(length(annotation_key)=8),
 revision INTEGER NOT NULL CHECK(revision>0),status TEXT NOT NULL CHECK(status IN ('SYNCED','LOCAL_DIRTY','REMOTE_CHANGED','CONFLICT','FAILED','READ_ONLY')),
 entry_json TEXT NOT NULL CHECK(json_valid(entry_json) AND length(cast(entry_json AS BLOB))<=1048576),
 UNIQUE(document_id,instance_id,library_type,library_id,item_key,attachment_key,content_hash,annotation_key),
 FOREIGN KEY(document_type,document_id) REFERENCES research_entities(resource_type,id)
) STRICT;
CREATE TABLE zotero_annotation_sync_revisions (
 revision_id TEXT PRIMARY KEY CHECK(length(revision_id)=32),change_id TEXT NOT NULL REFERENCES zotero_annotation_sync(id),
 version_number INTEGER NOT NULL CHECK(version_number>0),parent_revision_id TEXT REFERENCES zotero_annotation_sync_revisions(revision_id),
 created_at INTEGER NOT NULL CHECK(created_at>0),description TEXT NOT NULL CHECK(length(description)<=500),
 snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json) AND length(cast(snapshot_json AS BLOB))<=1048576),
 UNIQUE(change_id,version_number)
) STRICT;
CREATE TABLE zotero_annotation_sync_scopes (
 document_type TEXT NOT NULL DEFAULT 'document' CHECK(document_type='document'),document_id TEXT NOT NULL,
 instance_id TEXT NOT NULL CHECK(length(instance_id)=32),library_type TEXT NOT NULL CHECK(library_type IN ('user','group')),
 library_id INTEGER NOT NULL CHECK(library_id>=0),item_key TEXT NOT NULL CHECK(length(item_key)=8),attachment_key TEXT NOT NULL CHECK(length(attachment_key)=8),
 content_hash TEXT NOT NULL CHECK(length(content_hash)=64),scope_json TEXT NOT NULL CHECK(json_valid(scope_json) AND length(cast(scope_json AS BLOB))<=16384),
 PRIMARY KEY(document_id,instance_id,library_type,library_id,item_key,attachment_key,content_hash),
 FOREIGN KEY(document_type,document_id) REFERENCES research_entities(resource_type,id)
) STRICT;
