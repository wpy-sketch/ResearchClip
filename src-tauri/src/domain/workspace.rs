use serde::Serialize;

/// IPC contract: keep in sync with src/shared/types/workspace.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStatus {
    pub app_name: &'static str,
    pub app_version: &'static str,
    pub platform: &'static str,
    pub data_dir: String,
    pub database_path: String,
    pub schema_version: u32,
    pub sqlite_version: String,
}
