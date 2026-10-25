use super::{
    CommandError,
    library::{LibraryService, check, run},
};
use crate::{
    services::references::{ReferenceInput, import_bib},
    storage::{citations::ExternalCitationKey, references::Reference},
};
use std::sync::Arc;
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
fn fail(message: &str) -> CommandError {
    CommandError {
        code: "CITATION_ERROR",
        message: message.into(),
    }
}
#[tauri::command]
pub async fn citation_references(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    ids: Vec<String>,
) -> Result<Vec<Reference>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.citation_references(&ids)
    })
    .await
}
#[tauri::command]
pub async fn preview_bibtex(
    window: WebviewWindow,
    text: String,
) -> Result<Vec<ReferenceInput>, CommandError> {
    check(&window)?;
    import_bib(&text).map_err(|e| fail(&e))
}
#[tauri::command]
pub async fn resolve_citation_key(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    key: ExternalCitationKey,
) -> Result<Option<String>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.citation_key_mapping(&key)
    })
    .await
}
#[tauri::command]
pub async fn bind_citation_key(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    key: ExternalCitationKey,
    id: String,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.bind_citation_key(&key, &id)
    })
    .await
}
#[tauri::command]
pub async fn export_bibtex(
    window: WebviewWindow,
    app: AppHandle,
    service: State<'_, Arc<LibraryService>>,
    ids: Vec<String>,
) -> Result<bool, CommandError> {
    check(&window)?;
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let text = service.with(|s| s.citation_bibtex(&ids))?;
        let Some(file) = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("导出本地BibTeX（已有文件不会覆盖）")
            .set_file_name("researchclip-references.bib")
            .add_filter("BibTeX", &["bib"])
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let path = file.into_path().map_err(|_| fail("请选择本地.bib目标。"))?;
        export_new_bib(&path, &text)?;
        Ok(true)
    })
    .await
    .map_err(|_| fail("BibTeX导出任务失败；已有文件保留。"))?
}
pub fn export_new_bib(path: &std::path::Path, text: &str) -> Result<(), CommandError> {
    use std::io::Write;
    if !path
        .extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("bib"))
        || text.len() > crate::services::references::MAX_BIB_BYTES
    {
        return Err(fail("目标须为.bib，内容最多1MiB。"));
    }
    if let Some(parent) = path.parent() {
        crate::storage::files::ordinary(parent).map_err(|_| fail("导出目录不受支持。"))?;
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| fail("目标已存在或无法创建；已有文件未覆盖。"))?;
    f.write_all(text.as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|_| fail("BibTeX写入未完成；请保留当前资料并另选目标。"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_never_overwrites_and_preserves_utf8() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("文献.bib");
        let text = "@misc{local,title={中文练习}}";
        export_new_bib(&p, text).unwrap();
        assert!(export_new_bib(&p, "replace").is_err());
        assert_eq!(std::fs::read_to_string(p).unwrap(), text);
        assert!(export_new_bib(&t.path().join("wrong.txt"), text).is_err());
    }
}
