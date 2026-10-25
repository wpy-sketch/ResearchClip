use super::{
    Storage, StorageError,
    library::{check_id, now},
    revision_meta::RevisionMetadata,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentFolder {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizedDocument {
    pub id: String,
    pub name: String,
    pub folder_id: Option<String>,
    pub favorite: bool,
    pub last_opened_at: i64,
    pub available: bool,
}
#[derive(Serialize)]
pub struct DocumentDirectory {
    pub folders: Vec<DocumentFolder>,
    pub documents: Vec<OrganizedDocument>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionAsset {
    pub size: u64,
    pub sha256: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownRevision {
    #[serde(flatten)]
    pub metadata: RevisionMetadata,
    pub document_id: String,
    pub content_hash: String,
    pub restored_from: Option<String>,
}
pub struct RevisionContent {
    pub revision: MarkdownRevision,
    pub markdown: String,
    pub assets: BTreeMap<String, RevisionAsset>,
}
pub(super) fn record(
    connection: &Connection,
    id: &str,
    markdown: &str,
    assets: &BTreeMap<String, RevisionAsset>,
    restored_from: Option<&str>,
) -> Result<(), StorageError> {
    let hash = format!("{:x}", Sha256::digest(markdown.as_bytes()));
    let assets =
        serde_json::to_string(assets).map_err(|_| StorageError::Invalid("版本图片清单无效。"))?;
    let previous=connection.query_row("SELECT revision_id,version_number,content_hash,assets_json FROM markdown_revisions WHERE document_id=?1 ORDER BY version_number DESC LIMIT 1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).optional()?;
    let description = if let Some(revision) = restored_from {
        check_id(revision)?;
        let number:i64=connection.query_row("SELECT version_number FROM markdown_revisions WHERE document_id=?1 AND revision_id=?2",params![id,revision],|r|r.get(0)).optional()?.ok_or(StorageError::NotFound)?;
        format!("恢复版本 {number}")
    } else {
        "磁盘快照".into()
    };
    super::revision_meta::validate_description(&description)?;
    if restored_from.is_none()
        && previous
            .as_ref()
            .is_some_and(|p| p.2 == hash && p.3 == assets)
    {
        return Ok(());
    }
    let (local,total):(i64,i64)=connection.query_row("SELECT coalesce(sum(CASE WHEN document_id=?1 THEN byte_size ELSE 0 END),0),coalesce(sum(byte_size),0) FROM markdown_revisions",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;
    let size = (markdown.len() + assets.len()) as i64;
    if local + size > 128 * 1024 * 1024 || total + size > 512 * 1024 * 1024 {
        return Err(StorageError::Invalid(
            "历史达到单文档128MiB或资料库512MiB上限，历史未追加；文件可能已保存，请保留草稿。",
        ));
    }
    let number = previous.as_ref().map_or(1, |p| p.1 + 1);
    if number > 9_007_199_254_740_991 {
        return Err(StorageError::Invalid("版本号超出显示范围。"));
    }
    connection.execute("INSERT INTO markdown_revisions VALUES(lower(hex(randomblob(16))),?1,?2,?3,?4,?5,1,?6,?7,?8,?9,?10)",params![id,number,previous.map(|p|p.0),now()?,description,hash,markdown,assets,restored_from,size])?;
    Ok(())
}
fn revision(r: &rusqlite::Row<'_>) -> rusqlite::Result<MarkdownRevision> {
    Ok(MarkdownRevision {
        metadata: RevisionMetadata {
            revision_id: r.get(0)?,
            version_number: r.get(1)?,
            parent_revision_id: r.get(2)?,
            created_at: r.get(3)?,
            description: r.get(4)?,
            schema_version: r.get(5)?,
        },
        document_id: r.get(6)?,
        content_hash: r.get(7)?,
        restored_from: r.get(8)?,
    })
}
const FIELDS: &str = "revision_id,version_number,parent_revision_id,created_at,description,schema_version,document_id,content_hash,restored_from";
impl Storage {
    pub fn document_directory(&self) -> Result<DocumentDirectory, StorageError> {
        let folders=self.connection.prepare("SELECT id,parent_id,name FROM document_folders ORDER BY name COLLATE NOCASE,id LIMIT 1001")?.query_map([],|r|Ok(DocumentFolder{id:r.get(0)?,parent_id:r.get(1)?,name:r.get(2)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let documents=self.connection.prepare("SELECT d.id,d.name,o.folder_id,coalesce(o.is_favorite,0),coalesce(o.last_opened_at,0),d.path FROM markdown_documents d LEFT JOIN document_organization o ON o.document_id=d.id ORDER BY d.name COLLATE NOCASE,d.id LIMIT 5001")?.query_map([],|r|{let p:String=r.get(5)?;Ok(OrganizedDocument{id:r.get(0)?,name:r.get(1)?,folder_id:r.get(2)?,favorite:r.get(3)?,last_opened_at:r.get(4)?,available:Path::new(&p).is_file()&&super::files::ordinary(Path::new(&p)).is_ok()})})?.collect::<rusqlite::Result<Vec<_>>>()?;
        if folders.len() > 1000 || documents.len() > 5000 {
            return Err(StorageError::Invalid(
                "目录超过1000文件夹或5000文档，请先整理已登记资料。",
            ));
        }
        Ok(DocumentDirectory { folders, documents })
    }
    pub fn create_document_folder(
        &mut self,
        parent: Option<&str>,
        name: &str,
    ) -> Result<String, StorageError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(StorageError::Invalid(
                "文件夹名称须为1–80字且不能有控制字符。",
            ));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count: i64 = tx.query_row("SELECT count(*) FROM document_folders", [], |r| r.get(0))?;
        if count >= 1000 {
            return Err(StorageError::Invalid("最多1000个逻辑文件夹。"));
        }
        let mut current = parent.map(str::to_owned);
        let mut depth = 0;
        while let Some(id) = current {
            check_id(&id)?;
            depth += 1;
            if depth >= 8 {
                return Err(StorageError::Invalid("目录最多8层。"));
            }
            current = tx
                .query_row(
                    "SELECT parent_id FROM document_folders WHERE id=?1",
                    [id],
                    |r| r.get::<_, Option<String>>(0),
                )
                .optional()?
                .ok_or(StorageError::NotFound)?;
        }
        let id: String = tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
        tx.execute(
            "INSERT INTO document_folders VALUES(?1,?2,?3)",
            params![id, parent, name],
        )?;
        tx.commit()?;
        Ok(id)
    }
    pub fn delete_document_folder(&mut self, id: &str) -> Result<(), StorageError> {
        check_id(id)?;
        if self
            .connection
            .execute("DELETE FROM document_folders WHERE id=?1", [id])?
            != 1
        {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
    pub fn organize_document(
        &mut self,
        id: &str,
        folder: Option<&str>,
        favorite: bool,
    ) -> Result<(), StorageError> {
        check_id(id)?;
        if let Some(folder) = folder {
            check_id(folder)?;
        }
        if self.connection.execute(
            "UPDATE document_organization SET folder_id=?1,is_favorite=?2 WHERE document_id=?3",
            params![folder, favorite, id],
        )? != 1
        {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
    pub fn markdown_history(
        &self,
        id: &str,
        before: i64,
    ) -> Result<Vec<MarkdownRevision>, StorageError> {
        check_id(id)?;
        if before < 1 {
            return Err(StorageError::Invalid("历史分页无效。"));
        }
        Ok(self.connection.prepare(&format!("SELECT {FIELDS} FROM markdown_revisions WHERE document_id=?1 AND version_number<?2 ORDER BY version_number DESC LIMIT 50"))?.query_map(params![id,before],revision)?.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn markdown_revision(&self, id: &str, rid: &str) -> Result<RevisionContent, StorageError> {
        check_id(id)?;
        check_id(rid)?;
        let (revision,markdown,assets):(MarkdownRevision,String,String)=self.connection.query_row(&format!("SELECT {FIELDS},CASE WHEN length(CAST(markdown AS BLOB))<=2097152 THEN markdown ELSE '' END,CASE WHEN length(CAST(assets_json AS BLOB))<=16384 THEN assets_json ELSE '' END FROM markdown_revisions WHERE document_id=?1 AND revision_id=?2"),params![id,rid],|r|Ok((revision(r)?,r.get(9)?,r.get(10)?))).optional()?.ok_or(StorageError::NotFound)?;
        if revision.metadata.schema_version != 1
            || format!("{:x}", Sha256::digest(markdown.as_bytes())) != revision.content_hash
        {
            return Err(StorageError::Invalid("历史正文摘要或版本不支持，未恢复。"));
        }
        let assets: BTreeMap<String, RevisionAsset> = serde_json::from_str(&assets)
            .map_err(|_| StorageError::Invalid("历史图片清单损坏。"))?;
        if assets.len() > 32
            || assets
                .keys()
                .any(|n| !crate::services::markdown_document::asset_name(n))
        {
            return Err(StorageError::Invalid("历史图片清单超出范围。"));
        }
        Ok(RevisionContent {
            revision,
            markdown,
            assets,
        })
    }
    pub fn document_location(&self, id: &str) -> Result<(PathBuf, String), StorageError> {
        check_id(id)?;
        self.connection.query_row("SELECT d.path,f.content_hash FROM markdown_documents d JOIN research_document_fingerprints f ON f.document_id=d.id WHERE d.id=?1",[id],|r|Ok((PathBuf::from(r.get::<_,String>(0)?),r.get(1)?))).optional()?.ok_or(StorageError::NotFound)
    }
}

#[cfg(test)]
mod tests;
