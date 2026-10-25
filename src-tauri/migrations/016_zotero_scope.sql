-- Extend the existing external-identity adapter, preserving legacy scope exactly.
DROP TRIGGER research_zotero_valid;
ALTER TABLE research_zotero_keys RENAME TO research_zotero_keys_v15;
CREATE TABLE research_zotero_keys (
 instance_id TEXT NOT NULL CHECK(length(instance_id)=32 AND instance_id NOT GLOB '*[^0-9a-f]*'),
 library_id INTEGER NOT NULL CHECK(library_id>=0),
 item_key TEXT NOT NULL CHECK(length(item_key)=8 AND item_key NOT GLOB '*[^A-Z0-9]*'),
 resource_type TEXT NOT NULL CHECK(resource_type IN ('literature','pdfAttachment','annotation')),
 resource_id TEXT NOT NULL,
 library_type TEXT NOT NULL DEFAULT 'legacy' CHECK(library_type IN ('legacy','user','group')),
 metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json) AND length(CAST(metadata_json AS BLOB))<=65536),
 CHECK(library_id>0 OR library_type='user'),
 PRIMARY KEY(instance_id,library_type,library_id,item_key)
) STRICT;
INSERT INTO research_zotero_keys(instance_id,library_id,item_key,resource_type,resource_id)
 SELECT instance_id,library_id,item_key,resource_type,resource_id FROM research_zotero_keys_v15;
DROP TABLE research_zotero_keys_v15;
CREATE TRIGGER research_zotero_valid BEFORE INSERT ON research_zotero_keys BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM research_resource_catalog WHERE resource_type=NEW.resource_type AND id=NEW.resource_id)
 THEN RAISE(ABORT,'external key target missing') END;
END;

CREATE INDEX research_zotero_target ON research_zotero_keys(resource_type,resource_id);
