use crate::domain::{ConnectionTestResponse, WorkspaceStatus};
use serde::Serialize;
use std::sync::Arc;
use tauri::{State, WebviewWindow};
pub mod capture;
pub mod desktop;
pub mod document;
pub mod editor;
pub mod import;
pub mod library;
pub mod markdown;
pub mod ocr;
pub mod pins;
pub mod region;
pub mod scrolling;
pub mod workflow;

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

#[tauri::command]
pub fn test_connection() -> ConnectionTestResponse {
    ConnectionTestResponse {
        message: "ResearchClip：Rust 后端连接成功。",
        app_version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
pub async fn get_workspace_status(
    window: WebviewWindow,
    service: State<'_, Arc<library::LibraryService>>,
) -> Result<WorkspaceStatus, CommandError> {
    if window.label() != "main" {
        return Err(CommandError {
            code: "WINDOW_DENIED",
            message: "该窗口无权读取工作区。".into(),
        });
    }
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || service.with(|s| s.status()))
        .await
        .map_err(|_| CommandError {
            code: "STORAGE_TASK",
            message: "无法读取工作区。".into(),
        })?
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

pub mod clipper;
pub mod projects;

pub mod ocr_index;

pub mod math_ocr;
