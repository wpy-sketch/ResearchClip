CREATE TABLE screenshots (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 200),
  original_image_path TEXT NOT NULL UNIQUE,
  thumbnail_path TEXT NOT NULL UNIQUE,
  annotation_data TEXT NOT NULL CHECK(json_valid(annotation_data)),
  width_px INTEGER NOT NULL CHECK(width_px > 0),
  height_px INTEGER NOT NULL CHECK(height_px > 0 AND width_px * height_px <= 16777216),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  is_favorite INTEGER NOT NULL DEFAULT 0 CHECK(is_favorite IN (0,1)),
  revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0)
) STRICT;
CREATE INDEX screenshots_updated ON screenshots(updated_at DESC, id);
CREATE TABLE notes (
  id TEXT PRIMARY KEY,
  screenshot_id TEXT NOT NULL REFERENCES screenshots(id) ON DELETE CASCADE,
  content TEXT NOT NULL,
  source_url TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0)
) STRICT;
CREATE INDEX notes_screenshot ON notes(screenshot_id, created_at, id);
CREATE TABLE tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 50),
  normalized_name TEXT NOT NULL UNIQUE
) STRICT;
CREATE TABLE screenshot_tags (
  screenshot_id TEXT NOT NULL REFERENCES screenshots(id) ON DELETE CASCADE,
  tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY(screenshot_id, tag_id)
) STRICT;
CREATE INDEX screenshot_tags_tag ON screenshot_tags(tag_id, screenshot_id);
-- Intent is committed before files; successful file/record commit removes it atomically.
CREATE TABLE pending_screenshots (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  annotation_data TEXT NOT NULL CHECK(json_valid(annotation_data)),
  width_px INTEGER NOT NULL,
  height_px INTEGER NOT NULL,
  created_at INTEGER NOT NULL
) STRICT;
-- Only explicitly deleted, app-owned image references may enter this queue.
CREATE TABLE pending_file_deletions (
  relative_path TEXT PRIMARY KEY,
  screenshot_id TEXT NOT NULL
) STRICT;
