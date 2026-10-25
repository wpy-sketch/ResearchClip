use super::StorageError;
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionMetadata {
    pub revision_id: String,
    pub version_number: i64,
    pub parent_revision_id: Option<String>,
    pub created_at: i64,
    pub description: String,
    pub schema_version: i64,
}
pub fn validate_description(value: &str) -> Result<(), StorageError> {
    if value.chars().count() > 500 || value.contains('\0') {
        return Err(StorageError::Invalid(
            "版本说明最多500字符，不能包含空字符。",
        ));
    }
    Ok(())
}
impl From<&super::revisions::AnnotationRevision> for RevisionMetadata {
    fn from(r: &super::revisions::AnnotationRevision) -> Self {
        Self {
            revision_id: r.revision_id.clone(),
            version_number: r.version_number,
            parent_revision_id: r.parent_revision_id.clone(),
            created_at: r.created_at,
            description: r.description.clone(),
            schema_version: r.schema_version,
        }
    }
}
