CREATE TABLE image_features (
 screenshot_id TEXT PRIMARY KEY REFERENCES screenshots(id) ON DELETE CASCADE,
 content_hash TEXT CHECK(content_hash IS NULL OR length(content_hash)=64),
 perceptual_hash TEXT CHECK(perceptual_hash IS NULL OR length(perceptual_hash)=16),
 difference_hash TEXT CHECK(difference_hash IS NULL OR length(difference_hash)=16),
 descriptor BLOB,
 hash_algorithm_version INTEGER NOT NULL,
 source_stamp TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('ready','failed')),
 error TEXT,
 updated_at INTEGER NOT NULL,
 CHECK(status!='ready' OR (content_hash IS NOT NULL AND perceptual_hash IS NOT NULL AND difference_hash IS NOT NULL AND length(descriptor)=768))
);
CREATE INDEX image_features_exact ON image_features(content_hash) WHERE status='ready';
CREATE TRIGGER image_features_source_changed AFTER UPDATE OF original_image_path,width_px,height_px ON screenshots
 WHEN OLD.original_image_path!=NEW.original_image_path OR OLD.width_px!=NEW.width_px OR OLD.height_px!=NEW.height_px
 BEGIN DELETE FROM image_features WHERE screenshot_id=NEW.id; END;
