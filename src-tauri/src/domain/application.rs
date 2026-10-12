use serde::Serialize;

/// IPC contract: src/shared/types/application.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResponse {
    pub message: &'static str,
    pub app_version: &'static str,
}
