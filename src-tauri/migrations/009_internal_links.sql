-- IDs remain those of markdown_documents and screenshots. Missing targets are retained.
CREATE TABLE link_index (
 source_id TEXT NOT NULL REFERENCES markdown_documents(id) ON DELETE CASCADE,
 target_id TEXT NOT NULL CHECK(length(target_id)=32),
 link_type TEXT NOT NULL CHECK(link_type IN ('document','screenshot')),
 source_location INTEGER NOT NULL CHECK(source_location>=0),
 source_end INTEGER NOT NULL CHECK(source_end>source_location),
 source_line INTEGER NOT NULL CHECK(source_line>=1),
 label TEXT NOT NULL,
 updated_at INTEGER NOT NULL,
 PRIMARY KEY(source_id,source_location)
) STRICT;
CREATE INDEX link_index_edges ON link_index(source_id,link_type,target_id);
CREATE INDEX link_index_target ON link_index(link_type,target_id,source_id);
CREATE TABLE link_index_state (
 document_id TEXT PRIMARY KEY REFERENCES markdown_documents(id) ON DELETE CASCADE,
 source_stamp TEXT NOT NULL,
 available INTEGER NOT NULL CHECK(available IN (0,1)),
 updated_at INTEGER NOT NULL
) STRICT;
CREATE TABLE document_tags (
 document_id TEXT NOT NULL REFERENCES markdown_documents(id) ON DELETE CASCADE,
 tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
 PRIMARY KEY(document_id,tag_id)
) STRICT;
CREATE INDEX document_tags_tag ON document_tags(tag_id,document_id);
