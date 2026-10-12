use crate::{
    domain::{ConnectionTestResponse, WorkspaceStatus},
    storage::Storage,
};
use serde::Serialize;
use tauri::Manager;

#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}

#[tauri::command]
pub fn test_connection() -> ConnectionTestResponse {
    ConnectionTestResponse {
        message: "ResearchClip：Rust 后端连接成功。",
        app_version: env!("CARGO_PKG_VERSION"),
    }
}

// Preserve the earlier storage diagnostic without opening or modifying a DB on startup.
#[tauri::command]
pub fn get_workspace_status(app: tauri::AppHandle) -> Result<WorkspaceStatus, CommandError> {
    let data_dir = app.path().app_data_dir().map_err(|_| CommandError {
        code: "DATA_DIR",
        message: "无法解析本地工作区目录".into(),
    })?;
    let storage = Storage::open(data_dir).map_err(|_| CommandError {
        code: "STORAGE_OPEN",
        message: "无法打开本地工作区，请检查目录权限与数据库版本".into(),
    })?;
    storage.status().map_err(|_| CommandError {
        code: "STORAGE_READ",
        message: "无法读取本地工作区".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_response_matches_the_frontend_contract() {
        let value = serde_json::to_value(test_connection()).unwrap();
        assert_eq!(value["appVersion"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["message"], "ResearchClip：Rust 后端连接成功。");
        assert!(value.get("app_version").is_none());
    }
}
