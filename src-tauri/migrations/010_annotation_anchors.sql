CREATE TABLE annotation_anchors (
 anchor_id TEXT PRIMARY KEY CHECK(length(anchor_id)=32),
 screenshot_id TEXT NOT NULL REFERENCES screenshots(id) ON DELETE CASCADE,
 image_revision TEXT NOT NULL CHECK(length(image_revision)=64),
 x REAL NOT NULL CHECK(x>=0), y REAL NOT NULL CHECK(y>=0),
 width REAL NOT NULL CHECK(width>=0), height REAL NOT NULL CHECK(height>=0),
 title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 200),
 description TEXT NOT NULL CHECK(length(description)<=4000),
 revision INTEGER NOT NULL CHECK(revision>0),
 updated_at INTEGER NOT NULL,
 CHECK((width=0 AND height=0) OR (width>0 AND height>0))
) STRICT;
CREATE INDEX anchors_screenshot ON annotation_anchors(screenshot_id,updated_at,anchor_id);
ALTER TABLE link_index ADD COLUMN anchor_id TEXT CHECK(anchor_id IS NULL OR length(anchor_id)=32);
