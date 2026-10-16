use crate::{
    capture::{CaptureError, ErrorCode, MAX_PNG_BYTES},
    services::workflow::{decode_png, export_png_new},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

pub const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
fn invalid(message: &str) -> CaptureError {
    CaptureError::new(ErrorCode::SaveFailed, message)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Upload {
    document: Value,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Project {
    pub format: String,
    pub schema_version: u32,
    pub original_image: String,
    pub document: Value,
}

pub fn validate_document(doc: &Value) -> Result<(), CaptureError> {
    if doc["schemaVersion"].as_u64() != Some(1) {
        return Err(invalid("标注版本不支持，现有文件未改动。"));
    }
    let w = doc["image"]["widthPx"]
        .as_u64()
        .ok_or_else(|| invalid("项目图片尺寸无效。"))?;
    let h = doc["image"]["heightPx"]
        .as_u64()
        .ok_or_else(|| invalid("项目图片尺寸无效。"))?;
    if w == 0 || h == 0 || w.checked_mul(h).is_none_or(|n| n > 16_777_216) {
        return Err(invalid("项目图片尺寸超出上限。"));
    }
    let objects = doc["objects"]
        .as_array()
        .filter(|v| v.len() <= 500)
        .ok_or_else(|| invalid("项目标注数量无效。"))?;
    let mut ids = std::collections::HashSet::new();
    let mut total_points = 0;
    let number = |v: &Value, min: f64, max: f64| {
        v.as_f64()
            .is_some_and(|n| n.is_finite() && n >= min && n <= max)
    };
    for a in objects {
        let id = a["id"]
            .as_str()
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 128
                    && s.bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            })
            .ok_or_else(|| invalid("标注 ID 无效。"))?;
        if !ids.insert(id) {
            return Err(invalid("标注 ID 重复。"));
        }
        let color = a["color"].as_str().unwrap_or("");
        if !number(&a["x"], -1_000_000., 1_000_000.)
            || !number(&a["y"], -1_000_000., 1_000_000.)
            || !number(&a["strokeWidth"], 1., 32.)
            || color.len() != 7
            || !color.starts_with('#')
            || !color[1..].bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err(invalid("标注属性无效。"));
        }
        match a["type"].as_str() {
            Some("rectangle" | "ellipse")
                if number(&a["width"], 1., 1_000_000.) && number(&a["height"], 1., 1_000_000.) => {}
            Some(kind @ ("arrow" | "pen")) => {
                let p = a["points"]
                    .as_array()
                    .filter(|p| {
                        p.len() >= 4
                            && p.len() <= 10000
                            && p.len() % 2 == 0
                            && (kind != "arrow" || p.len() == 4)
                    })
                    .ok_or_else(|| invalid("标注路径无效。"))?;
                if !p.iter().all(|v| number(v, -1_000_000., 1_000_000.))
                    || !p.iter().enumerate().any(|(i, v)| {
                        (v.as_f64().unwrap() - p[i % 2].as_f64().unwrap()).abs() >= 1.
                    })
                {
                    return Err(invalid("标注路径为空或无效。"));
                }
                total_points += p.len() / 2;
            }
            Some("text")
                if a["text"]
                    .as_str()
                    .is_some_and(|s| !s.trim().is_empty() && s.encode_utf16().count() <= 2000)
                    && number(&a["fontSize"], 8., 200.) => {}
            _ => return Err(invalid("标注类型或数据不支持。")),
        }
    }
    if total_points > 50000
        || serde_json::to_vec(doc)
            .map_err(|_| invalid("标注数据无效。"))?
            .len()
            > MAX_DOCUMENT_BYTES
    {
        return Err(invalid("标注文件超过大小上限。"));
    }
    Ok(())
}
pub fn unpack_upload(bytes: &[u8]) -> Result<(Value, &[u8]), CaptureError> {
    if bytes.len() < 4 || bytes.len() > 4 + MAX_DOCUMENT_BYTES + MAX_PNG_BYTES {
        return Err(invalid("编辑图片数据大小无效。"));
    }
    let size = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    if size == 0 || size > MAX_DOCUMENT_BYTES || size + 4 >= bytes.len() {
        return Err(invalid("编辑图片数据不完整。"));
    }
    let upload: Upload =
        serde_json::from_slice(&bytes[4..4 + size]).map_err(|_| invalid("标注 JSON 无效。"))?;
    validate_document(&upload.document)?;
    let png = &bytes[4 + size..];
    validate_image(&upload.document, png)?;
    Ok((upload.document, png))
}
fn validate_image(doc: &Value, png: &[u8]) -> Result<(), CaptureError> {
    let frame = decode_png(png)?;
    if doc["image"]["widthPx"].as_u64() != Some(frame.width_px.into())
        || doc["image"]["heightPx"].as_u64() != Some(frame.height_px.into())
    {
        return Err(invalid("原图尺寸与标注数据不一致。"));
    }
    Ok(())
}
fn sidecar_name(name: &str) -> bool {
    name.strip_prefix("original-")
        .and_then(|s| s.strip_suffix(".png"))
        .is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 100
                && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), CaptureError> {
    let mut stage =
        tempfile::NamedTempFile::new_in(path.parent().ok_or_else(|| invalid("保存目录无效。"))?)
            .map_err(|_| invalid("无法写入所选目录。"))?;
    stage
        .write_all(bytes)
        .and_then(|_| stage.as_file().sync_all())
        .map_err(|_| invalid("写入项目失败，请检查磁盘空间和权限。"))?;
    stage.persist_noclobber(path).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            CaptureError::new(
                ErrorCode::FileExists,
                "同名项目已存在，请选择新的文件名。原文件未改动。",
            )
        } else {
            invalid("无法提交项目文件。")
        }
    })?;
    Ok(())
}
pub fn save_project(path: &Path, doc: Value, png: &[u8]) -> Result<(), CaptureError> {
    validate_document(&doc)?;
    validate_image(&doc, png)?;
    if !path.is_absolute()
        || !path
            .file_name()
            .is_some_and(|s| s.to_string_lossy().ends_with(".researchclip.json"))
    {
        return Err(invalid("项目文件名须以 .researchclip.json 结尾。"));
    }
    if path.exists() {
        return Err(CaptureError::new(
            ErrorCode::FileExists,
            "同名项目已存在，请选择新的文件名。原文件未改动。",
        ));
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| invalid("系统时间无效。"))?
        .as_nanos();
    let name = format!("original-{stamp}-{}.png", std::process::id());
    let original = path.parent().unwrap().join(&name);
    let json = serde_json::to_vec_pretty(&Project {
        format: "ResearchClip".into(),
        schema_version: 1,
        original_image: name,
        document: doc,
    })
    .map_err(|_| invalid("无法序列化项目。"))?;
    if json.len() > MAX_DOCUMENT_BYTES {
        return Err(invalid("项目文件超过大小上限。"));
    }
    export_png_new(&original, png)?;
    // On failure keep the newly committed original; never delete a potentially user-owned file.
    write_new(path, &json).map_err(|error| {
        CaptureError::new(
            error.code,
            "项目 JSON 未保存，原始 PNG 副本已保留。现有文件未改动，请选择新的项目名称或目录重试。",
        )
    })
}
fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, CaptureError> {
    let file = File::open(path)
        .map_err(|_| invalid("无法读取项目或原图，请检查文件是否齐全和目录权限。"))?;
    if !file
        .metadata()
        .is_ok_and(|m| m.is_file() && m.len() <= limit as u64)
    {
        return Err(invalid("项目或原图不是有效文件，或超过大小上限。"));
    }
    let mut data = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|_| invalid("读取项目失败。"))?;
    if data.len() > limit {
        return Err(invalid("文件超过大小上限。"));
    }
    Ok(data)
}
pub fn open_project(path: &Path) -> Result<Vec<u8>, CaptureError> {
    let json = read_bounded(path, MAX_DOCUMENT_BYTES)?;
    let project: Project =
        serde_json::from_slice(&json).map_err(|_| invalid("项目 JSON 无效，文件未改动。"))?;
    if project.format != "ResearchClip" || project.schema_version != 1 {
        return Err(invalid("项目版本不支持，文件未改动。"));
    }
    validate_document(&project.document)?;
    if !sidecar_name(&project.original_image) {
        return Err(invalid("项目原图引用无效。"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| invalid("项目目录无效。"))?
        .canonicalize()
        .map_err(|_| invalid("项目目录不可用。"))?;
    let original = parent
        .join(&project.original_image)
        .canonicalize()
        .map_err(|_| invalid("项目原图缺失，请将 JSON 与原始 PNG 一起移动。"))?;
    if original.parent() != Some(parent.as_path()) {
        return Err(invalid("项目原图引用超出所选目录。"));
    }
    let png = read_bounded(&original, MAX_PNG_BYTES)?;
    validate_image(&project.document, &png)?;
    let mut result = Vec::with_capacity(4 + json.len() + png.len());
    result.extend_from_slice(&(json.len() as u32).to_le_bytes());
    result.extend_from_slice(&json);
    result.extend_from_slice(&png);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::RawFrame, services::screen_capture::encode_png};
    fn source() -> Vec<u8> {
        encode_png(RawFrame {
            width_px: 2,
            height_px: 1,
            rgba: vec![0, 0, 0, 0, 20, 30, 40, 128],
        })
        .unwrap()
    }
    fn doc() -> Value {
        serde_json::json!({"schemaVersion":1,"image":{"widthPx":2,"heightPx":1},"objects":[
            {"id":"text","type":"text","x":0,"y":0,"color":"#ff0000","strokeWidth":2,"fontSize":12,"text":"研究\nA"},
            {"id":"rect","type":"rectangle","x":-1,"y":0,"width":2,"height":1,"color":"#123456","strokeWidth":3},
            {"id":"ellipse","type":"ellipse","x":0,"y":0,"width":2,"height":1,"color":"#abcdef","strokeWidth":1},
            {"id":"arrow","type":"arrow","x":-1,"y":0,"points":[1,0,3,1],"color":"#00ff00","strokeWidth":4},
            {"id":"pen","type":"pen","x":0,"y":0,"points":[0,0,1,1,2,0],"color":"#0000ff","strokeWidth":2}
        ]})
    }
    #[test]
    fn project_roundtrip_preserves_original_and_all_vectors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.researchclip.json");
        let png = source();
        save_project(&path, doc(), &png).unwrap();
        let bytes = open_project(&path).unwrap();
        let n = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        let project: Project = serde_json::from_slice(&bytes[4..4 + n]).unwrap();
        assert_eq!(project.document, doc());
        assert_eq!(project.document["objects"].as_array().unwrap().len(), 5);
        assert_eq!(&bytes[4 + n..], png);
        assert_eq!(
            save_project(&path, doc(), &png).unwrap_err().code,
            ErrorCode::FileExists
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
        let original = dir.path().join(project.original_image);
        assert_eq!(std::fs::read(original).unwrap(), png);
    }
    #[test]
    fn malformed_future_and_path_escape_projects_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.researchclip.json");
        for bytes in [b"{\"schemaVersion\":99}".as_slice(), b"{broken}"] {
            std::fs::write(&path, bytes).unwrap();
            assert!(open_project(&path).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        let future = Project {
            format: "ResearchClip".into(),
            schema_version: 99,
            original_image: "original-123.png".into(),
            document: doc(),
        };
        let future_bytes = serde_json::to_vec(&future).unwrap();
        std::fs::write(&path, &future_bytes).unwrap();
        assert!(open_project(&path).unwrap_err().message.contains("版本"));
        assert_eq!(std::fs::read(&path).unwrap(), future_bytes);
        let project = Project {
            format: "ResearchClip".into(),
            schema_version: 1,
            original_image: "../original-user.png".into(),
            document: doc(),
        };
        std::fs::write(&path, serde_json::to_vec(&project).unwrap()).unwrap();
        assert!(open_project(&path).is_err());
        assert!(!sidecar_name("original-..png"));
        assert!(!sidecar_name("C:\\outside.png"));
        assert!(sidecar_name("original-123-45.png"));
    }
    #[test]
    fn upload_limits_versions_geometry_and_dimensions_are_validated() {
        let json = serde_json::to_vec(&serde_json::json!({"document":doc()})).unwrap();
        let mut bytes = (json.len() as u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(&json);
        bytes.extend_from_slice(&source());
        assert!(unpack_upload(&bytes).is_ok());
        assert!(unpack_upload(&[255; 4]).is_err());
        assert!(unpack_upload(&bytes[..bytes.len() - 3]).is_err());
        let mut bad = doc();
        bad["schemaVersion"] = 99.into();
        assert!(validate_document(&bad).is_err());
        bad = doc();
        bad["objects"][0]["fontSize"] = 0.into();
        assert!(validate_document(&bad).is_err());
        for (index, key, value) in [
            (1, "width", serde_json::json!(0)),
            (2, "height", serde_json::json!(-1)),
            (3, "points", serde_json::json!([0, 0, 1])),
            (4, "points", serde_json::json!([0, 0, 1000001, 0])),
        ] {
            bad = doc();
            bad["objects"][index][key] = value;
            assert!(validate_document(&bad).is_err());
        }
        bad = doc();
        bad["image"]["widthPx"] = 3.into();
        assert!(validate_image(&bad, &source()).is_err());
    }
}
