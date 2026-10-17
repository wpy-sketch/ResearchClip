use super::{CommandError, library::LibraryService};
use crate::{
    capture::CaptureError,
    services::{
        image_import::{self, MAX_BATCH, MAX_HEADER},
        screen_capture::{ScreenCaptureService, encode_png},
    },
    storage::library::Screenshot,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{
    State, WebviewWindow,
    ipc::{InvokeBody, Request},
};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn choose_comparison_image(
    window: WebviewWindow,
    app: tauri::AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
) -> Result<tauri::ipc::Response, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::Read;
        let _permit = permit;
        let Some(path) = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("选择对比图片")
            .add_filter("图片", &["png", "jpg", "jpeg", "webp"])
            .blocking_pick_file()
        else {
            return Ok(tauri::ipc::Response::new(Vec::<u8>::new()));
        };
        let path = path
            .into_path()
            .map_err(|_| image_import::invalid("请选择本地图片。"))?;
        crate::storage::files::ordinary(&path)
            .map_err(|_| image_import::invalid("所选图片不可访问或不是安全的普通文件。"))?;
        let file =
            std::fs::File::open(&path).map_err(|_| image_import::invalid("无法打开所选图片。"))?;
        let meta = file
            .metadata()
            .map_err(|_| image_import::invalid("无法读取图片大小。"))?;
        if !meta.is_file() || meta.len() == 0 || meta.len() > image_import::MAX_FILE as u64 {
            return Err(image_import::invalid("单张图片需非空且不超过20 MiB。"));
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| image_import::invalid("图片文件名无效。"))?;
        let mut bytes = Vec::new();
        file.take(image_import::MAX_FILE as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| image_import::invalid("图片读取失败。"))?;
        let frame = image_import::decode_file(name, &bytes)?;
        let format = match image::guess_format(&bytes).ok() {
            Some(image::ImageFormat::Png) => "PNG",
            Some(image::ImageFormat::Jpeg) => "JPEG",
            _ => "WebP",
        };
        let png = encode_png(frame)?;
        let header = serde_json::to_vec(&serde_json::json!({"name":name,"format":format}))
            .map_err(|_| image_import::invalid("图片信息编码失败。"))?;
        let mut body = (header.len() as u32).to_le_bytes().to_vec();
        body.extend(header);
        body.extend(png);
        Ok(tauri::ipc::Response::new(body))
    })
    .await
    .map_err(|_| image_import::invalid("图片选择任务异常，请重试；源图片未修改。"))?
}
#[derive(Serialize)]
pub struct ImportItem {
    name: String,
    screenshot: Option<Screenshot>,
    error: Option<String>,
}
#[derive(Serialize)]
pub struct ImportReport {
    items: Vec<ImportItem>,
}
fn check(window: &WebviewWindow) -> Result<(), CaptureError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(image_import::invalid("此窗口不能导入外部图片。"))
    }
}
fn store(
    service: &LibraryService,
    title: &str,
    frame: crate::capture::RawFrame,
) -> Result<Screenshot, CommandError> {
    let document = serde_json::json!({"schemaVersion":1,"image":{"widthPx":frame.width_px,"heightPx":frame.height_px},"objects":[]});
    let png = encode_png(frame).map_err(|e| CommandError {
        code: "IMAGE_ENCODE",
        message: e.message,
    })?;
    service.with(|s| s.create_screenshot(title, document, &png))
}
#[tauri::command]
pub async fn import_clipboard_image(
    window: WebviewWindow,
    captures: State<'_, Arc<ScreenCaptureService>>,
    service: State<'_, Arc<LibraryService>>,
) -> Result<Screenshot, CommandError> {
    let wrap = |e: CaptureError| CommandError {
        code: "IMAGE_IMPORT",
        message: e.message,
    };
    check(&window).map_err(wrap)?;
    let permit = captures.try_begin().map_err(wrap)?;
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let frame = image_import::clipboard_frame().map_err(wrap)?;
        store(&service, "剪贴板图片", frame)
    })
    .await
    .map_err(|_| CommandError {
        code: "IMPORT_TASK",
        message: "图片导入任务异常，请检查素材库状态后重试；已有资料未重置。".into(),
    })?
}
#[tauri::command]
pub async fn import_image_files(
    window: WebviewWindow,
    captures: State<'_, Arc<ScreenCaptureService>>,
    service: State<'_, Arc<LibraryService>>,
    request: Request<'_>,
) -> Result<ImportReport, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let bytes = match request.body() {
        InvokeBody::Raw(b) if b.len() <= 4 + MAX_HEADER + MAX_BATCH => b.clone(),
        _ => return Err(image_import::invalid("导入封包超过64 MiB上限。")),
    };
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let (batch, files) = image_import::unpack(&bytes)?;
        let mut items = Vec::new();
        for (meta, bytes) in batch.files.iter().zip(files) {
            let result = image_import::decode_file(&meta.name, bytes)
                .map_err(|e| e.message)
                .and_then(|frame| {
                    let title = image_import::title(&meta.name).map_err(|e| e.message)?;
                    store(&service, &title, frame).map_err(|e| e.message)
                });
            let (screenshot, error) = match result {
                Ok(s) => (Some(s), None),
                Err(e) => (None, Some(e)),
            };
            // Only a filename is returned, never an external path or pixels in an error message.
            let name = if meta.name.contains(['/', '\\']) {
                "无效文件名".into()
            } else {
                meta.name
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(200)
                    .collect()
            };
            items.push(ImportItem {
                name,
                screenshot,
                error,
            });
        }
        Ok(ImportReport { items })
    })
    .await
    .map_err(|_| {
        image_import::invalid("图片导入任务异常；部分素材可能已保存，请刷新素材库后重试。")
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_images_and_thumbnails_survive_restart_duplicates_are_independent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("workspace");
        let service = LibraryService::new(root.clone());
        let frame = || crate::capture::RawFrame {
            width_px: 2,
            height_px: 1,
            rgba: vec![1, 2, 3, 128, 4, 5, 6, 255],
        };
        let a = store(&service, "相同标题", frame()).unwrap();
        let b = store(&service, "相同标题", frame()).unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(
            std::fs::read(root.join(&a.original_image_path)).unwrap(),
            std::fs::read(root.join(&b.original_image_path)).unwrap()
        );
        assert!(root.join(&a.thumbnail_path).is_file());
        drop(service);
        LibraryService::new(root)
            .with(|s| {
                assert_eq!(s.get_screenshot(&a.id)?.title, "相同标题");
                assert_eq!(s.get_screenshot(&b.id)?.width_px, 2);
                Ok(())
            })
            .unwrap();
    }
}
