-- Reading metadata only; PDF bytes remain immutable managed files.
CREATE TABLE pdf_reading_positions (
 resource_type TEXT NOT NULL DEFAULT 'document' CHECK(resource_type='document'),
 document_id TEXT NOT NULL,
 content_hash TEXT NOT NULL CHECK(length(content_hash)=64 AND content_hash NOT GLOB '*[^0-9a-f]*'),
 format_version INTEGER NOT NULL CHECK(format_version=1),
 revision INTEGER NOT NULL CHECK(revision>0),
 location_json TEXT NOT NULL CHECK(json_valid(location_json) AND length(location_json)<=2048),
 updated_at INTEGER NOT NULL,
 PRIMARY KEY(document_id),
 FOREIGN KEY(resource_type,document_id) REFERENCES research_entities(resource_type,id) ON DELETE RESTRICT
) STRICT;
