use super::{
    Storage, StorageError,
    library::{check_id, now},
    search::pattern,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    path::Path,
};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexedLink {
    #[serde(default)]
    pub anchor_id: Option<String>,
    pub target_id: String,
    pub link_type: String,
    pub start: usize,
    pub end: usize,
    pub label: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkSource {
    pub markdown: String,
    pub stamp: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkNode {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub project_id: Option<String>,
    pub available: bool,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphFilter {
    pub project_id: Option<String>,
    #[serde(default)]
    pub unclassified: bool,
    pub tag_id: Option<String>,
    pub focus: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkGraph {
    pub nodes: Vec<LinkNode>,
    pub edges: Vec<GraphEdge>,
    pub truncated: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetPage {
    pub items: Vec<LinkNode>,
    pub more: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backlink {
    pub source_id: String,
    pub title: String,
    pub start: usize,
    pub line: usize,
    pub label: String,
    pub available: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingLink {
    pub anchor_id: Option<String>,
    pub id: String,
    pub kind: String,
    pub label: String,
    pub start: usize,
    pub line: usize,
    pub available: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkDetails {
    pub backlinks: Vec<Backlink>,
    pub outgoing: Vec<OutgoingLink>,
    pub tag_ids: Vec<String>,
    pub truncated: bool,
    pub indexed: bool,
    pub stale: bool,
}

pub fn check_target(kind: &str, id: &str) -> Result<(), StorageError> {
    check_id(id)?;
    if !matches!(kind, "document" | "screenshot") {
        return Err(StorageError::Invalid("内部引用类型无效。"));
    }
    Ok(())
}
fn read_source(path: &Path) -> Result<LinkSource, StorageError> {
    super::files::ordinary(path)?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take((crate::services::markdown_document::MAX_TEXT + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > crate::services::markdown_document::MAX_TEXT {
        return Err(StorageError::Invalid("文档超过2MiB，链接索引未更新。"));
    }
    let stamp = format!("{:x}", Sha256::digest(&bytes));
    let markdown = String::from_utf8(bytes)
        .map_err(|_| StorageError::Invalid("文档不是有效UTF-8，链接索引未更新。"))?;
    Ok(LinkSource { markdown, stamp })
}
fn source_offsets(
    text: &str,
    links: &[IndexedLink],
) -> Result<Vec<(usize, usize, usize)>, StorageError> {
    let wanted: HashSet<usize> = links.iter().flat_map(|l| [l.start, l.end]).collect();
    let mut found = HashMap::new();
    let mut units = 0;
    let mut line = 1;
    for (byte, c) in text.char_indices() {
        if wanted.contains(&units) {
            found.insert(units, (byte, line));
        }
        units += c.len_utf16();
        if c == '\n' {
            line += 1;
        }
    }
    found.insert(units, (text.len(), line));
    links
        .iter()
        .map(|l| {
            let a = found
                .get(&l.start)
                .ok_or(StorageError::Invalid("引用位置无效。"))?;
            let b = found
                .get(&l.end)
                .ok_or(StorageError::Invalid("引用位置无效。"))?;
            Ok((a.0, b.0, a.1))
        })
        .collect()
}
fn checked_span<'a>(
    source: &'a str,
    link: &IndexedLink,
    start: usize,
    end: usize,
) -> Result<&'a str, StorageError> {
    check_target(&link.link_type, &link.target_id)?;
    if link.label.chars().count() > 200
        || link.label.chars().any(char::is_control)
        || link.start >= link.end
    {
        return Err(StorageError::Invalid("内部引用位置或名称无效。"));
    }
    let raw = source
        .get(start..end)
        .ok_or(StorageError::Invalid("引用位置越界。"))?;
    let anchor_suffix = if let Some(id) = &link.anchor_id {
        check_id(id)?;
        if link.link_type != "screenshot" {
            return Err(StorageError::Invalid("锚点只能引用截图。"));
        }
        format!("/anchor/{id}")
    } else {
        String::new()
    };
    let suffix = format!(
        "](researchclip://{}/{}{})",
        link.link_type, link.target_id, anchor_suffix
    );
    if !raw.starts_with('[') || !raw.ends_with(&suffix) || raw.len() > 4096 {
        return Err(StorageError::Invalid("引用内容与已保存文档不一致。"));
    }
    Ok(raw)
}
const NODES: &str = r#"WITH nodes AS (
 SELECT 'document' kind,d.id,d.name title,d.project_id,d.path FROM markdown_documents d
 UNION ALL SELECT 'screenshot',s.id,s.title,p.project_id,NULL FROM screenshots s LEFT JOIN screenshot_projects p ON p.screenshot_id=s.id
 UNION ALL SELECT l.link_type,l.target_id,min(l.label),NULL,NULL FROM link_index l
 WHERE (l.link_type='document' AND NOT EXISTS(SELECT 1 FROM markdown_documents d WHERE d.id=l.target_id)) OR (l.link_type='screenshot' AND NOT EXISTS(SELECT 1 FROM screenshots s WHERE s.id=l.target_id)) GROUP BY l.link_type,l.target_id
) SELECT kind,id,title,project_id,path FROM nodes n WHERE
 (?1 IS NULL OR project_id=?1) AND (?2=0 OR project_id IS NULL)
 AND (?3 IS NULL OR (kind='document' AND EXISTS(SELECT 1 FROM document_tags dt WHERE dt.document_id=n.id AND dt.tag_id=?3)) OR (kind='screenshot' AND EXISTS(SELECT 1 FROM screenshot_tags st WHERE st.screenshot_id=n.id AND st.tag_id=?3)))
 AND (?4='' OR title LIKE ?4 ESCAPE '\')
 AND (?5 IS NULL OR kind||':'||id=?5 OR EXISTS(SELECT 1 FROM link_index l WHERE ('document:'||l.source_id=?5 AND l.link_type||':'||l.target_id=n.kind||':'||n.id) OR (l.link_type||':'||l.target_id=?5 AND n.kind='document' AND l.source_id=n.id)))
 ORDER BY title COLLATE NOCASE,kind,id LIMIT ?6 OFFSET ?7"#;
impl Storage {
    pub fn link_source(&mut self, id: &str) -> Result<LinkSource, StorageError> {
        let path = self.indexed_document_path(id)?;
        match read_source(&path) {
            Ok(s) => Ok(s),
            Err(e) => {
                self.connection.execute(
                    "UPDATE link_index_state SET available=0 WHERE document_id=?1",
                    [id],
                )?;
                Err(e)
            }
        }
    }
    pub fn replace_links(
        &mut self,
        id: &str,
        stamp: &str,
        links: &[IndexedLink],
    ) -> Result<(), StorageError> {
        check_id(id)?;
        if stamp.len() != 64 || links.len() > 5000 {
            return Err(StorageError::Invalid("链接索引数据超过限制。"));
        }
        let source = self.link_source(id)?;
        if source.stamp != stamp {
            return Err(StorageError::Conflict);
        }
        let mut previous = 0;
        let mut rows = Vec::new();
        let offsets = source_offsets(&source.markdown, links)?;
        for (link, (start, end, line)) in links.iter().zip(offsets) {
            if link.start < previous {
                return Err(StorageError::Invalid("引用位置重叠或未排序。"));
            }
            checked_span(&source.markdown, link, start, end)?;
            previous = link.end;
            rows.push((link, line));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let time = now()?;
        tx.execute("DELETE FROM link_index WHERE source_id=?1", [id])?;
        for (l, line) in rows {
            tx.execute(
                "INSERT INTO link_index VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    id,
                    l.target_id,
                    l.link_type,
                    l.start as i64,
                    l.end as i64,
                    line as i64,
                    l.label,
                    time,
                    l.anchor_id
                ],
            )?;
        }
        tx.execute("INSERT INTO link_index_state VALUES(?1,?2,1,?3) ON CONFLICT(document_id) DO UPDATE SET source_stamp=excluded.source_stamp,available=1,updated_at=excluded.updated_at",params![id,stamp,time])?;
        tx.commit()?;
        Ok(())
    }
    fn link_nodes(
        &self,
        filter: &GraphFilter,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LinkNode>, StorageError> {
        for id in [filter.project_id.as_deref(), filter.tag_id.as_deref()]
            .into_iter()
            .flatten()
        {
            check_id(id)?;
        }
        if let Some(focus) = &filter.focus {
            let (k, id) = focus
                .split_once(':')
                .ok_or(StorageError::Invalid("聚焦节点无效。"))?;
            check_target(k, id)?;
        }
        if query.chars().count() > 200 {
            return Err(StorageError::Invalid("搜索词最多200个字符。"));
        }
        let q = if query.trim().is_empty() {
            String::new()
        } else {
            pattern(query.trim())
        };
        let mut nodes = self
            .connection
            .prepare(NODES)?
            .query_map(
                params![
                    filter.project_id,
                    filter.unclassified,
                    filter.tag_id,
                    q,
                    filter.focus,
                    limit,
                    offset
                ],
                |r| {
                    Ok(LinkNode {
                        kind: r.get(0)?,
                        id: r.get(1)?,
                        title: r.get(2)?,
                        project_id: r.get(3)?,
                        available: true,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for n in &mut nodes {
            n.available = self.link_target_available(&n.kind, &n.id)?;
        }
        Ok(nodes)
    }
    fn link_target_available(&self, kind: &str, id: &str) -> Result<bool, StorageError> {
        if kind == "document" {
            let path = self
                .connection
                .query_row(
                    "SELECT path FROM markdown_documents WHERE id=?1",
                    [id],
                    |r| r.get::<_, String>(0),
                )
                .optional()?;
            Ok(path.is_some_and(|p| {
                Path::new(&p).is_file() && super::files::ordinary(Path::new(&p)).is_ok()
            }))
        } else {
            Ok(self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM screenshots WHERE id=?1)",
                [id],
                |r| r.get(0),
            )?)
        }
    }
    pub fn link_targets(&self, query: &str, page: u32) -> Result<TargetPage, StorageError> {
        if page > 10000 {
            return Err(StorageError::Invalid("候选页码无效。"));
        }
        let mut items =
            self.link_nodes(&GraphFilter::default(), query, 31, i64::from(page) * 30)?;
        let more = items.len() > 30;
        items.truncate(30);
        Ok(TargetPage { items, more })
    }
    pub fn link_graph(&self, filter: &GraphFilter) -> Result<LinkGraph, StorageError> {
        let mut nodes = self.link_nodes(filter, "", 1001, 0)?;
        let mut truncated = nodes.len() > 1000;
        nodes.truncate(1000);
        let keys: HashSet<String> = nodes
            .iter()
            .map(|n| format!("{}:{}", n.kind, n.id))
            .collect();
        let mut statement=self.connection.prepare("SELECT DISTINCT source_id,link_type,target_id FROM link_index ORDER BY source_id,link_type,target_id")?;
        let all = statement.query_map([], |r| {
            Ok(GraphEdge {
                source: format!("document:{}", r.get::<_, String>(0)?),
                target: format!("{}:{}", r.get::<_, String>(1)?, r.get::<_, String>(2)?),
            })
        })?;
        let mut edges = Vec::new();
        for edge in all {
            let e = edge?;
            if keys.contains(&e.source) && keys.contains(&e.target) {
                if edges.len() == 5000 {
                    truncated = true;
                    break;
                }
                edges.push(e);
            }
        }
        Ok(LinkGraph {
            nodes,
            edges,
            truncated,
        })
    }
    pub fn link_details(&self, kind: &str, id: &str) -> Result<LinkDetails, StorageError> {
        check_target(kind, id)?;
        let mut backlinks=self.connection.prepare("SELECT l.source_id,d.name,l.source_location,l.source_line,l.label,d.path FROM link_index l JOIN markdown_documents d ON d.id=l.source_id WHERE l.link_type=?1 AND l.target_id=?2 ORDER BY d.name,l.source_location LIMIT 501")?.query_map(params![kind,id],|r|{let p:String=r.get(5)?;Ok(Backlink{source_id:r.get(0)?,title:r.get(1)?,start:r.get::<_,i64>(2)? as usize,line:r.get::<_,i64>(3)? as usize,label:r.get(4)?,available:Path::new(&p).is_file()&&super::files::ordinary(Path::new(&p)).is_ok()})})?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut outgoing = if kind == "document" {
            self.connection.prepare("SELECT target_id,link_type,label,source_location,source_line,anchor_id FROM link_index WHERE source_id=?1 ORDER BY source_location LIMIT 501")?.query_map([id],|r|Ok(OutgoingLink{anchor_id:r.get(5)?,id:r.get(0)?,kind:r.get(1)?,label:r.get(2)?,start:r.get::<_,i64>(3)? as usize,line:r.get::<_,i64>(4)? as usize,available:false}))?.collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            Vec::new()
        };
        let mut anchors = HashMap::new();
        for link in &mut outgoing {
            link.available = if let Some(anchor_id) = &link.anchor_id {
                if !anchors.contains_key(&link.id) {
                    anchors.insert(link.id.clone(), self.available_anchors(&link.id)?);
                }
                anchors[&link.id].contains_key(anchor_id)
            } else {
                self.link_target_available(&link.kind, &link.id)?
            };
        }
        let tag_ids = if kind == "document" {
            self.connection
                .prepare("SELECT tag_id FROM document_tags WHERE document_id=?1 ORDER BY tag_id")?
                .query_map([id], |r| r.get(0))?
                .collect::<rusqlite::Result<Vec<String>>>()?
        } else {
            Vec::new()
        };
        let stamp = self
            .connection
            .query_row(
                "SELECT source_stamp FROM link_index_state WHERE document_id=?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        let stale = if kind == "document" {
            match (
                stamp.as_deref(),
                self.indexed_document_path(id).and_then(|p| read_source(&p)),
            ) {
                (Some(a), Ok(b)) => a != b.stamp,
                _ => true,
            }
        } else {
            false
        };
        let truncated = backlinks.len() > 500 || outgoing.len() > 500;
        backlinks.truncate(500);
        outgoing.truncate(500);
        Ok(LinkDetails {
            backlinks,
            outgoing,
            tag_ids,
            truncated,
            indexed: stamp.is_some(),
            stale,
        })
    }
    pub fn set_document_link_tags(
        &mut self,
        id: &str,
        tags: &[String],
    ) -> Result<(), StorageError> {
        check_id(id)?;
        if tags.len() > 100 {
            return Err(StorageError::Invalid("文档最多100个标签。"));
        }
        for tag in tags {
            check_id(tag)?;
        }
        self.indexed_document_path(id)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM document_tags WHERE document_id=?1", [id])?;
        for tag in tags {
            tx.execute(
                "INSERT OR IGNORE INTO document_tags VALUES(?1,?2)",
                params![id, tag],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn portable_links(
        &mut self,
        id: &str,
        destination: &Path,
    ) -> Result<(String, Vec<String>), StorageError> {
        let source = self.link_source(id)?;
        let stamp = self
            .connection
            .query_row(
                "SELECT source_stamp FROM link_index_state WHERE document_id=?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        if stamp.as_deref() != Some(&source.stamp) {
            return Err(StorageError::Invalid("请先更新当前文档链接索引，再导出。"));
        }
        let links=self.connection.prepare("SELECT target_id,link_type,source_location,source_end,label,anchor_id FROM link_index WHERE source_id=?1 ORDER BY source_location DESC")?.query_map([id],|r|Ok(IndexedLink{anchor_id:r.get(5)?,target_id:r.get(0)?,link_type:r.get(1)?,start:r.get::<_,i64>(2)? as usize,end:r.get::<_,i64>(3)? as usize,label:r.get(4)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let parent = destination
            .parent()
            .ok_or(StorageError::Invalid("导出目录无效。"))?
            .canonicalize()?;
        let offsets = source_offsets(&source.markdown, &links)?;
        let mut text = source.markdown;
        let mut warnings = Vec::new();
        let mut anchors = HashMap::new();
        for (l, (start, end, _)) in links.into_iter().zip(offsets) {
            let raw = checked_span(&text, &l, start, end)?.to_owned();
            let relative = if l.link_type == "document"
                && self.link_target_available("document", &l.target_id)?
            {
                relative_url(&parent, &self.indexed_document_path(&l.target_id)?)
            } else {
                None
            };
            let replacement = if let Some(anchor_id) = &l.anchor_id {
                let label = l.label.replace(['[', ']', '<', '>', '\\'], " ");
                if !anchors.contains_key(&l.target_id) {
                    anchors.insert(l.target_id.clone(), self.available_anchors(&l.target_id)?);
                }
                match anchors[&l.target_id].get(anchor_id) {
                    Some(a) => format!(
                        "{}（图片区域：x={}，y={}，宽={}，高={}；{}）",
                        label,
                        a.x,
                        a.y,
                        a.width,
                        a.height,
                        a.description
                            .replace(['\n', '\r', '<', '>', '[', ']', '\\', '`', '*', '_'], " ")
                    ),
                    None => {
                        warnings.push(format!("{label}：锚点已失效，保留文字提示。"));
                        format!("{label}（图片锚点已失效）")
                    }
                }
            } else if let Some(url) = relative {
                let at = raw
                    .rfind("](researchclip://")
                    .ok_or(StorageError::Conflict)?;
                format!("{}]({url})", &raw[..at])
            } else {
                warnings.push(format!(
                    "{}：{}无法导出为文件链接；保留文字提示。",
                    l.label,
                    if l.link_type == "screenshot" {
                        "截图卡片"
                    } else {
                        "目标缺失或跨磁盘"
                    }
                ));
                format!(
                    "{}（内部引用未导出）",
                    l.label.replace(['[', ']', '<', '>', '\\'], " ")
                )
            };
            text.replace_range(start..end, &replacement);
        }
        if text.contains("researchclip://") {
            warnings.push(
                "导出源码仍含researchclip://字样，可能是示例代码或不支持的引用格式，请核对。"
                    .into(),
            );
        }
        if !warnings.is_empty() {
            text.push_str("\n\n> 部分内部引用未导出；请在ResearchClip中查看原文档。\n");
        }
        Ok((text, warnings))
    }
}
fn relative_url(from: &Path, to: &Path) -> Option<String> {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let same = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if same == 0 {
        return None;
    }
    let mut parts = vec!["..".to_owned(); a.len() - same];
    for component in &b[same..] {
        let text = component.as_os_str().to_str()?;
        let encoded = text
            .as_bytes()
            .iter()
            .map(|c| {
                if c.is_ascii_alphanumeric() || b"-._~".contains(c) {
                    (*c as char).to_string()
                } else {
                    format!("%{c:02X}")
                }
            })
            .collect::<String>();
        parts.push(encoded);
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anchor_index_export_and_deleted_target_preserve_readable_text() {
        use crate::{
            capture::RawFrame, services::screen_capture::encode_png, storage::anchors::AnchorInput,
        };
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().join("data")).unwrap();
        let png = encode_png(RawFrame {
            width_px: 100,
            height_px: 80,
            rgba: [0, 40, 80, 255].repeat(100 * 80),
        })
        .unwrap();
        let shot=s.create_screenshot("图",serde_json::json!({"schemaVersion":1,"image":{"widthPx":100,"heightPx":80},"objects":[]}),&png).unwrap();
        let page = s.anchor_page(&shot.id).unwrap();
        let a = s
            .save_anchor(AnchorInput {
                anchor_id: None,
                revision: None,
                screenshot_id: shot.id.clone(),
                image_revision: page.image_revision,
                x: 10.,
                y: 12.,
                width: 20.,
                height: 30.,
                title: "区域".into(),
                description: "说明".into(),
            })
            .unwrap();
        let text = format!(
            "中文😀 [区域](researchclip://screenshot/{}/anchor/{})",
            shot.id, a.anchor_id
        );
        let start = "中文😀 ".encode_utf16().count();
        let input = IndexedLink {
            anchor_id: Some(a.anchor_id.clone()),
            target_id: shot.id.clone(),
            link_type: "screenshot".into(),
            start,
            end: text.encode_utf16().count(),
            label: "区域".into(),
        };
        let id = doc(&mut s, &tmp.path().join("note.md"), &text);
        index(&mut s, &id, &[input]);
        assert!(s.link_details("document", &id).unwrap().outgoing[0].available);
        assert_eq!(
            s.link_details("screenshot", &shot.id)
                .unwrap()
                .backlinks
                .len(),
            1
        );
        let (portable, warnings) = s
            .portable_links(&id, &tmp.path().join("export.md"))
            .unwrap();
        assert!(!portable.contains("researchclip://"));
        assert!(portable.contains("x=10"));
        assert!(portable.contains("说明"));
        assert!(warnings.is_empty());
        s.delete_anchor(&shot.id, &a.anchor_id, a.revision).unwrap();
        assert!(!s.link_details("document", &id).unwrap().outgoing[0].available);
        let (portable, warnings) = s
            .portable_links(&id, &tmp.path().join("missing.md"))
            .unwrap();
        assert!(portable.contains("图片锚点已失效"));
        assert_eq!(warnings.len(), 1);
        drop(s);
        let s = Storage::open(tmp.path().join("data")).unwrap();
        assert_eq!(
            s.link_details("document", &id).unwrap().outgoing[0]
                .anchor_id
                .as_deref(),
            Some(a.anchor_id.as_str())
        );
    }
    fn doc(s: &mut Storage, p: &Path, text: &str) -> String {
        std::fs::write(p, text).unwrap();
        s.index_document(p, None).unwrap().id
    }
    fn link(kind: &str, id: &str, label: &str, start: usize) -> (String, IndexedLink) {
        let raw = format!("[{label}](researchclip://{kind}/{id})");
        let end = start + raw.encode_utf16().count();
        (
            raw,
            IndexedLink {
                anchor_id: None,
                target_id: id.into(),
                link_type: kind.into(),
                start,
                end,
                label: label.into(),
            },
        )
    }
    fn index(s: &mut Storage, id: &str, links: &[IndexedLink]) {
        let source = s.link_source(id).unwrap();
        s.replace_links(id, &source.stamp, links).unwrap();
    }
    #[test]
    fn references_rename_missing_target_cycles_and_reopen_are_consistent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("workspace");
        let mut s = Storage::open(root.clone()).unwrap();
        let a_path = tmp.path().join("A.md");
        let b_path = tmp.path().join("B.md");
        let a = doc(&mut s, &a_path, "");
        let b = doc(&mut s, &b_path, "");
        let prefix = "中文😀\r\n";
        let (raw, ab) = link("document", &b, "B", prefix.encode_utf16().count());
        std::fs::write(&a_path, format!("{prefix}{raw}")).unwrap();
        index(&mut s, &a, &[ab]);
        let (raw, ba) = link("document", &a, "A", 0);
        std::fs::write(&b_path, raw).unwrap();
        index(&mut s, &b, &[ba]);
        let details = s.link_details("document", &b).unwrap();
        assert_eq!(details.backlinks.len(), 1);
        assert_eq!(details.backlinks[0].line, 2);
        assert_eq!(details.backlinks[0].start, prefix.encode_utf16().count());
        assert!(!details.stale);
        assert_eq!(
            s.link_graph(&GraphFilter::default()).unwrap().edges.len(),
            2
        );
        let renamed = tmp.path().join("新的名字.md");
        std::fs::rename(&b_path, &renamed).unwrap();
        assert_eq!(s.index_document(&renamed, Some(&b_path)).unwrap().id, b);
        assert!(s.link_details("document", &a).unwrap().outgoing[0].available);
        assert!(
            s.link_targets("新的名字", 0)
                .unwrap()
                .items
                .iter()
                .any(|n| n.id == b)
        );
        std::fs::remove_file(&renamed).unwrap();
        assert!(!s.link_details("document", &a).unwrap().outgoing[0].available);
        assert!(s.link_source(&b).is_err());
        assert_eq!(s.link_details("document", &b).unwrap().backlinks.len(), 1);
        drop(s);
        let s = Storage::open(root).unwrap();
        assert_eq!(
            s.link_graph(&GraphFilter::default()).unwrap().edges.len(),
            2
        );
        assert!(!s.link_details("document", &a).unwrap().outgoing[0].available);
    }
    #[test]
    fn stale_positions_malicious_ids_and_duplicate_spans_preserve_prior_index() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().join("w")).unwrap();
        let path = tmp.path().join("A.md");
        let a = doc(&mut s, &path, "");
        let (raw, l) = link("document", &a, "A", 0);
        std::fs::write(&path, &raw).unwrap();
        index(&mut s, &a, std::slice::from_ref(&l));
        let snapshot = s.link_source(&a).unwrap();
        assert!(
            s.replace_links(&a, &snapshot.stamp, &[l.clone(), l.clone()])
                .is_err()
        );
        let mut bad = l.clone();
        bad.target_id = "../outside".into();
        assert!(s.replace_links(&a, &snapshot.stamp, &[bad]).is_err());
        std::fs::write(&path, format!("😀{raw}")).unwrap();
        assert!(matches!(
            s.replace_links(&a, &snapshot.stamp, &[]),
            Err(StorageError::Conflict)
        ));
        let fresh = s.link_source(&a).unwrap();
        let mut bad = l;
        bad.start = 1;
        assert!(s.replace_links(&a, &fresh.stamp, &[bad]).is_err());
        assert_eq!(s.link_details("document", &a).unwrap().outgoing.len(), 1);
        assert!(s.link_details("document", &a).unwrap().stale);
        assert!(s.link_source("../../secret").is_err());
        assert!(s.link_source(&"f".repeat(32)).is_err());
    }
    #[test]
    fn project_tag_neighbor_filters_and_atomic_tag_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().join("w")).unwrap();
        let a_path = tmp.path().join("A_100%.md");
        let a = doc(&mut s, &a_path, "");
        let b = doc(&mut s, &tmp.path().join("B.md"), "");
        let c = doc(&mut s, &tmp.path().join("C.md"), "");
        let (raw, l) = link("document", &b, "B", 0);
        std::fs::write(&a_path, raw).unwrap();
        index(&mut s, &a, &[l]);
        s.save_project(None, "研究").unwrap();
        let project = s.projects().unwrap()[0].id.clone();
        s.assign_document(&a, Some(&project)).unwrap();
        let tag = s.save_tag(None, "方法").unwrap();
        s.set_document_link_tags(&a, std::slice::from_ref(&tag.id))
            .unwrap();
        let filtered = s
            .link_graph(&GraphFilter {
                project_id: Some(project.clone()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered.nodes.len(), 1);
        assert_eq!(filtered.nodes[0].id, a);
        assert!(filtered.edges.is_empty());
        assert_eq!(
            s.link_graph(&GraphFilter {
                tag_id: Some(tag.id.clone()),
                ..Default::default()
            })
            .unwrap()
            .nodes
            .len(),
            1
        );
        let neighbors = s
            .link_graph(&GraphFilter {
                focus: Some(format!("document:{a}")),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(neighbors.nodes.len(), 2);
        assert!(!neighbors.nodes.iter().any(|n| n.id == c));
        assert_eq!(neighbors.edges.len(), 1);
        assert!(s.set_document_link_tags(&a, &["f".repeat(32)]).is_err());
        assert_eq!(
            s.link_details("document", &a).unwrap().tag_ids,
            vec![tag.id]
        );
        assert_eq!(s.link_targets("_100%", 0).unwrap().items.len(), 1);
        assert!(
            s.link_graph(&GraphFilter {
                project_id: Some("../".into()),
                ..Default::default()
            })
            .is_err()
        );
    }
    #[test]
    fn deleted_screenshot_is_a_broken_node_and_portable_export_warns() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().join("w")).unwrap();
        let png = crate::services::screen_capture::encode_png(crate::capture::RawFrame {
            width_px: 2,
            height_px: 2,
            rgba: [1, 2, 3, 255].repeat(4),
        })
        .unwrap();
        let shot=s.create_screenshot("Screenshot",serde_json::json!({"schemaVersion":1,"image":{"widthPx":2,"heightPx":2},"objects":[]}),&png).unwrap();
        let a_path = tmp.path().join("A.md");
        let a = doc(&mut s, &a_path, "");
        let b = doc(&mut s, &tmp.path().join("中文 B.md"), "");
        let (one, l1) = link("document", &b, "中文 B", 0);
        let (two, l2) = link("screenshot", &shot.id, "图", one.encode_utf16().count() + 2);
        std::fs::write(&a_path, format!("{one}\n\n{two}")).unwrap();
        index(&mut s, &a, &[l1, l2]);
        assert_eq!(
            s.link_details("screenshot", &shot.id)
                .unwrap()
                .backlinks
                .len(),
            1
        );
        let (portable, warnings) = s.portable_links(&a, &tmp.path().join("export.md")).unwrap();
        assert!(portable.contains("%E4%B8%AD%E6%96%87%20B.md"));
        assert!(!portable.contains("researchclip://"));
        assert_eq!(warnings.len(), 1);
        s.delete_screenshot(&shot.id, shot.revision).unwrap();
        let graph = s.link_graph(&GraphFilter::default()).unwrap();
        assert!(graph.nodes.iter().any(|n| n.id == shot.id && !n.available));
        assert!(!s.link_details("document", &a).unwrap().outgoing[1].available);
    }
    #[test]
    fn schema8_upgrade_backs_up_and_unknown_future_schema_refuses_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("w");
        let s = Storage::open(root.clone()).unwrap();
        let db = s.database_path.clone();
        drop(s);
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("DROP TABLE annotation_anchors;DROP TABLE document_tags; DROP TABLE link_index_state; DROP TABLE link_index; PRAGMA user_version=8;").unwrap();
        drop(conn);
        let s = Storage::open(root.clone()).unwrap();
        assert_eq!(s.status().unwrap().schema_version, 10);
        let backups = std::fs::read_dir(root.join("backups"))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(backups.len(), 1);
        let backup = rusqlite::Connection::open(backups[0].path()).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            8
        );
        drop(s);
        let conn = rusqlite::Connection::open(db).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        drop(conn);
        assert!(matches!(
            Storage::open(root),
            Err(StorageError::NewerSchema(99))
        ));
    }
    #[test]
    fn interrupted_v9_upgrade_rolls_back_and_keeps_v8_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("w");
        let s = Storage::open(root.clone()).unwrap();
        let path = s.database_path.clone();
        drop(s);
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("DROP TABLE annotation_anchors;DROP TABLE document_tags; DROP TABLE link_index_state; DROP TABLE link_index; CREATE TABLE link_index_state(unexpected TEXT); PRAGMA user_version=8;").unwrap();
        drop(conn);
        assert!(Storage::open(root.clone()).is_err());
        let conn = rusqlite::Connection::open(path).unwrap();
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            8
        );
        assert!(conn.prepare("SELECT * FROM link_index").is_err());
        assert!(conn.prepare("SELECT * FROM document_tags").is_err());
        assert_eq!(std::fs::read_dir(root.join("backups")).unwrap().count(), 1);
    }
}
