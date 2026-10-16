use crate::{
    capture::{CaptureError, ErrorCode},
    services::{
        region_capture::RegionCaptureService,
        screen_capture::ScreenCaptureService,
        settings::{PreferencesPatch, SettingsService, SettingsSnapshot, ShortcutAction, shortcut},
        workflow::{CaptureInfo, CaptureWorkflow, decode_png, export_png_new},
    },
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, ipc::Response};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

fn main_only(window: &WebviewWindow) -> Result<(), CaptureError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(CaptureError::new(
            ErrorCode::WindowState,
            "该窗口无权操作截图和设置。",
        ))
    }
}
fn task_error() -> CaptureError {
    CaptureError::new(ErrorCode::Internal, "操作任务异常结束，请重试。")
}
pub(crate) fn registration(app: &AppHandle, key: &str, enable: bool) -> Result<(), CaptureError> {
    let key = shortcut(key)?;
    if enable {
        app.global_shortcut().register(key).map_err(|_| {
            CaptureError::new(
                ErrorCode::ShortcutConflict,
                "截图快捷键注册失败，可能已被其他应用占用。请在设置中选择其他组合。",
            )
        })
    } else {
        app.global_shortcut().unregister(key).map_err(|_| {
            CaptureError::new(
                ErrorCode::SettingsUnavailable,
                "无法注销旧快捷键，修改未生效，请重试或重新启动应用。",
            )
        })
    }
}
#[tauri::command]
pub async fn get_preferences(
    window: WebviewWindow,
    app: AppHandle,
    settings: State<'_, Arc<SettingsService>>,
) -> Result<SettingsSnapshot, CaptureError> {
    main_only(&window)?;
    let settings = Arc::clone(settings.inner());
    tauri::async_runtime::spawn_blocking(move || {
        settings.initialize(|k, e| registration(&app, k, e))?;
        settings.snapshot()
    })
    .await
    .map_err(|_| task_error())?
}
#[tauri::command]
pub async fn update_preferences(
    window: WebviewWindow,
    app: AppHandle,
    settings: State<'_, Arc<SettingsService>>,
    patch: PreferencesPatch,
) -> Result<SettingsSnapshot, CaptureError> {
    main_only(&window)?;
    let settings = Arc::clone(settings.inner());
    tauri::async_runtime::spawn_blocking(move || {
        settings.update(patch, None, |k, e| registration(&app, k, e))
    })
    .await
    .map_err(|_| task_error())?
}
#[tauri::command]
pub async fn choose_save_directory(
    window: WebviewWindow,
    app: AppHandle,
    settings: State<'_, Arc<SettingsService>>,
    captures: State<'_, Arc<ScreenCaptureService>>,
) -> Result<Option<SettingsSnapshot>, CaptureError> {
    main_only(&window)?;
    let permit = captures.try_begin()?;
    let settings = Arc::clone(settings.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let mut dialog = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("选择截图默认保存目录");
        if let Some(dir) = settings.snapshot()?.preferences.save_directory {
            dialog = dialog.set_directory(dir);
        }
        let Some(selected) = dialog.blocking_pick_folder() else {
            return Ok(None);
        };
        let path = selected
            .into_path()
            .map_err(|_| CaptureError::new(ErrorCode::SaveFailed, "只能选择本地文件夹。"))?;
        if !path.is_dir() {
            return Err(CaptureError::new(
                ErrorCode::SaveFailed,
                "所选目录不可用，请选择其他文件夹。",
            ));
        }
        settings
            .update(
                PreferencesPatch::default(),
                Some(path.to_string_lossy().into()),
                |k, e| registration(&app, k, e),
            )
            .map(Some)
    })
    .await
    .map_err(|_| task_error())?
}
#[tauri::command]
pub fn capture_result_png(
    window: WebviewWindow,
    workflow: State<'_, Arc<CaptureWorkflow>>,
    capture_id: String,
) -> Result<Response, CaptureError> {
    main_only(&window)?;
    Ok(Response::new(workflow.get(&capture_id)?.png.clone()))
}
#[tauri::command]
pub fn current_capture(
    window: WebviewWindow,
    workflow: State<'_, Arc<CaptureWorkflow>>,
) -> Result<Option<CaptureInfo>, CaptureError> {
    main_only(&window)?;
    Ok(workflow.current())
}
#[tauri::command]
pub fn discard_capture(
    window: WebviewWindow,
    workflow: State<'_, Arc<CaptureWorkflow>>,
    capture_id: String,
) -> Result<(), CaptureError> {
    main_only(&window)?;
    workflow.discard(&capture_id)
}
#[tauri::command]
pub async fn copy_capture_image(
    window: WebviewWindow,
    app: AppHandle,
    workflow: State<'_, Arc<CaptureWorkflow>>,
    captures: State<'_, Arc<ScreenCaptureService>>,
    capture_id: String,
) -> Result<(), CaptureError> {
    main_only(&window)?;
    let permit = captures.try_begin()?;
    let resource = workflow.get(&capture_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let frame = decode_png(&resource.png)?;
        app.clipboard()
            .write_image(&tauri::image::Image::new(
                &frame.rgba,
                frame.width_px,
                frame.height_px,
            ))
            .map_err(|_| {
                CaptureError::new(
                    ErrorCode::ClipboardUnavailable,
                    "无法写入图片剪贴板，可能正被其他应用占用。请稍后重试。",
                )
            })
    })
    .await
    .map_err(|_| task_error())?
}
#[tauri::command]
pub async fn save_capture_png(
    window: WebviewWindow,
    app: AppHandle,
    workflow: State<'_, Arc<CaptureWorkflow>>,
    settings: State<'_, Arc<SettingsService>>,
    captures: State<'_, Arc<ScreenCaptureService>>,
    capture_id: String,
) -> Result<Option<String>, CaptureError> {
    main_only(&window)?;
    let permit = captures.try_begin()?;
    let resource = workflow.get(&capture_id)?;
    let directory = settings.snapshot()?.preferences.save_directory;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let mut dialog = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("保存截图 PNG")
            .add_filter("PNG 图片", &["png"])
            .set_file_name(format!("ResearchClip-{}.png", resource.info.capture_id));
        if let Some(dir) = directory {
            if !std::path::Path::new(&dir).is_dir() {
                return Err(CaptureError::new(
                    ErrorCode::SaveFailed,
                    "默认保存目录已不可用，请先在设置中重新选择目录。",
                ));
            }
            dialog = dialog.set_directory(dir);
        } else if let Ok(dir) = app.path().picture_dir() {
            dialog = dialog.set_directory(dir);
        }
        let Some(selected) = dialog.blocking_save_file() else {
            return Ok(None);
        };
        let path = selected
            .into_path()
            .map_err(|_| CaptureError::new(ErrorCode::SaveFailed, "请选择本地 PNG 文件。"))?;
        export_png_new(&path, &resource.png)?;
        Ok(Some(path.to_string_lossy().into()))
    })
    .await
    .map_err(|_| task_error())?
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum WorkflowEvent {
    Started,
    Ready { capture: CaptureInfo },
    Failed { code: ErrorCode, message: String },
}
pub(crate) fn shortcut_capture(app: AppHandle, action: ShortcutAction) {
    let captures = Arc::clone(app.state::<Arc<ScreenCaptureService>>().inner());
    let Ok(permit) = captures.try_begin() else {
        return;
    };
    // All native work runs in Rust even when the main webview is minimized.
    tauri::async_runtime::spawn(async move {
        let regions = Arc::clone(app.state::<Arc<RegionCaptureService>>().inner());
        let Some(window) = app.get_webview_window("main") else {
            return;
        };
        let result = async {
            let displays = tauri::async_runtime::spawn_blocking(crate::capture::enumerate_displays)
                .await
                .map_err(|_| task_error())??;
            let id = displays
                .iter()
                .find(|d| d.is_primary)
                .or(displays.first())
                .ok_or_else(|| CaptureError::new(ErrorCode::NoDisplays, "没有可用显示器。"))?
                .id
                .clone();
            let _ = app.emit_to("main", "capture-workflow", WorkflowEvent::Started);
            match action {
                ShortcutAction::Region => {
                    super::region::run_region_capture(
                        window.clone(),
                        app.clone(),
                        permit,
                        regions,
                        id,
                    )
                    .await
                }
                ShortcutAction::Fullscreen => {
                    super::capture::run_fullscreen_capture(
                        window.clone(),
                        Arc::clone(&captures),
                        Arc::clone(app.state::<Arc<CaptureWorkflow>>().inner()),
                        permit,
                        id,
                    )
                    .await
                }
            }
        }
        .await;
        let event = match result {
            Ok(capture) => WorkflowEvent::Ready { capture },
            Err(error) if error.code == ErrorCode::Busy => return,
            Err(error) => WorkflowEvent::Failed {
                code: error.code,
                message: error.message,
            },
        };
        if (matches!(event, WorkflowEvent::Ready { .. })
            || matches!(&event,WorkflowEvent::Failed{code,..} if *code!=ErrorCode::Cancelled))
            && window
                .show()
                .and_then(|_| window.unminimize())
                .and_then(|_| window.set_focus())
                .is_err()
        {
            eprintln!("ResearchClip: captured result window focus failed");
        }
        if app.emit_to("main", "capture-workflow", event).is_err() {
            eprintln!("ResearchClip: capture result notification failed");
        }
    });
}
