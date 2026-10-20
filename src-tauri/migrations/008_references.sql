CREATE TABLE literature_references (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    authors_json TEXT NOT NULL CHECK(json_valid(authors_json)),
    year INTEGER,
    venue TEXT NOT NULL,
    doi TEXT UNIQUE,
    url TEXT NOT NULL,
    bibtex_key TEXT NOT NULL UNIQUE COLLATE BINARY,
    bibtex_entry TEXT NOT NULL,
    verified_source TEXT NOT NULL CHECK(verified_source IN ('manual','crossref','bibtex','edited')),
    source_metadata TEXT NOT NULL CHECK(json_valid(source_metadata)),
    updated_at INTEGER NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE screenshot_references (
    screenshot_id TEXT NOT NULL REFERENCES screenshots(id) ON DELETE CASCADE,
    reference_id TEXT NOT NULL REFERENCES literature_references(id) ON DELETE CASCADE,
    PRIMARY KEY(screenshot_id,reference_id)
);
CREATE TABLE document_references (
    document_id TEXT NOT NULL REFERENCES markdown_documents(id) ON DELETE CASCADE,
    reference_id TEXT NOT NULL REFERENCES literature_references(id) ON DELETE CASCADE,
    PRIMARY KEY(document_id,reference_id)
);
