-- Editable snapshots share the immutable original. No flattened image duplication.
CREATE TABLE annotation_revisions (
  revision_id TEXT PRIMARY KEY,
  screenshot_id TEXT NOT NULL REFERENCES screenshots(id) ON DELETE CASCADE,
  version_number INTEGER NOT NULL CHECK(version_number > 0),
  parent_revision_id TEXT,
  annotation_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  description TEXT NOT NULL CHECK(length(description) <= 500),
  schema_version INTEGER NOT NULL,
  UNIQUE(screenshot_id, version_number),
  UNIQUE(screenshot_id, revision_id),
  FOREIGN KEY(screenshot_id, parent_revision_id)
    REFERENCES annotation_revisions(screenshot_id, revision_id)
) STRICT;
-- Preserve legacy bytes, including unsupported versions. Reads validate separately.
INSERT INTO annotation_revisions
SELECT lower(hex(randomblob(16))), id, 1, NULL, annotation_data, updated_at,
       '升级前已保存的标注',
       CASE WHEN json_valid(annotation_data) THEN coalesce(json_extract(annotation_data,'$.schemaVersion'),0) ELSE 0 END
FROM screenshots;
CREATE TRIGGER initial_annotation_revision AFTER INSERT ON screenshots BEGIN
  INSERT INTO annotation_revisions VALUES
    (lower(hex(randomblob(16))), NEW.id, 1, NULL, NEW.annotation_data,
     NEW.created_at, '首次保存', json_extract(NEW.annotation_data,'$.schemaVersion'));
END;
