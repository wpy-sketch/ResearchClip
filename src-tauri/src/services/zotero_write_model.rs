//! Bounded standard-annotation operations. No arbitrary JSON, URL, path or SQL.
use super::zotero_local::valid_item_key;
use serde::{Deserialize, Serialize};
pub const WRITE_TYPES: [&str; 3] = ["highlight", "underline", "note"];
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WriteGeometry {
    pub page_index: u32,
    pub view: [f64; 4],
    pub user_unit: f64,
    /// Unrotated physical points from the crop's top-left: x,y,width,height.
    pub rectangles: Vec<[f64; 4]>,
}
impl WriteGeometry {
    pub fn raw_rectangles(&self) -> Result<Vec<[f64; 4]>, &'static str> {
        let [x0, y0, x1, y1] = self.view;
        let u = self.user_unit;
        if self.page_index >= 2000
            || !self
                .view
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
            || !u.is_finite()
            || u <= 0.0
            || u > 1000.0
            || x1 <= x0
            || y1 <= y0
            || self.rectangles.is_empty()
            || self.rectangles.len() > 100
        {
            return Err("批注页面、裁切框或矩形数量无效。");
        }
        let w = (x1 - x0) * u;
        let h = (y1 - y0) * u;
        if !w.is_finite() || !h.is_finite() || w > 1_000_000.0 || h > 1_000_000.0 {
            return Err("批注页面尺寸超出保护范围。");
        }
        self.rectangles
            .iter()
            .map(|r| {
                let [x, y, rw, rh] = *r;
                if !r.iter().all(|v| v.is_finite())
                    || x < 0.0
                    || y < 0.0
                    || rw <= 0.0
                    || rh <= 0.0
                    || x + rw > w + 0.000001
                    || y + rh > h + 0.000001
                {
                    return Err("批注区域必须位于当前页面裁切范围内。");
                }
                Ok([x0 + x / u, y1 - (y + rh) / u, x0 + (x + rw) / u, y1 - y / u])
            })
            .collect()
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AnnotationWriteInput {
    Create {
        annotation_type: String,
        color: String,
        text: String,
        comment: String,
        geometry: WriteGeometry,
    },
    Update {
        annotation_key: String,
        expected_version: u64,
        annotation_type: String,
        color: String,
        text: String,
        comment: String,
    },
    Delete {
        annotation_key: String,
        expected_version: u64,
    },
}
fn validate_text(value: &str) -> bool {
    value.len() <= 65536
        && value
            .chars()
            .all(|c| !c.is_control() || matches!(c, '\r' | '\n' | '\t'))
}
impl AnnotationWriteInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Create {
                annotation_type,
                color,
                text,
                comment,
                geometry,
            } => {
                validate_content(annotation_type, color, text, comment)?;
                geometry.raw_rectangles()?;
                if annotation_type == "note" && geometry.rectangles.len() != 1 {
                    return Err("普通文字便签只绑定一个页面区域。");
                }
            }
            Self::Update {
                annotation_key,
                expected_version,
                annotation_type,
                color,
                text,
                comment,
            } => {
                validate_identity(annotation_key, *expected_version)?;
                validate_content(annotation_type, color, text, comment)?;
            }
            Self::Delete {
                annotation_key,
                expected_version,
            } => validate_identity(annotation_key, *expected_version)?,
        }
        Ok(())
    }
}
fn validate_identity(key: &str, version: u64) -> Result<(), &'static str> {
    if !valid_item_key(key) || version == 0 || version > 9_007_199_254_740_991 {
        return Err("批注原始键或对象版本无效，请重新读取。");
    }
    Ok(())
}
fn validate_content(
    kind: &str,
    color: &str,
    text: &str,
    comment: &str,
) -> Result<(), &'static str> {
    if !WRITE_TYPES.contains(&kind) {
        return Err("此批注类型尚未验证写入，本版禁用。");
    }
    if !super::zotero_annotations::color_valid(color)
        || !validate_text(text)
        || !validate_text(comment)
        || kind == "note" && (!text.is_empty() || comment.trim().is_empty())
        || kind != "note" && text.trim().is_empty()
    {
        return Err("批注文字、颜色或便签内容无效。");
    }
    Ok(())
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationWriteCapabilities {
    pub product_version: Option<String>,
    pub supported_types: Vec<String>,
    pub authorized: bool,
    pub remembered: bool,
    pub message: String,
}
pub fn tested_write_version(version: Option<&str>) -> bool {
    // Enable only the exact installed version exercised against a separate real library.
    version == Some("10.0.6")
}
#[cfg(test)]
mod tests {
    use super::*;
    fn geometry() -> WriteGeometry {
        WriteGeometry {
            page_index: 2,
            view: [40.0, 50.0, 640.0, 850.0],
            user_unit: 2.0,
            rectangles: vec![[100.0, 150.0, 200.0, 100.0]],
        }
    }
    #[test]
    fn physical_crop_userunit_coordinates_become_pdf_native_rects_without_view_rotation_or_dpi() {
        assert_eq!(
            geometry().raw_rectangles().unwrap(),
            vec![[90.0, 725.0, 190.0, 775.0]]
        );
        let mut g = geometry();
        g.view = [0.0, 0.0, 600.0, 800.0];
        g.user_unit = 1.0;
        assert_eq!(
            g.raw_rectangles().unwrap(),
            vec![[100.0, 550.0, 300.0, 650.0]]
        );
    }
    #[test]
    fn rejects_bad_pages_nonfinite_inverted_outside_and_excess_geometry() {
        for g in [
            WriteGeometry {
                page_index: 2000,
                ..geometry()
            },
            WriteGeometry {
                user_unit: 0.0,
                ..geometry()
            },
            WriteGeometry {
                view: [40.0, 50.0, f64::INFINITY, 850.0],
                ..geometry()
            },
            WriteGeometry {
                rectangles: vec![[0.0, 0.0, -1.0, 10.0]],
                ..geometry()
            },
            WriteGeometry {
                rectangles: vec![[1199.0, 0.0, 2.0, 10.0]],
                ..geometry()
            },
            WriteGeometry {
                rectangles: vec![[0.0, 0.0, 1.0, 1.0]; 101],
                ..geometry()
            },
        ] {
            assert!(g.raw_rectangles().is_err());
        }
    }
    #[test]
    fn strict_wire_rejects_unknown_fields_paths_arbitrary_operations_and_private_ink() {
        assert!(serde_json::from_str::<AnnotationWriteInput>(r#"{"action":"delete","annotationKey":"ANN00001","expectedVersion":1,"path":"C:/anything"}"#).is_err());
        assert!(
            serde_json::from_str::<AnnotationWriteInput>(r#"{"action":"execute","sql":"DELETE"}"#)
                .is_err()
        );
        for kind in ["text", "image", "ink", "other"] {
            assert!(
                AnnotationWriteInput::Create {
                    annotation_type: kind.into(),
                    color: "#ffd400".into(),
                    text: "选文".into(),
                    comment: String::new(),
                    geometry: geometry()
                }
                .validate()
                .is_err()
            );
        }
    }
    #[test]
    fn validates_scoped_foreign_identity_version_content_and_unverified_clients() {
        assert!(validate_identity("../bad", 1).is_err());
        assert!(validate_identity("ANN00001", 0).is_err());
        assert!(validate_content("note", "#ffd400", "", "文字便签").is_ok());
        assert!(validate_content("note", "#ffd400", "不允许annotationText", "评论").is_err());
        assert!(validate_content("highlight", "url(x)", "选文", "").is_err());
        assert!(validate_content("highlight", "#ffd400", "选文", &"x".repeat(65537)).is_err());
        assert!(tested_write_version(Some("10.0.6")));
        for v in [
            None,
            Some("9.0"),
            Some("10.0.5"),
            Some("10.0.7"),
            Some("10.0.6-unverified"),
        ] {
            assert!(!tested_write_version(v));
        }
    }
}
