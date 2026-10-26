-- Derived PDF-location edges reuse stable Markdown IDs and the existing CAS index state.
CREATE TABLE pdf_note_links (
 source_id TEXT NOT NULL REFERENCES markdown_documents(id) ON DELETE CASCADE,
 target_id TEXT NOT NULL CHECK(length(target_id)=32),
 link_type TEXT NOT NULL CHECK(link_type='pdf-location'),
 source_location INTEGER NOT NULL CHECK(source_location>=0),
 source_end INTEGER NOT NULL CHECK(source_end>source_location),
 source_line INTEGER NOT NULL CHECK(source_line>=1),
 label TEXT NOT NULL,
 updated_at INTEGER NOT NULL,
 anchor_id TEXT CHECK(anchor_id IS NULL),
 PRIMARY KEY(source_id,source_location)
) STRICT;
CREATE INDEX pdf_note_links_target ON pdf_note_links(target_id,source_id);
