-- Logical membership only: no image path changes and no backfill deletion.
CREATE TABLE projects (
 id TEXT PRIMARY KEY,
 name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 80),
 normalized_name TEXT NOT NULL UNIQUE,
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
) STRICT;
CREATE TABLE screenshot_projects (
 screenshot_id TEXT PRIMARY KEY REFERENCES screenshots(id) ON DELETE CASCADE,
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT
) STRICT;
CREATE INDEX screenshot_projects_project ON screenshot_projects(project_id,screenshot_id);
-- An index of explicitly opened/saved files, not another copy of Markdown content.
CREATE TABLE markdown_documents (
 id TEXT PRIMARY KEY,
 path TEXT NOT NULL UNIQUE,
 name TEXT NOT NULL,
 project_id TEXT REFERENCES projects(id) ON DELETE RESTRICT,
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
) STRICT;
CREATE INDEX markdown_documents_project ON markdown_documents(project_id,updated_at DESC,id);
