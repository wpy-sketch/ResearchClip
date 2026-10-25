CREATE TABLE citation_key_mappings (
 provider TEXT NOT NULL CHECK(provider='betterBibtex'),
 instance_id TEXT NOT NULL CHECK(length(instance_id)=32 AND instance_id NOT GLOB '*[^0-9a-f]*'),
 library_id INTEGER NOT NULL CHECK(library_id>0),
 external_key TEXT NOT NULL CHECK(length(external_key) BETWEEN 1 AND 100),
 reference_id TEXT NOT NULL REFERENCES literature_references(id) ON DELETE RESTRICT,
 PRIMARY KEY(provider,instance_id,library_id,external_key)
) STRICT;
