//! Read-only protocol model. No Zotero code, database access or write operations.
use super::zotero_local::valid_item_key;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnotationSource {
    pub instance_id: String,
    pub library_type: String,
    pub library_id: u64,
    pub item_key: String,
    pub attachment_key: String,
}

impl AnnotationSource {
    pub fn validate(&self) -> Result<(), String> {
        if self.instance_id.len() != 32
            || !self
                .instance_id
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            || !matches!(self.library_type.as_str(), "user" | "group")
            || self.library_id > 9_007_199_254_740_991
            || self.library_type == "group" && self.library_id == 0
            || !valid_item_key(&self.item_key)
            || !valid_item_key(&self.attachment_key)
            || self.item_key == self.attachment_key
        {
            return Err("Zotero批注来源身份无效。".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZoteroAnnotationIdentity {
    pub source: AnnotationSource,
    pub annotation_key: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnotationPosition {
    pub page_index: u32,
    pub rects: Vec<[f64; 4]>,
    pub next_page_rects: Vec<[f64; 4]>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZoteroAnnotation {
    pub identity: ZoteroAnnotationIdentity,
    pub version: u64,
    pub annotation_type: String,
    pub color: String,
    pub text: String,
    pub comment: String,
    pub page_label: String,
    pub position: Option<AnnotationPosition>,
    pub unsupported_reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnotationReadCache {
    pub format_version: u32,
    pub source: AnnotationSource,
    pub pdf_content_hash: String,
    pub fetched_at: i64,
    pub product_version: Option<String>,
    pub library_version: Option<u64>,
    pub annotations: Vec<ZoteroAnnotation>,
    pub truncated: bool,
}
pub const SOURCE_MISMATCH: &str = "ZOTERO_ANNOTATION_SOURCE_MISMATCH";
pub fn supported(kind: &str) -> bool {
    matches!(kind, "highlight" | "underline" | "note" | "text" | "image")
}
fn safe_text(value: &Value, key: &str, max: usize) -> Result<String, String> {
    let text = value.get(key).and_then(Value::as_str).unwrap_or("");
    if text.len() > max
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err("Zotero批注文字超过限制或含不支持的控制字符。".into());
    }
    Ok(text.into())
}
pub fn color_valid(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|b| b.is_ascii_hexdigit())
}
fn rectangles(value: Option<&Value>) -> Result<Vec<[f64; 4]>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let list = value.as_array().ok_or("批注矩形不是数组。")?;
    if list.len() > 200 {
        return Err("批注矩形超过200段。".into());
    }
    list.iter()
        .map(|r| {
            let a = r.as_array().ok_or("批注矩形无效。")?;
            if a.len() != 4 {
                return Err("批注矩形需要4个坐标。".into());
            }
            let mut rect = [0.0; 4];
            for (i, n) in a.iter().enumerate() {
                rect[i] = n
                    .as_f64()
                    .filter(|n| n.is_finite() && n.abs() <= 1_000_000.0)
                    .ok_or("批注坐标越界。")?;
            }
            if rect[0] >= rect[2] || rect[1] >= rect[3] {
                return Err("批注矩形为空或方向无效。".into());
            }
            Ok(rect)
        })
        .collect()
}
pub fn parse_position(value: &Value) -> Result<AnnotationPosition, String> {
    let text = value.as_str().ok_or("annotationPosition不是JSON字符串。")?;
    if text.len() > 65000 {
        return Err("批注位置超过65KB限制。".into());
    }
    let p: Value = serde_json::from_str(text).map_err(|_| "批注位置JSON无效。")?;
    let page_index = p
        .get("pageIndex")
        .and_then(Value::as_u64)
        .filter(|n| *n < 2000)
        .ok_or("批注页码超出支持范围。")? as u32;
    let rects = rectangles(p.get("rects"))?;
    let next_page_rects = rectangles(p.get("nextPageRects"))?;
    if rects.is_empty() {
        return Err("本版不支持此批注几何。".into());
    }
    if !next_page_rects.is_empty() && page_index >= 1999 {
        return Err("跨页批注超出支持范围。".into());
    }
    Ok(AnnotationPosition {
        page_index,
        rects,
        next_page_rects,
    })
}
pub fn parse_annotation(
    item: &Value,
    source: &AnnotationSource,
) -> Result<ZoteroAnnotation, String> {
    source.validate()?;
    let data = item.get("data").ok_or("Zotero批注缺少data。")?;
    let key = item
        .get("key")
        .and_then(Value::as_str)
        .ok_or("Zotero批注缺少key。")?;
    if !valid_item_key(key)
        || data.get("itemType").and_then(Value::as_str) != Some("annotation")
        || data.get("parentItem").and_then(Value::as_str) != Some(source.attachment_key.as_str())
        || data.get("key").is_some_and(|v| v.as_str() != Some(key))
    {
        return Err(SOURCE_MISMATCH.into());
    }
    let library = item.get("library").ok_or(SOURCE_MISMATCH)?;
    if library.get("type").and_then(Value::as_str) != Some(source.library_type.as_str())
        || library.get("id").and_then(Value::as_u64) != Some(source.library_id)
    {
        return Err(SOURCE_MISMATCH.into());
    }
    let version = item
        .get("version")
        .and_then(Value::as_u64)
        .filter(|v| *v <= 9_007_199_254_740_991)
        .ok_or("Zotero批注版本无效。")?;
    let kind = safe_text(data, "annotationType", 32)?;
    if kind.is_empty() {
        return Err("Zotero批注类型缺失。".into());
    }
    let raw_color = safe_text(data, "annotationColor", 64)?;
    let color = if color_valid(&raw_color) {
        raw_color.to_lowercase()
    } else {
        "#aaaaaa".into()
    };
    let (position, unsupported_reason) = if !supported(&kind) {
        (
            None,
            Some(format!(
                "Zotero {kind} 批注：本版仅保留身份与文字，不显示此类型图形。"
            )),
        )
    } else {
        match parse_position(data.get("annotationPosition").unwrap_or(&Value::Null)) {
            Ok(p) => (Some(p), None),
            Err(e) => (None, Some(e)),
        }
    };
    let unsupported_reason = unsupported_reason.or_else(|| {
        (!color_valid(&raw_color))
            .then(|| "颜色字段不受支持，以灰色显示；原Zotero批注未修改。".into())
    });
    Ok(ZoteroAnnotation {
        identity: ZoteroAnnotationIdentity {
            source: source.clone(),
            annotation_key: key.into(),
        },
        version,
        annotation_type: kind,
        color,
        text: safe_text(data, "annotationText", 65536)?,
        comment: safe_text(data, "annotationComment", 65536)?,
        page_label: safe_text(data, "annotationPageLabel", 200)?,
        position,
        unsupported_reason,
    })
}
impl AnnotationReadCache {
    pub fn validate(&self) -> Result<(), String> {
        self.source.validate()?;
        if self.format_version != 1
            || self.pdf_content_hash.len() != 64
            || !self
                .pdf_content_hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.fetched_at <= 0
            || self.fetched_at > 9_007_199_254_740_991
            || self.annotations.len() > 1000
            || self.product_version.as_ref().is_some_and(|v| v.len() > 64)
            || self
                .library_version
                .is_some_and(|v| v > 9_007_199_254_740_991)
        {
            return Err("批注缓存版本、摘要或预算无效。".into());
        }
        let mut keys = std::collections::HashSet::new();
        for a in &self.annotations {
            if a.identity.source != self.source
                || !valid_item_key(&a.identity.annotation_key)
                || !keys.insert(&a.identity.annotation_key)
                || a.version > 9_007_199_254_740_991
                || a.annotation_type.is_empty()
                || a.annotation_type.len() > 32
                || !color_valid(&a.color)
                || a.text.len() > 65536
                || a.comment.len() > 65536
                || a.page_label.len() > 200
                || a.unsupported_reason.as_ref().is_some_and(|r| r.len() > 500)
                || a.position.is_none() && a.unsupported_reason.is_none()
            {
                return Err("批注缓存身份、文字或版本无效。".into());
            }
            if let Some(p) = &a.position {
                if !supported(&a.annotation_type)
                    || p.page_index >= 2000
                    || p.rects.is_empty()
                    || p.rects.len() > 200
                    || p.next_page_rects.len() > 200
                    || p.page_index >= 1999 && !p.next_page_rects.is_empty()
                {
                    return Err("批注缓存位置无效。".into());
                }
                for rect in p.rects.iter().chain(&p.next_page_rects) {
                    if rect.iter().any(|n| !n.is_finite() || n.abs() > 1_000_000.0)
                        || rect[0] >= rect[2]
                        || rect[1] >= rect[3]
                    {
                        return Err("批注缓存矩形无效。".into());
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn source() -> AnnotationSource {
        AnnotationSource {
            instance_id: "a".repeat(32),
            library_type: "user".into(),
            library_id: 42,
            item_key: "PARENT01".into(),
            attachment_key: "ATTACH01".into(),
        }
    }
    fn item(kind: &str) -> Value {
        json!({"key":"ANN00001","version":2,"library":{"type":"user","id":42},"data":{"key":"ANN00001","parentItem":"ATTACH01","itemType":"annotation","annotationType":kind,"annotationColor":"#FFD400","annotationText":"实际样式练习（虚构）","annotationComment":"备注 <script>仅原文</script>","annotationPageLabel":"iv","annotationPosition":"{\"pageIndex\":0,\"rects\":[[10,20,110,40]],\"nextPageRects\":[[10,700,110,720]]}"}})
    }
    #[test]
    fn preserves_scoped_original_identity_text_colors_and_cross_page_geometry_for_five_standard_types()
     {
        for kind in ["highlight", "underline", "note", "text", "image"] {
            let a = parse_annotation(&item(kind), &source()).unwrap();
            assert_eq!(a.identity.annotation_key, "ANN00001");
            assert_eq!(a.identity.source.library_id, 42);
            assert_eq!(a.color, "#ffd400");
            assert!(a.comment.contains("<script>"));
            assert_eq!(a.position.unwrap().next_page_rects.len(), 1);
            assert!(a.unsupported_reason.is_none());
        }
    }
    #[test]
    fn refuses_other_library_parent_or_annotation_identity_instead_of_guessing() {
        for (field, value) in [
            ("parentItem", json!("OTHER001")),
            ("key", json!("OTHER001")),
            ("itemType", json!("attachment")),
        ] {
            let mut raw = item("highlight");
            raw["data"][field] = value;
            assert_eq!(
                parse_annotation(&raw, &source()).unwrap_err(),
                SOURCE_MISMATCH
            );
        }
        let mut raw = item("note");
        raw["library"]["id"] = json!(43);
        assert_eq!(
            parse_annotation(&raw, &source()).unwrap_err(),
            SOURCE_MISMATCH
        );
    }
    #[test]
    fn unsupported_ink_is_zotero_data_and_invalid_positions_remain_explicit_unrenderable_items() {
        let mut raw = item("ink");
        raw["data"]["annotationPosition"] =
            json!("{\"pageIndex\":0,\"paths\":[[1,2,3,4]],\"width\":2}");
        let a = parse_annotation(&raw, &source()).unwrap();
        assert_eq!(a.annotation_type, "ink");
        assert!(a.position.is_none());
        assert!(a.unsupported_reason.unwrap().contains("Zotero ink"));
        for value in [
            json!("not-json"),
            json!("{\"pageIndex\":2000,\"rects\":[[0,0,1,1]]}"),
            json!("{\"pageIndex\":0,\"rects\":[[5,0,1,1]]}"),
            json!({"pageIndex":0,"rects":[[0,0,1,1]]}),
        ] {
            let mut raw = item("highlight");
            raw["data"]["annotationPosition"] = value;
            let a = parse_annotation(&raw, &source()).unwrap();
            assert!(a.position.is_none() && a.unsupported_reason.is_some());
        }
    }
    #[test]
    fn cache_rejects_duplicate_future_or_wrong_scope_and_reports_unsafe_color_without_executing_markup()
     {
        let mut raw = item("highlight");
        raw["data"]["annotationColor"] = json!("url(javascript:x)");
        let a = parse_annotation(&raw, &source()).unwrap();
        assert_eq!(a.color, "#aaaaaa");
        assert!(a.unsupported_reason.is_some());
        let mut cache = AnnotationReadCache {
            format_version: 1,
            source: source(),
            pdf_content_hash: "b".repeat(64),
            fetched_at: 1,
            product_version: Some("10.0.6-fixture".into()),
            library_version: Some(2),
            annotations: vec![a.clone()],
            truncated: false,
        };
        cache.validate().unwrap();
        cache.annotations.push(a);
        assert!(cache.validate().is_err());
        cache.annotations.pop();
        cache.format_version = 2;
        assert!(cache.validate().is_err());
        cache.format_version = 1;
        cache.source.library_id = 43;
        assert!(cache.validate().is_err());
    }
}
