CREATE TABLE semantic_embeddings (
    screenshot_id TEXT PRIMARY KEY REFERENCES screenshots(id) ON DELETE CASCADE,
    embedding_model_id TEXT NOT NULL,
    embedding_version INTEGER NOT NULL CHECK (embedding_version > 0),
    image_hash TEXT NOT NULL CHECK (length(image_hash) = 64),
    embedding_data BLOB NOT NULL CHECK (length(embedding_data) = 2048),
    source_stamp TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
