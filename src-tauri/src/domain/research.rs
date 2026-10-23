//! Versioned research metadata. None of these values grant filesystem or execution access.
use serde::{Deserialize, Serialize};

pub const RESOURCE_FORMAT_VERSION: u32 = 1;
pub const MAX_RESOURCE_JSON_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ResourceKind {
    Project,
    Document,
    Literature,
    Screenshot,
    PdfAttachment,
    PdfLocation,
    Annotation,
    HandwritingLayer,
    Stroke,
    CodeWorkspace,
    Evidence,
    AgentSession,
}
impl ResourceKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Document => "document",
            Self::Literature => "literature",
            Self::Screenshot => "screenshot",
            Self::PdfAttachment => "pdfAttachment",
            Self::PdfLocation => "pdfLocation",
            Self::Annotation => "annotation",
            Self::HandwritingLayer => "handwritingLayer",
            Self::Stroke => "stroke",
            Self::CodeWorkspace => "codeWorkspace",
            Self::Evidence => "evidence",
            Self::AgentSession => "agentSession",
        }
    }
    pub fn parse(value: &str) -> Result<Self, &'static str> {
        serde_json::from_value(serde_json::Value::String(value.into()))
            .map_err(|_| "资源类型不受支持。")
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceKey {
    pub kind: ResourceKind,
    pub id: String,
}
impl ResourceKey {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.len() != 32
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("资源ID须为32位小写十六进制稳定标识。");
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DocumentType {
    Markdown,
    Pdf,
    Code,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentResource {
    pub document_type: DocumentType,
    pub project_id: Option<String>,
    /// Managed-data-relative for PDFs; code-workspace-relative for code. Never an access grant.
    pub relative_path: Option<String>,
    pub code_workspace_id: Option<String>,
    pub content_hash: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PdfAttachment {
    pub document_id: String,
    pub reference_id: Option<String>,
    pub content_hash: String,
    pub byte_size: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageRectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextAnchor {
    pub exact: String,
    pub prefix: String,
    pub suffix: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PdfLocation {
    pub attachment_id: String,
    pub content_hash: String,
    /// Zero based, independent of the display page label. Coordinates use the unrotated crop box, top-left origin, PDF points.
    pub page_index: u32,
    pub page_label: Option<String>,
    pub page_width: f64,
    pub page_height: f64,
    pub rectangles: Vec<PageRectangle>,
    pub text_anchor: Option<TextAnchor>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StandardAnnotationType {
    Highlight,
    Underline,
    Note,
    Image,
    Ink,
    Text,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Annotation {
    pub location_id: String,
    pub annotation_type: StandardAnnotationType,
    pub text: String,
    pub comment: String,
    /// This is standard PDF metadata, never a private pressure layer or an instruction to write Zotero.
    pub source: AnnotationSource,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnnotationSource {
    Pdf,
    Zotero,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandwritingLayer {
    pub attachment_id: String,
    pub content_hash: String,
    pub page_index: u32,
    pub page_width: f64,
    pub page_height: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StrokePoint {
    pub x: f64,
    pub y: f64,
    pub elapsed_ms: u32,
    pub pressure: Option<f64>,
    pub tilt_x: Option<f64>,
    pub tilt_y: Option<f64>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StrokeTool {
    Pen,
    Highlighter,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Stroke {
    pub layer_id: String,
    pub tool: StrokeTool,
    pub color: String,
    pub width: f64,
    pub points: Vec<StrokePoint>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodeWorkspace {
    pub project_id: Option<String>,
    /// A locator only. Opening or executing anything still requires an explicit native grant.
    pub root_path: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchEvidence {
    pub claim: String,
    pub document_id: Option<String>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentProvider {
    Codex,
    ClaudeCode,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentPhase {
    Idle,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentSession {
    pub workspace_id: String,
    pub provider: AgentProvider,
    pub provider_session_id: Option<String>,
    pub phase: AgentPhase,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum ResearchData {
    Document(DocumentResource),
    PdfAttachment(PdfAttachment),
    PdfLocation(PdfLocation),
    Annotation(Annotation),
    HandwritingLayer(HandwritingLayer),
    Stroke(Stroke),
    CodeWorkspace(CodeWorkspace),
    Evidence(ResearchEvidence),
    AgentSession(AgentSession),
}
fn visible(text: &str, max: usize) -> bool {
    text.len() <= max && !text.contains('\0')
}
pub fn hash_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn relative_path_valid(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && path.split('/').all(|part| {
            let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            let device = stem
                .strip_prefix("COM")
                .or_else(|| stem.strip_prefix("LPT"))
                .is_some_and(|v| {
                    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³"].contains(&v)
                });
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.ends_with(['.', ' '])
                && !part
                    .chars()
                    .any(|c| c.is_control() || "<>:\\\"|?*".contains(c))
                && !["CON", "PRN", "AUX", "NUL", "CLOCK$"].contains(&stem.as_str())
                && !device
        })
}
impl ResearchData {
    pub fn kind(&self) -> ResourceKind {
        match self {
            Self::Document(_) => ResourceKind::Document,
            Self::PdfAttachment(_) => ResourceKind::PdfAttachment,
            Self::PdfLocation(_) => ResourceKind::PdfLocation,
            Self::Annotation(_) => ResourceKind::Annotation,
            Self::HandwritingLayer(_) => ResourceKind::HandwritingLayer,
            Self::Stroke(_) => ResourceKind::Stroke,
            Self::CodeWorkspace(_) => ResourceKind::CodeWorkspace,
            Self::Evidence(_) => ResourceKind::Evidence,
            Self::AgentSession(_) => ResourceKind::AgentSession,
        }
    }
    pub fn dependencies(&self) -> Vec<ResourceKey> {
        let mut refs = Vec::new();
        let mut add = |kind, id: &str| {
            refs.push(ResourceKey {
                kind,
                id: id.into(),
            })
        };
        match self {
            Self::Document(d) => {
                if let Some(id) = &d.project_id {
                    add(ResourceKind::Project, id)
                }
                if let Some(id) = &d.code_workspace_id {
                    add(ResourceKind::CodeWorkspace, id)
                }
            }
            Self::PdfAttachment(d) => {
                add(ResourceKind::Document, &d.document_id);
                if let Some(id) = &d.reference_id {
                    add(ResourceKind::Literature, id)
                }
            }
            Self::PdfLocation(d) => add(ResourceKind::PdfAttachment, &d.attachment_id),
            Self::Annotation(d) => add(ResourceKind::PdfLocation, &d.location_id),
            Self::HandwritingLayer(d) => add(ResourceKind::PdfAttachment, &d.attachment_id),
            Self::Stroke(d) => add(ResourceKind::HandwritingLayer, &d.layer_id),
            Self::CodeWorkspace(d) => {
                if let Some(id) = &d.project_id {
                    add(ResourceKind::Project, id)
                }
            }
            Self::Evidence(d) => {
                if let Some(id) = &d.document_id {
                    add(ResourceKind::Document, id)
                }
            }
            Self::AgentSession(d) => add(ResourceKind::CodeWorkspace, &d.workspace_id),
        }
        refs
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if serde_json::to_vec(self)
            .map_err(|_| "资源元数据无法编码。")?
            .len()
            > MAX_RESOURCE_JSON_BYTES
        {
            return Err("资源元数据超过2MiB。");
        }
        for key in self.dependencies() {
            key.validate()?;
        }
        let page = |w: f64, h: f64| {
            w.is_finite() && h.is_finite() && w > 0. && h > 0. && w <= 1_000_000. && h <= 1_000_000.
        };
        let ok = match self {
            Self::Document(d) => {
                d.document_type != DocumentType::Markdown
                    && d.relative_path.as_deref().is_none_or(relative_path_valid)
                    && d.content_hash.as_deref().is_none_or(hash_valid)
                    && match d.document_type {
                        DocumentType::Pdf => {
                            d.code_workspace_id.is_none()
                                && d.relative_path
                                    .as_ref()
                                    .is_none_or(|p| p.starts_with("pdfs/") && p.ends_with(".pdf"))
                        }
                        DocumentType::Code => {
                            d.code_workspace_id.is_some()
                                && d.relative_path.as_ref().is_none_or(|p| {
                                    let name = p.to_ascii_lowercase();
                                    !name.ends_with(".md") && !name.ends_with(".markdown")
                                })
                        }
                        DocumentType::Markdown => false,
                    }
            }
            Self::PdfAttachment(d) => {
                hash_valid(&d.content_hash)
                    && d.byte_size > 0
                    && d.byte_size <= 2 * 1024 * 1024 * 1024
            }
            Self::PdfLocation(d) => {
                hash_valid(&d.content_hash)
                    && d.page_index < 1_000_000
                    && page(d.page_width, d.page_height)
                    && d.page_label.as_ref().is_none_or(|s| visible(s, 200))
                    && d.rectangles.len() <= 100
                    && d.rectangles.iter().all(|r| {
                        [r.x, r.y, r.width, r.height].iter().all(|n| n.is_finite())
                            && r.x >= 0.
                            && r.y >= 0.
                            && r.width > 0.
                            && r.height > 0.
                            && r.x + r.width <= d.page_width
                            && r.y + r.height <= d.page_height
                    })
                    && d.text_anchor.as_ref().is_none_or(|t| {
                        !t.exact.is_empty()
                            && visible(&t.exact, 65536)
                            && visible(&t.prefix, 4096)
                            && visible(&t.suffix, 4096)
                    })
            }
            Self::Annotation(d) => visible(&d.text, 65536) && visible(&d.comment, 65536),
            Self::HandwritingLayer(d) => {
                hash_valid(&d.content_hash)
                    && d.page_index < 1_000_000
                    && page(d.page_width, d.page_height)
            }
            Self::Stroke(d) => {
                d.width.is_finite()
                    && d.width > 0.
                    && d.width <= 1000.
                    && d.color.len() == 7
                    && d.color.starts_with('#')
                    && d.color[1..].bytes().all(|c| c.is_ascii_hexdigit())
                    && !d.points.is_empty()
                    && d.points.len() <= 10_000
                    && d.points
                        .windows(2)
                        .all(|p| p[0].elapsed_ms <= p[1].elapsed_ms)
                    && d.points.iter().all(|p| {
                        p.x.is_finite()
                            && p.y.is_finite()
                            && p.x >= 0.
                            && p.y >= 0.
                            && p.pressure
                                .is_none_or(|v| v.is_finite() && (0.0..=1.0).contains(&v))
                            && [p.tilt_x, p.tilt_y]
                                .into_iter()
                                .flatten()
                                .all(|v| v.is_finite() && (-90.0..=90.0).contains(&v))
                    })
            }
            Self::CodeWorkspace(d) => {
                !d.root_path.is_empty()
                    && visible(&d.root_path, 4096)
                    && std::path::Path::new(&d.root_path).is_absolute()
            }
            Self::Evidence(d) => !d.claim.trim().is_empty() && visible(&d.claim, 65536),
            Self::AgentSession(d) => d
                .provider_session_id
                .as_ref()
                .is_none_or(|s| !s.is_empty() && visible(s, 512)),
        };
        if ok {
            Ok(())
        } else {
            Err("资源元数据、类型、坐标或版本无效；未写入资料。")
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZoteroKey {
    pub instance_id: String,
    pub library_id: i64,
    pub item_key: String,
}
impl ZoteroKey {
    pub fn validate(&self) -> Result<(), &'static str> {
        ResourceKey {
            kind: ResourceKind::Literature,
            id: self.instance_id.clone(),
        }
        .validate()?;
        if self.library_id <= 0
            || self.item_key.len() != 8
            || !self
                .item_key
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        {
            return Err("Zotero外部键必须限定实例、library ID及8位item key。");
        }
        Ok(())
    }
}
