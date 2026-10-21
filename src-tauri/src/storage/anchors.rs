use super::{
    Storage, StorageError,
    library::{check_id, now},
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnotationAnchor {
    pub anchor_id: String,
    pub screenshot_id: String,
    pub image_revision: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub title: String,
    pub description: String,
    pub revision: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::RawFrame, services::screen_capture::encode_png};
    fn setup() -> (tempfile::TempDir, Storage, String) {
        let temp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(temp.path().into()).unwrap();
        let png = encode_png(RawFrame {
            width_px: 320,
            height_px: 240,
            rgba: [20, 40, 80, 255].repeat(320 * 240),
        })
        .unwrap();
        let item=s.create_screenshot("练习图",serde_json::json!({"schemaVersion":1,"image":{"widthPx":320,"heightPx":240},"objects":[]}),&png).unwrap();
        (temp, s, item.id)
    }
    fn input(s: &Storage, id: &str) -> AnchorInput {
        AnchorInput {
            anchor_id: None,
            revision: None,
            screenshot_id: id.into(),
            image_revision: s.anchor_page(id).unwrap().image_revision,
            x: 12.5,
            y: 20.,
            width: 80.,
            height: 40.,
            title: "峰值区域".into(),
            description: "对照原图核对".into(),
        }
    }
    #[test]
    fn create_move_conflict_delete_reopen_and_original_unchanged() {
        let (temp, mut s, id) = setup();
        let original = s.screenshot_image(&id, false).unwrap();
        let first = s.save_anchor(input(&s, &id)).unwrap();
        let mut moved = input(&s, &id);
        moved.anchor_id = Some(first.anchor_id.clone());
        moved.revision = Some(first.revision);
        moved.x = 52.;
        let second = s.save_anchor(moved.clone()).unwrap();
        assert_eq!(second.anchor_id, first.anchor_id);
        assert_eq!(second.revision, 2);
        assert_eq!(second.x, 52.);
        assert!(matches!(s.save_anchor(moved), Err(StorageError::Conflict)));
        drop(s);
        let mut s = Storage::open(temp.path().into()).unwrap();
        assert_eq!(s.resolve_anchor(&id, &first.anchor_id).unwrap().x, 52.);
        assert_eq!(s.screenshot_image(&id, false).unwrap(), original);
        assert!(s.delete_anchor(&id, &first.anchor_id, 1).is_err());
        s.delete_anchor(&id, &first.anchor_id, 2).unwrap();
        assert!(s.resolve_anchor(&id, &first.anchor_id).is_err());
        let new = s.save_anchor(input(&s, &id)).unwrap();
        assert_ne!(new.anchor_id, first.anchor_id);
    }
    #[test]
    fn points_bounds_nan_versions_and_paths_fail_closed() {
        let (_temp, mut s, id) = setup();
        let mut p = input(&s, &id);
        p.width = 0.;
        p.height = 0.;
        assert!(s.save_anchor(p).is_ok());
        for (x, y, w, h) in [
            (-1., 0., 2., 2.),
            (0., 0., 0., 2.),
            (319., 0., 2., 2.),
            (320., 0., 0., 0.),
            (f64::NAN, 0., 0., 0.),
            (f64::INFINITY, 0., 0., 0.),
        ] {
            let mut p = input(&s, &id);
            p.x = x;
            p.y = y;
            p.width = w;
            p.height = h;
            assert!(s.save_anchor(p).is_err());
        }
        let mut p = input(&s, &id);
        p.image_revision = "0".repeat(64);
        assert!(matches!(s.save_anchor(p), Err(StorageError::Conflict)));
        assert!(s.anchor_page("../../private").is_err());
        assert!(s.resolve_anchor(&id, "file:///secret").is_err());
        let mut p = input(&s, &id);
        p.title = " ".into();
        assert!(s.save_anchor(p).is_err());
        let mut p = input(&s, &id);
        p.description = "a".repeat(4001);
        assert!(s.save_anchor(p).is_err());
    }
    #[test]
    fn annotation_version_keeps_coordinates_but_original_replacement_invalidates() {
        let (temp, mut s, id) = setup();
        let a = s.save_anchor(input(&s, &id)).unwrap();
        let item = s.get_screenshot(&id).unwrap();
        let mut doc = item.annotation_data;
        doc["objects"] = serde_json::json!([{"id":"test","type":"rectangle","x":1,"y":1,"width":2,"height":2,"color":"#ff0000","strokeWidth":2}]);
        s.save_annotation_revision(&id, item.revision, doc, "另一个标注版本")
            .unwrap();
        assert_eq!(
            s.resolve_anchor(&id, &a.anchor_id).unwrap().image_revision,
            a.image_revision
        );
        let changed = encode_png(RawFrame {
            width_px: 320,
            height_px: 240,
            rgba: [60, 40, 80, 255].repeat(320 * 240),
        })
        .unwrap();
        std::fs::write(temp.path().join(format!("originals/{id}.png")), changed).unwrap();
        assert!(s.resolve_anchor(&id, &a.anchor_id).is_err());
        assert_eq!(s.anchor_page(&id).unwrap().anchors.len(), 1);
    }
    #[test]
    fn v9_migration_preserves_image_and_creates_verified_snapshot() {
        let (temp, s, id) = setup();
        let bytes = s.screenshot_image(&id, false).unwrap();
        s.connection.execute_batch("DROP TABLE annotation_anchors; ALTER TABLE link_index DROP COLUMN anchor_id; PRAGMA user_version=9;").unwrap();
        drop(s);
        let s = Storage::open(temp.path().into()).unwrap();
        assert_eq!(s.status().unwrap().schema_version, 10);
        assert_eq!(s.screenshot_image(&id, false).unwrap(), bytes);
        assert!(s.anchor_page(&id).unwrap().anchors.is_empty());
        assert!(
            std::fs::read_dir(temp.path().join("backups"))
                .unwrap()
                .next()
                .is_some()
        );
    }
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnchorInput {
    pub anchor_id: Option<String>,
    pub screenshot_id: String,
    pub image_revision: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub title: String,
    pub description: String,
    pub revision: Option<i64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorPage {
    pub image_revision: String,
    pub width_px: u32,
    pub height_px: u32,
    pub anchors: Vec<AnnotationAnchor>,
}
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<AnnotationAnchor> {
    Ok(AnnotationAnchor {
        anchor_id: r.get(0)?,
        screenshot_id: r.get(1)?,
        image_revision: r.get(2)?,
        x: r.get(3)?,
        y: r.get(4)?,
        width: r.get(5)?,
        height: r.get(6)?,
        title: r.get(7)?,
        description: r.get(8)?,
        revision: r.get(9)?,
    })
}
const COLUMNS: &str =
    "anchor_id,screenshot_id,image_revision,x,y,width,height,title,description,revision";
impl Storage {
    pub(super) fn available_anchors(
        &self,
        id: &str,
    ) -> Result<std::collections::HashMap<String, AnnotationAnchor>, StorageError> {
        match self.anchor_page(id) {
            Ok(p) => Ok(p
                .anchors
                .into_iter()
                .filter(|a| a.image_revision == p.image_revision)
                .map(|a| (a.anchor_id.clone(), a))
                .collect()),
            Err(e @ StorageError::Database(_)) => Err(e),
            Err(_) => Ok(std::collections::HashMap::new()),
        }
    }

    pub fn anchor_page(&self, id: &str) -> Result<AnchorPage, StorageError> {
        let image = self.get_screenshot(id)?;
        let bytes = self.screenshot_image(id, false)?;
        let image_revision = format!("{:x}", Sha256::digest(bytes));
        let anchors=self.connection.prepare(&format!("SELECT {COLUMNS} FROM annotation_anchors WHERE screenshot_id=?1 ORDER BY updated_at,anchor_id LIMIT 201"))?.query_map([id],row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        if anchors.len() > 200 {
            return Err(StorageError::Invalid(
                "锚点数量超过200，已保留资料，请核对数据库。",
            ));
        }
        Ok(AnchorPage {
            image_revision,
            width_px: image.width_px,
            height_px: image.height_px,
            anchors,
        })
    }
    pub fn resolve_anchor(
        &self,
        screenshot_id: &str,
        anchor_id: &str,
    ) -> Result<AnnotationAnchor, StorageError> {
        check_id(screenshot_id)?;
        check_id(anchor_id)?;
        let page = self.anchor_page(screenshot_id)?;
        let anchor = page
            .anchors
            .into_iter()
            .find(|a| a.anchor_id == anchor_id)
            .ok_or(StorageError::Invalid(
                "图片锚点已删除或不存在，引用已失效。",
            ))?;
        if anchor.image_revision != page.image_revision {
            return Err(StorageError::Invalid(
                "锚点原图版本不匹配；停止跳转，未重定位到其他区域。",
            ));
        }
        Ok(anchor)
    }
    pub fn save_anchor(&mut self, input: AnchorInput) -> Result<AnnotationAnchor, StorageError> {
        let p = self.anchor_page(&input.screenshot_id)?;
        let title = input.title.trim();
        if input.image_revision != p.image_revision {
            return Err(StorageError::Conflict);
        }
        if title.is_empty()
            || title.chars().count() > 200
            || title.chars().any(char::is_control)
            || input.description.chars().count() > 4000
            || input.description.contains('\0')
            || ![input.x, input.y, input.width, input.height]
                .iter()
                .all(|n| n.is_finite() && *n >= 0.)
            || !((input.width == 0. && input.height == 0.)
                || (input.width > 0. && input.height > 0.))
            || input.x >= f64::from(p.width_px)
            || input.y >= f64::from(p.height_px)
            || input.x + input.width > f64::from(p.width_px)
            || input.y + input.height > f64::from(p.height_px)
        {
            return Err(StorageError::Invalid("锚点标题、说明或原图像素区域无效。"));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (id, version) = if let Some(id) = &input.anchor_id {
            check_id(id)?;
            let old=tx.query_row(&format!("SELECT {COLUMNS} FROM annotation_anchors WHERE anchor_id=?1 AND screenshot_id=?2"),params![id,input.screenshot_id],row).optional()?.ok_or(StorageError::NotFound)?;
            if Some(old.revision) != input.revision || old.image_revision != input.image_revision {
                return Err(StorageError::Conflict);
            }
            (id.clone(), old.revision + 1)
        } else {
            if input.revision.is_some() || p.anchors.len() >= 200 {
                return Err(StorageError::Invalid(
                    "每张图片最多200个锚点；新增锚点不能指定旧版本。",
                ));
            }
            (
                tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| {
                    r.get::<_, String>(0)
                })?,
                1,
            )
        };
        tx.execute("INSERT INTO annotation_anchors VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(anchor_id) DO UPDATE SET x=excluded.x,y=excluded.y,width=excluded.width,height=excluded.height,title=excluded.title,description=excluded.description,revision=excluded.revision,updated_at=excluded.updated_at",params![id,input.screenshot_id,input.image_revision,input.x,input.y,input.width,input.height,title,input.description,version,now()?])?;
        let anchor = tx.query_row(
            &format!("SELECT {COLUMNS} FROM annotation_anchors WHERE anchor_id=?1"),
            [id],
            row,
        )?;
        tx.commit()?;
        Ok(anchor)
    }
    pub fn delete_anchor(
        &mut self,
        screenshot_id: &str,
        id: &str,
        revision: i64,
    ) -> Result<(), StorageError> {
        check_id(screenshot_id)?;
        check_id(id)?;
        if self.connection.execute("DELETE FROM annotation_anchors WHERE screenshot_id=?1 AND anchor_id=?2 AND revision=?3",params![screenshot_id,id,revision])?!=1{return Err(StorageError::Conflict);}
        Ok(())
    }
}
