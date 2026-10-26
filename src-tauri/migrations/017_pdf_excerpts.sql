-- Extend the existing screenshot write intent; recovery commits provenance with the image.
CREATE TABLE pending_pdf_excerpts (
 screenshot_id TEXT PRIMARY KEY REFERENCES pending_screenshots(id) ON DELETE CASCADE,
 draft_json TEXT NOT NULL CHECK(json_valid(draft_json) AND length(draft_json)<=16384)
) STRICT;
CREATE TABLE pdf_excerpts (
 screenshot_id TEXT PRIMARY KEY REFERENCES screenshots(id) ON DELETE CASCADE,
 location_type TEXT NOT NULL DEFAULT 'pdfLocation' CHECK(location_type='pdfLocation'),
 location_id TEXT NOT NULL UNIQUE,
 literature_id TEXT REFERENCES literature_references(id),
 dpi INTEGER NOT NULL CHECK(dpi IN (144,216,288)),
 rotation INTEGER NOT NULL CHECK(rotation IN (0,90,180,270)),
 FOREIGN KEY(location_type,location_id) REFERENCES research_entities(resource_type,id)
) STRICT;
