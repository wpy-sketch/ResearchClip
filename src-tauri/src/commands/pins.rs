use crate::{
    capture::{CaptureError, ErrorCode, MAX_PNG_BYTES},
    services::{
        desktop::DesktopService,
        pins::{PinImage, PinInfo, PinManager, PinStatus, failure},
        screen_capture::ScreenCaptureService,
        settings::SettingsService,
        workflow::{decode_png, export_png_new},
    },
};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{
    AppHandle, Manager, State, WebviewWindow,
    ipc::{InvokeBody, Request, Response},
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
fn main_only(window: &WebviewWindow) -> Result<(), CaptureError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(failure("该窗口无权创建或管理其他贴图。"))
    }
}
#[tauri::command]
pub async fn create_pin(
    window: WebviewWindow,
    app: AppHandle,
    request: Request<'_>,
    pins: State<'_, Arc<PinManager>>,
) -> Result<String, CaptureError> {
    main_only(&window)?;
    if !app.state::<DesktopService>().running() {
        return Err(failure("正在退出应用，无法创建贴图。"));
    }
    let permit = app.state::<Arc<ScreenCaptureService>>().try_begin()?;
    let InvokeBody::Raw(data) = request.body() else {
        return Err(failure("贴图必须使用 PNG 二进制数据。"));
    };
    if data.is_empty() || data.len() > MAX_PNG_BYTES {
        return Err(CaptureError::new(
            ErrorCode::ResourceLimit,
            "贴图 PNG 为空或超过资源上限。",
        ));
    }
    let png = data.clone();
    let pins = Arc::clone(pins.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let frame = decode_png(&png)?;
        let (width, height) = (frame.width_px, frame.height_px);
        drop(frame);
        pins.create(&app, PinImage { png, width, height })
    })
    .await
    .map_err(|_| failure("贴图创建任务异常结束。"))?
}
#[tauri::command]
pub async fn pin_image(
    window: WebviewWindow,
    pins: State<'_, Arc<PinManager>>,
) -> Result<Response, CaptureError> {
    Ok(Response::new(pins.image(window.label())?.png.clone()))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PinAction {
    Ready,
    Zoom,
    Opacity,
    Drag,
    Close,
    Copy,
    Save,
    Recover,
}
#[tauri::command]
pub async fn pin_action(
    window: WebviewWindow,
    app: AppHandle,
    pins: State<'_, Arc<PinManager>>,
    action: PinAction,
    value: Option<f64>,
) -> Result<Option<PinInfo>, CaptureError> {
    let image = pins.image(window.label())?;
    if matches!(action, PinAction::Copy | PinAction::Save) {
        let permit = app.state::<Arc<ScreenCaptureService>>().try_begin()?;
        let window = window.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            if matches!(action, PinAction::Copy) {
                let frame = decode_png(&image.png)?;
                app.clipboard()
                    .write_image(&tauri::image::Image::new(
                        &frame.rgba,
                        frame.width_px,
                        frame.height_px,
                    ))
                    .map_err(|_| {
                        CaptureError::new(
                            ErrorCode::ClipboardUnavailable,
                            "剪贴板不可用，请关闭占用剪贴板的程序后重试。",
                        )
                    })?;
            } else {
                let mut dialog = app
                    .dialog()
                    .file()
                    .set_parent(&window)
                    .set_title("保存贴图 PNG")
                    .add_filter("PNG 图片", &["png"])
                    .set_file_name("ResearchClip-pin.png");
                if let Some(dir) = app
                    .state::<Arc<SettingsService>>()
                    .snapshot()?
                    .preferences
                    .save_directory
                {
                    dialog = dialog.set_directory(dir);
                }
                if let Some(path) = dialog.blocking_save_file() {
                    let path = path
                        .into_path()
                        .map_err(|_| failure("请选择本地 PNG 文件位置。"))?;
                    export_png_new(&path, &image.png)?;
                }
            }
            Ok::<_, CaptureError>(())
        })
        .await
        .map_err(|_| failure("贴图操作任务异常结束。"))??;
    } else {
        match action {
            PinAction::Ready => pins.ready(&window)?,
            PinAction::Zoom => {
                pins.zoom(&window, value.ok_or_else(|| failure("缺少缩放参数。"))?)?
            }
            PinAction::Opacity => {
                pins.opacity(&window, value.ok_or_else(|| failure("缺少透明度参数。"))?)?
            }
            PinAction::Drag => window
                .start_dragging()
                .map_err(|_| failure("无法移动贴图。"))?,
            PinAction::Close => {
                window.destroy().map_err(|_| failure("无法关闭贴图。"))?;
                pins.release(window.label());
                return Ok(None);
            }
            PinAction::Recover => pins.recover(&window)?,
            PinAction::Copy | PinAction::Save => unreachable!(),
        }
    }
    pins.info(window.label()).map(Some)
}
#[tauri::command]
pub async fn recover_pins(
    window: WebviewWindow,
    app: AppHandle,
    pins: State<'_, Arc<PinManager>>,
) -> Result<(), CaptureError> {
    main_only(&window)?;
    pins.recover_all(&app)
}
#[tauri::command]
pub async fn pin_status(
    window: WebviewWindow,
    pins: State<'_, Arc<PinManager>>,
) -> Result<PinStatus, CaptureError> {
    main_only(&window)?;
    pins.status()
}
