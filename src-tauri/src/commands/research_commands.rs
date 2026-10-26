//! Strict metadata gate for local typed application commands. No generic execution or file API.
use super::{CommandError, library::LibraryService};
use crate::{
    domain::research::{
        DocumentType, RESOURCE_FORMAT_VERSION, ResearchData, ResourceKey, ResourceKind,
    },
    storage::resources::ResourceSnapshot,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{State, WebviewWindow};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenArgs {
    resource: ResourceKey,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InsertArgs {
    document: ResourceKey,
    image: ResourceKey,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocationArgs {
    location: ResourceKey,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnnotationType {
    Highlight,
    Comment,
    Ink,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnotationArgs {
    location: ResourceKey,
    annotation_type: AnnotationType,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceArgs {
    workspace: ResourceKey,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditArgs {
    session: ResourceKey,
    document: ResourceKey,
    expected_revision: i64,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "name", content = "args", deny_unknown_fields)]
pub enum ApplicationCommand {
    #[serde(rename = "document.open")]
    Open(OpenArgs),
    #[serde(rename = "markdown.insertImage")]
    Insert(InsertArgs),
    #[serde(rename = "pdf.navigateTo")]
    Navigate(LocationArgs),
    #[serde(rename = "screenshot.capturePdfSelection")]
    Capture(LocationArgs),
    #[serde(rename = "annotation.create")]
    Annotate(AnnotationArgs),
    #[serde(rename = "codeWorkspace.open")]
    Workspace(WorkspaceArgs),
    #[serde(rename = "agent.requestEdit")]
    Edit(EditArgs),
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandRequest {
    version: u32,
    request_id: String,
    command: ApplicationCommand,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveRequest {
    version: u32,
    key: ResourceKey,
}
fn problem(code: &'static str, message: &str) -> CommandError {
    CommandError {
        code,
        message: message.into(),
    }
}
fn window_allowed(label: &str) -> Result<(), CommandError> {
    if label == "main" {
        Ok(())
    } else {
        Err(problem("WINDOW_DENIED", "该窗口无权调用科研命令。"))
    }
}
fn version(value: u32) -> Result<(), CommandError> {
    if value == 1 {
        Ok(())
    } else {
        Err(problem("UNSUPPORTED_VERSION", "科研命令版本不受支持。"))
    }
}
impl ApplicationCommand {
    fn keys(&self) -> Vec<&ResourceKey> {
        match self {
            Self::Open(a) => vec![&a.resource],
            Self::Insert(a) => vec![&a.document, &a.image],
            Self::Navigate(a) | Self::Capture(a) => vec![&a.location],
            Self::Annotate(a) => vec![&a.location],
            Self::Workspace(a) => vec![&a.workspace],
            Self::Edit(a) => vec![&a.session, &a.document],
        }
    }
    fn validate(&self) -> Result<(), CommandError> {
        let invalid = || problem("INVALID_INPUT", "科研命令参数或资源类型无效。");
        for key in self.keys() {
            key.validate().map_err(|_| invalid())?;
        }
        let valid = match self {
            Self::Open(a) => matches!(
                a.resource.kind,
                ResourceKind::Document | ResourceKind::Screenshot | ResourceKind::PdfAttachment
            ),
            Self::Insert(a) => {
                a.document.kind == ResourceKind::Document
                    && a.image.kind == ResourceKind::Screenshot
            }
            Self::Navigate(a) | Self::Capture(a) => a.location.kind == ResourceKind::PdfLocation,
            Self::Annotate(a) => {
                let _ = &a.annotation_type;
                a.location.kind == ResourceKind::PdfLocation
            }
            Self::Workspace(a) => a.workspace.kind == ResourceKind::CodeWorkspace,
            Self::Edit(a) => {
                a.session.kind == ResourceKind::AgentSession
                    && a.document.kind == ResourceKind::Document
                    && a.expected_revision > 0
                    && a.expected_revision <= 9_007_199_254_740_991
            }
        };
        if valid { Ok(()) } else { Err(invalid()) }
    }
    fn available(&self) -> Result<(), CommandError> {
        if matches!(
            self,
            Self::Open(_) | Self::Insert(_) | Self::Navigate(_) | Self::Capture(_)
        ) {
            Ok(())
        } else {
            Err(problem(
                "MODULE_UNAVAILABLE",
                "对应模块尚未安装或启动；未执行任何资源修改。",
            ))
        }
    }
}
impl CommandRequest {
    fn validate(&self) -> Result<(), CommandError> {
        version(self.version)?;
        if self.request_id.len() != 32
            || !self
                .request_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(problem("INVALID_INPUT", "命令请求标识无效。"));
        }
        self.command.validate()
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedResource {
    key: ResourceKey,
    format_version: u32,
    editor: Option<&'static str>,
    module_available: bool,
}
fn describe(resource: ResourceSnapshot) -> Result<ResolvedResource, CommandError> {
    if resource.format_version != RESOURCE_FORMAT_VERSION {
        return Err(problem(
            "UNSUPPORTED_VERSION",
            "资源格式不受支持；原资料已保留。",
        ));
    }
    let editor = if resource.legacy {
        match resource.key.kind {
            ResourceKind::Document => Some("markdown"),
            ResourceKind::Screenshot => Some("image"),
            _ => None,
        }
    } else {
        match resource.data().map_err(super::library::error)? {
            ResearchData::Document(d) => Some(match d.document_type {
                DocumentType::Markdown => "markdown",
                DocumentType::Pdf => "pdf",
                DocumentType::Code => "code",
            }),
            ResearchData::PdfAttachment(_) | ResearchData::PdfLocation(_) => Some("pdf"),
            _ => None,
        }
    };
    Ok(ResolvedResource {
        key: resource.key,
        format_version: RESOURCE_FORMAT_VERSION,
        editor,
        module_available: matches!(editor, Some("markdown" | "image" | "pdf")),
    })
}
#[tauri::command]
pub async fn resolve_research_resource(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    request: ResolveRequest,
) -> Result<ResolvedResource, CommandError> {
    window_allowed(window.label())?;
    version(request.version)?;
    request
        .key
        .validate()
        .map_err(|_| problem("INVALID_INPUT", "资源标识无效。"))?;
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        service
            .access(|s| s.research_resource(&request.key))
            .map_err(super::library::error)
            .and_then(describe)
    })
    .await
    .map_err(|_| problem("OPERATION_FAILED", "资源查询异常，资料未被修改。"))?
}
#[tauri::command]
pub async fn prepare_research_command(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    request: CommandRequest,
) -> Result<Vec<ResolvedResource>, CommandError> {
    window_allowed(window.label())?;
    request.validate()?;
    request.command.available()?;
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        // Keep the same repository lock for both lookups; do not expose catalog JSON or paths.
        service
            .access(|s| {
                request
                    .command
                    .keys()
                    .iter()
                    .map(|k| s.research_resource(k))
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(super::library::error)
            .and_then(|rows| {
                rows.into_iter()
                    .map(describe)
                    .collect::<Result<Vec<_>, _>>()
            })
            .and_then(|resources| {
                let valid = match request.command {
                    ApplicationCommand::Open(_) => resources[0].module_available,
                    ApplicationCommand::Insert(_) => {
                        resources[0].editor == Some("markdown")
                            && resources[1].editor == Some("image")
                    }
                    ApplicationCommand::Navigate(_) | ApplicationCommand::Capture(_) => {
                        resources[0].editor == Some("pdf")
                    }
                    _ => false,
                };
                if valid {
                    Ok(resources)
                } else {
                    Err(problem(
                        "MODULE_UNAVAILABLE",
                        "该资源对应的编辑模块尚未接入。",
                    ))
                }
            })
    })
    .await
    .map_err(|_| problem("OPERATION_FAILED", "命令校验异常，未开始资源操作。"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use serde_json::{Value, json};
    fn key(kind: &str) -> Value {
        json!({"kind":kind,"id":"a".repeat(32)})
    }
    fn request(name: &str, args: Value) -> Value {
        json!({"version":1,"requestId":"b".repeat(32),"command":{"name":name,"args":args}})
    }
    #[test]
    fn seven_examples_validate_and_four_implemented_commands_are_available() {
        for (name, args, available) in [
            ("document.open", json!({"resource":key("document")}), true),
            (
                "markdown.insertImage",
                json!({"document":key("document"),"image":key("screenshot")}),
                true,
            ),
            (
                "pdf.navigateTo",
                json!({"location":key("pdfLocation")}),
                true,
            ),
            (
                "screenshot.capturePdfSelection",
                json!({"location":key("pdfLocation")}),
                true,
            ),
            (
                "annotation.create",
                json!({"location":key("pdfLocation"),"annotationType":"ink"}),
                false,
            ),
            (
                "codeWorkspace.open",
                json!({"workspace":key("codeWorkspace")}),
                false,
            ),
            (
                "agent.requestEdit",
                json!({"session":key("agentSession"),"document":key("document"),"expectedRevision":1}),
                false,
            ),
        ] {
            let r: CommandRequest = serde_json::from_value(request(name, args)).unwrap();
            r.validate().unwrap();
            assert_eq!(r.command.available().is_ok(), available);
        }
    }
    #[test]
    fn rejects_arbitrary_names_unknown_fields_and_caller_privileges() {
        for patch in [
            json!({"actor":"user"}),
            json!({"permissions":["all"]}),
            json!({"sql":"SELECT 1"}),
            json!({"path":"C:/secret"}),
        ] {
            let mut v = request("document.open", json!({"resource":key("document")}));
            v.as_object_mut()
                .unwrap()
                .extend(patch.as_object().unwrap().clone());
            assert!(serde_json::from_value::<CommandRequest>(v).is_err());
        }
        for v in [
            request("shell.execute", json!({})),
            request(
                "document.open",
                json!({"resource":key("document"),"path":"C:/x"}),
            ),
            request(
                "document.open",
                json!({"resource":{"kind":"document","id":"a".repeat(32),"token":"secret"}}),
            ),
            request(
                "annotation.create",
                json!({"location":key("pdfLocation"),"annotationType":"privateStroke"}),
            ),
        ] {
            assert!(serde_json::from_value::<CommandRequest>(v).is_err());
        }
    }
    #[test]
    fn rejects_future_versions_malformed_ids_and_wrong_resource_kinds() {
        let mut v = request("document.open", json!({"resource":key("document")}));
        v["version"] = json!(2);
        assert_eq!(
            serde_json::from_value::<CommandRequest>(v)
                .unwrap()
                .validate()
                .unwrap_err()
                .code,
            "UNSUPPORTED_VERSION"
        );
        for id in ["../x", "AABB", "", &"A".repeat(32)] {
            let mut v = request("document.open", json!({"resource":key("document")}));
            v["command"]["args"]["resource"]["id"] = json!(id);
            assert!(
                serde_json::from_value::<CommandRequest>(v)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        for v in [
            request("document.open", json!({"resource":key("agentSession")})),
            request(
                "markdown.insertImage",
                json!({"document":key("document"),"image":key("annotation")}),
            ),
            request(
                "agent.requestEdit",
                json!({"session":key("agentSession"),"document":key("document"),"expectedRevision":0}),
            ),
        ] {
            assert!(
                serde_json::from_value::<CommandRequest>(v)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
    }
    #[test]
    fn only_runtime_main_window_can_cross_native_gate() {
        for label in ["agent", "pdf", "main-fake", "capture-overlay-1", ""] {
            assert_eq!(window_allowed(label).unwrap_err().code, "WINDOW_DENIED");
        }
        window_allowed("main").unwrap();
    }
    #[test]
    fn resolver_request_is_strict_and_descriptors_omit_private_metadata() {
        assert!(
            serde_json::from_value::<ResolveRequest>(
                json!({"version":1,"key":key("document"),"path":"secret"})
            )
            .is_err()
        );
        let row = ResourceSnapshot {
            key: ResourceKey {
                kind: ResourceKind::Document,
                id: "a".repeat(32),
            },
            title: "private title".into(),
            format_version: 1,
            revision: None,
            created_at: 1,
            updated_at: 1,
            legacy: true,
            metadata: json!({"path":"C:/secret","markdown":"private body","token":"secret"}),
        };
        let json = serde_json::to_string(&describe(row).unwrap()).unwrap();
        assert!(json.contains("markdown"));
        assert!(!json.contains("private") && !json.contains("secret") && !json.contains("path"));
    }
    #[test]
    fn missing_resource_returns_not_found_without_creating_new_identity() {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage::open(directory.path().join("workspace")).unwrap();
        let key = ResourceKey {
            kind: ResourceKind::Document,
            id: "a".repeat(32),
        };
        assert_eq!(
            storage
                .research_resource(&key)
                .map_err(super::super::library::error)
                .and_then(describe)
                .unwrap_err()
                .code,
            "NOT_FOUND"
        );
        drop(storage);
    }
}
