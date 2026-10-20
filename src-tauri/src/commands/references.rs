use super::{
    CommandError,
    library::{LibraryService, check, run},
};
use crate::{
    services::references::{self, ReferenceInput},
    storage::references::{Reference, ReferencePage, ReferenceTarget, ReferenceWrite},
};
use std::{
    io::Read,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
#[derive(Default)]
pub struct ReferenceNetwork {
    gate: Mutex<Option<Instant>>,
}
fn failure(message: impl Into<String>) -> CommandError {
    CommandError {
        code: "REFERENCE_ERROR",
        message: message.into(),
    }
}
impl ReferenceNetwork {
    fn lookup(&self, doi: &str) -> Result<ReferenceInput, CommandError> {
        let doi = references::normalize_doi(doi).map_err(failure)?;
        if doi.is_empty() {
            return Err(failure("请先输入 DOI。"));
        }
        let mut last = self
            .gate
            .try_lock()
            .map_err(|_| failure("已有 DOI 查询正在处理，请稍后重试。"))?;
        if last.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
            return Err(failure("查询过于频繁，请稍候2秒后重试。"));
        }
        *last = Some(Instant::now());
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .user_agent("ResearchClip/3.1.3 (user-initiated DOI lookup)")
            .build()
            .map_err(|_| failure("无法初始化 DOI 查询。"))?;
        let mut url = reqwest::Url::parse("https://api.crossref.org/works/")
            .map_err(|_| failure("DOI 服务地址无效。"))?;
        url.path_segments_mut()
            .map_err(|_| failure("DOI 服务地址无效。"))?
            .push(&doi);
        let mut response = client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .map_err(|_| {
                failure("Crossref 连接失败或超时。离线笔记仍可使用，也可手工填写文献。")
            })?;
        match response.status().as_u16() {
            200 => {}
            404 => return Err(failure("Crossref 未找到此 DOI，请核对或手工填写。")),
            429 => {
                *last = Some(Instant::now() + Duration::from_secs(58));
                return Err(failure("Crossref 请求限流，请至少60秒后重试或手工填写。"));
            }
            _ => {
                return Err(failure(
                    "Crossref 暂不可用或返回异常状态，请稍后重试；已有资料未修改。",
                ));
            }
        }
        if response.content_length().is_some_and(|n| n > 262144) {
            return Err(failure("Crossref 响应超过256 KiB限制。"));
        }
        let mut bytes = vec![];
        response
            .by_ref()
            .take(262145)
            .read_to_end(&mut bytes)
            .map_err(|_| failure("Crossref 响应读取失败。"))?;
        if bytes.len() > 262144 {
            return Err(failure("Crossref 响应过大。"));
        }
        let value =
            serde_json::from_slice(&bytes).map_err(|_| failure("Crossref 返回无效 JSON。"))?;
        references::crossref(&value, &doi).map_err(failure)
    }
}
#[tauri::command]
pub async fn lookup_reference_doi(
    window: WebviewWindow,
    network: State<'_, Arc<ReferenceNetwork>>,
    doi: String,
) -> Result<ReferenceInput, CommandError> {
    check(&window)?;
    let network = Arc::clone(network.inner());
    tauri::async_runtime::spawn_blocking(move || network.lookup(&doi))
        .await
        .map_err(|_| failure("DOI 查询异常结束，请重试。"))?
}
#[tauri::command]
pub async fn reference_page(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    query: String,
    page: u32,
    target: Option<ReferenceTarget>,
) -> Result<ReferencePage, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.reference_page(&query, page, target.as_ref())
    })
    .await
}
#[tauri::command]
pub async fn save_references(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    items: Vec<ReferenceWrite>,
) -> Result<Vec<Reference>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.save_references(items)
    })
    .await
}
#[tauri::command]
pub async fn associate_reference(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    target: ReferenceTarget,
    id: String,
    linked: bool,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.associate_reference(&target, &id, linked)
    })
    .await
}
#[tauri::command]
pub async fn import_bibtex(
    window: WebviewWindow,
    app: AppHandle,
) -> Result<Option<Vec<ReferenceInput>>, CommandError> {
    check(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_title("导入 BibTeX 文献")
            .add_filter("BibTeX", &["bib"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file
            .into_path()
            .map_err(|_| failure("请选择本地 .bib 文件。"))?;
        crate::storage::files::ordinary(&path)
            .map_err(|_| failure("不允许读取重解析或非常规 BibTeX 文件。"))?;
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("bib"))
        {
            return Err(failure("只接受 .bib 文件。"));
        }
        let file = std::fs::File::open(&path).map_err(|_| failure("无法读取 BibTeX 文件。"))?;
        if !file
            .metadata()
            .map_err(|_| failure("无法读取文件信息。"))?
            .is_file()
        {
            return Err(failure("请选择普通文件。"));
        }
        let mut bytes = vec![];
        file.take((references::MAX_BIB_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| failure("BibTeX 读取失败。"))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| failure("BibTeX 需要 UTF-8 编码，请转换后重试。"))?;
        references::import_bib(&text).map(Some).map_err(failure)
    })
    .await
    .map_err(|_| failure("BibTeX 导入预览失败；未修改资料。"))?
}
#[tauri::command]
pub async fn open_reference_doi(
    window: WebviewWindow,
    app: AppHandle,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<(), CommandError> {
    check(&window)?;
    let service = Arc::clone(service.inner());
    let doi = tauri::async_runtime::spawn_blocking(move || service.with(|s| s.reference_doi(&id)))
        .await
        .map_err(|_| failure("无法读取文献。"))??;
    let mut url = reqwest::Url::parse("https://doi.org/").map_err(|_| failure("DOI 地址无效。"))?;
    url.path_segments_mut()
        .map_err(|_| failure("DOI 地址无效。"))?
        .push(&doi);
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|_| failure("无法打开 DOI，请检查系统默认浏览器。"))
}
