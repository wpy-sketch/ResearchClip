-- Existing resources are projected, not copied or assigned new IDs.
CREATE TABLE research_entities (
 resource_type TEXT NOT NULL CHECK(resource_type IN ('document','pdfAttachment','pdfLocation','annotation','handwritingLayer','stroke','codeWorkspace','evidence','agentSession')),
 id TEXT NOT NULL CHECK(length(id)=32 AND id NOT GLOB '*[^0-9a-f]*'),
 title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 200),
 format_version INTEGER NOT NULL CHECK(format_version>0),
 revision INTEGER NOT NULL CHECK(revision>0),
 data_json TEXT NOT NULL CHECK(json_valid(data_json) AND length(CAST(data_json AS BLOB))<=2097152 AND json_extract(data_json,'$.kind')=resource_type),
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
 PRIMARY KEY(resource_type,id)
) STRICT;
CREATE VIEW research_resource_catalog AS
 SELECT 'project' AS resource_type,id,name AS title,1 AS format_version,NULL AS revision,created_at,updated_at,1 AS is_legacy,
 json_object('name',name) AS data_json FROM projects
 UNION ALL SELECT 'document',id,name,1,NULL,created_at,updated_at,1,
 json_object('documentType','markdown','path',path,'projectId',project_id) FROM markdown_documents
 UNION ALL SELECT 'literature',id,title,1,revision,updated_at,updated_at,1,
 json_object('doi',doi,'bibtexKey',bibtex_key,'source',verified_source) FROM literature_references
 UNION ALL SELECT 'screenshot',id,title,1,revision,created_at,updated_at,1,
 json_object('originalImagePath',original_image_path,'widthPx',width_px,'heightPx',height_px) FROM screenshots
 UNION ALL SELECT resource_type,id,title,format_version,revision,created_at,updated_at,0,data_json FROM research_entities;
CREATE TRIGGER research_entity_no_legacy_document BEFORE INSERT ON research_entities
 WHEN NEW.resource_type='document' AND EXISTS(SELECT 1 FROM markdown_documents WHERE id=NEW.id)
 BEGIN SELECT RAISE(ABORT,'resource identity already exists'); END;
CREATE TRIGGER research_legacy_document_no_entity BEFORE INSERT ON markdown_documents
 WHEN EXISTS(SELECT 1 FROM research_entities WHERE resource_type='document' AND id=NEW.id)
 BEGIN SELECT RAISE(ABORT,'resource identity already exists'); END;
CREATE TABLE research_relations (
 source_type TEXT NOT NULL,source_id TEXT NOT NULL,target_type TEXT NOT NULL,target_id TEXT NOT NULL,
 relation_type TEXT NOT NULL CHECK(relation_type IN ('related','source','contains','citation','supports','dependsOn')),
 origin TEXT NOT NULL CHECK(origin IN ('manual','model')),
 PRIMARY KEY(source_type,source_id,target_type,target_id,relation_type,origin),
 CHECK(source_type!=target_type OR source_id!=target_id)
) STRICT;
-- A view cannot be an FK parent. Check inserts transactionally; retain dangling links on legacy deletion for diagnostics.
CREATE TRIGGER research_relation_valid BEFORE INSERT ON research_relations BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM research_resource_catalog WHERE resource_type=NEW.source_type AND id=NEW.source_id)
 OR NOT EXISTS(SELECT 1 FROM research_resource_catalog WHERE resource_type=NEW.target_type AND id=NEW.target_id)
 THEN RAISE(ABORT,'relation endpoint missing') END;
END;
CREATE INDEX research_relations_target ON research_relations(target_type,target_id);
CREATE VIEW research_resource_edges AS
 SELECT * FROM research_relations
 UNION ALL SELECT 'screenshot',screenshot_id,'project',project_id,'dependsOn','legacy' FROM screenshot_projects
 UNION ALL SELECT 'document',id,'project',project_id,'dependsOn','legacy' FROM markdown_documents WHERE project_id IS NOT NULL
 UNION ALL SELECT 'screenshot',screenshot_id,'literature',reference_id,'citation','legacy' FROM screenshot_references
 UNION ALL SELECT 'document',document_id,'literature',reference_id,'citation','legacy' FROM document_references
 UNION ALL SELECT DISTINCT 'document',source_id,CASE link_type WHEN 'document' THEN 'document' ELSE 'screenshot' END,target_id,'citation','legacyLink' FROM link_index;
CREATE TABLE research_zotero_keys (
 instance_id TEXT NOT NULL CHECK(length(instance_id)=32 AND instance_id NOT GLOB '*[^0-9a-f]*'),
 library_id INTEGER NOT NULL CHECK(library_id>0),item_key TEXT NOT NULL CHECK(length(item_key)=8 AND item_key NOT GLOB '*[^A-Z0-9]*'),
 resource_type TEXT NOT NULL CHECK(resource_type IN ('literature','pdfAttachment','annotation')),
 resource_id TEXT NOT NULL,
 PRIMARY KEY(instance_id,library_id,item_key)
) STRICT;
CREATE TRIGGER research_zotero_valid BEFORE INSERT ON research_zotero_keys BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM research_resource_catalog WHERE resource_type=NEW.resource_type AND id=NEW.resource_id)
 THEN RAISE(ABORT,'external key target missing') END;
END;
CREATE TABLE research_document_fingerprints (
 document_id TEXT PRIMARY KEY REFERENCES markdown_documents(id) ON DELETE CASCADE,
 content_hash TEXT NOT NULL CHECK(length(content_hash)=64 AND content_hash NOT GLOB '*[^0-9a-f]*'),
 byte_size INTEGER NOT NULL CHECK(byte_size>=0 AND byte_size<=2097152)
) STRICT;

CREATE INDEX research_strokes_layer ON research_entities(json_extract(data_json,'$.value.layerId')) WHERE resource_type='stroke';
CREATE UNIQUE INDEX research_document_locator ON research_entities(json_extract(data_json,'$.value.documentType'),coalesce(json_extract(data_json,'$.value.codeWorkspaceId'),''),json_extract(data_json,'$.value.relativePath')) WHERE resource_type='document' AND json_extract(data_json,'$.value.relativePath') IS NOT NULL;

-- Required typed dependencies cannot be removed through legacy delete paths.
CREATE TRIGGER research_project_in_use BEFORE DELETE ON projects WHEN EXISTS(SELECT 1 FROM research_relations WHERE origin='model' AND target_type='project' AND target_id=OLD.id) BEGIN SELECT RAISE(ABORT,'project has research resources'); END;
CREATE TRIGGER research_document_in_use BEFORE DELETE ON markdown_documents WHEN EXISTS(SELECT 1 FROM research_relations WHERE origin='model' AND target_type='document' AND target_id=OLD.id) BEGIN SELECT RAISE(ABORT,'document has research resources'); END;
CREATE TRIGGER research_literature_in_use BEFORE DELETE ON literature_references WHEN EXISTS(SELECT 1 FROM research_relations WHERE origin='model' AND target_type='literature' AND target_id=OLD.id) BEGIN SELECT RAISE(ABORT,'reference has research resources'); END;
CREATE TRIGGER research_entity_in_use BEFORE DELETE ON research_entities WHEN EXISTS(SELECT 1 FROM research_relations WHERE origin='model' AND target_type=OLD.resource_type AND target_id=OLD.id) BEGIN SELECT RAISE(ABORT,'resource has typed dependents'); END;
