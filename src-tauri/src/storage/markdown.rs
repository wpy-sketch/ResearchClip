use super::{Storage, StorageError, files, knowledge::KnowledgeCard};
use crate::{capture::MAX_PNG_BYTES, services::workflow::decode_png};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_EXPORT_CARDS: usize = 100;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_EXPORT_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportIssue {
    pub id: String,
    pub message: String,
}
pub struct ExportCard {
    card: KnowledgeCard,
    original: File,
}
pub struct ExportSnapshot {
    cards: Vec<ExportCard>,
    failures: Vec<ExportIssue>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub directory: String,
    pub exported_ids: Vec<String>,
    pub failures: Vec<ExportIssue>,
    pub warnings: Vec<ExportIssue>,
}

pub fn validate_ids(ids: &[String]) -> Result<(), StorageError> {
    if ids.is_empty()
        || ids.len() > MAX_EXPORT_CARDS
        || ids.iter().any(|id| !files::id_valid(id))
        || ids.iter().collect::<HashSet<_>>().len() != ids.len()
    {
        return Err(StorageError::Invalid("请选择1至100张不同的已存库卡片。"));
    }
    Ok(())
}
impl Storage {
    // Metadata shares one read snapshot. Open immutable originals while the library
    // is locked, then release both SQL and service locks before decoding/writing.
    pub fn markdown_snapshot(&self, ids: &[String]) -> Result<ExportSnapshot, StorageError> {
        validate_ids(ids)?;
        let tx = self.connection.unchecked_transaction()?;
        let mut snapshot = ExportSnapshot {
            cards: Vec::new(),
            failures: Vec::new(),
        };
        let mut text_bytes = 0;
        let mut image_bytes = 0;
        for id in ids {
            let card = match self.knowledge_card_from(&tx, id) {
                Ok(card) => card,
                Err(StorageError::NotFound) => {
                    snapshot.failures.push(issue(id, "卡片已删除，未导出。"));
                    continue;
                }
                Err(error) => return Err(error),
            };
            text_bytes += card.note.as_ref().map_or(0, |n| n.content.len());
            if text_bytes > MAX_TEXT_BYTES {
                return Err(StorageError::Invalid(
                    "所选笔记正文超过16 MiB，请分批导出。",
                ));
            }
            let original = (|| {
                let file = File::open(files::image_path(
                    &self.data_dir,
                    &card.screenshot.original_image_path,
                )?)?;
                let metadata = file.metadata()?;
                if !metadata.is_file() || metadata.len() > MAX_PNG_BYTES as u64 {
                    return Err(StorageError::Invalid("原图大小无效。"));
                }
                image_bytes += metadata.len();
                Ok::<_, StorageError>(file)
            })();
            match original {
                Ok(original) => snapshot.cards.push(ExportCard { card, original }),
                Err(_) => snapshot.failures.push(issue(
                    id,
                    "原图缺失或不可访问，卡片未导出；资料库记录已保留。",
                )),
            }
            if image_bytes > MAX_EXPORT_BYTES {
                return Err(StorageError::Invalid("所选原图超过512 MiB，请分批导出。"));
            }
        }
        tx.commit()?;
        Ok(snapshot)
    }
}
fn issue(id: &str, message: &str) -> ExportIssue {
    ExportIssue {
        id: id.into(),
        message: message.into(),
    }
}

// Only ASCII URL-safe characters enter generated filenames. Prefix and index
// also avoid Windows device names and collisions between identical titles.
fn image_name(index: usize, title: &str) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if slug.len() >= 40 {
            break;
        }
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    format!(
        "screenshot-{index:03}-{}.png",
        if slug.is_empty() { "card" } else { slug }
    )
}
fn escape(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if c.is_control() {
            out.push(' ');
        } else if c == '<' {
            out.push_str("&lt;");
        } else if c == '>' {
            out.push_str("&gt;");
        } else if c == '&' {
            out.push_str("&amp;");
        } else {
            if c.is_ascii_punctuation() {
                out.push('\\');
            }
            out.push(c);
        }
    }
    out
}
fn created_utc(stamp: i64) -> String {
    if !(0..=253_402_300_799_999).contains(&stamp) {
        return format!("Unix 毫秒：{stamp}");
    }
    let seconds = stamp / 1000;
    let mut days = seconds / 86400;
    let mut year = 1970;
    let leap = |y: i64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    while days >= if leap(year) { 366 } else { 365 } {
        days -= if leap(year) { 366 } else { 365 };
        year += 1;
    }
    let months = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    while days >= months[month] {
        days -= months[month];
        month += 1;
    }
    format!(
        "{year:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        month + 1,
        days + 1,
        seconds % 86400 / 3600,
        seconds % 3600 / 60,
        seconds % 60
    )
}
fn card_markdown(card: &KnowledgeCard, filename: &str, warnings: &mut Vec<ExportIssue>) -> String {
    let title = escape(&card.screenshot.title);
    let mut out = format!(
        "# {title}\n\n![{title}](assets/{filename})\n\n创建时间：{}\n\n## 研究笔记\n\n",
        created_utc(card.screenshot.created_at)
    );
    if let Some(note) = &card.note {
        out.push_str(&note.content);
    }
    out.push_str("\n\n## 来源\n\n");
    if let Some(url) = card.note.as_ref().and_then(|n| n.source_url.as_deref()) {
        match super::knowledge::source_url(Some(url)) {
            Ok(Some(url)) => out.push_str(&format!(
                "<{}>",
                url.replace('<', "%3C").replace('>', "%3E")
            )),
            _ => {
                out.push_str("来源链接无效，未导出。");
                warnings.push(issue(
                    &card.screenshot.id,
                    "来源不是有效的HTTP(S)链接，未导出该链接。",
                ));
            }
        }
    } else {
        out.push_str("未填写");
    }
    out.push_str("\n\n## 标签\n\n");
    out.push_str(
        &card
            .tags
            .iter()
            .map(|tag| format!("#{}", escape(&tag.name)))
            .collect::<Vec<_>>()
            .join(" "),
    );
    out.push_str("\n\n");
    if card.additional_note_count > 0 {
        warnings.push(issue(
            &card.screenshot.id,
            "本次导出知识卡片中显示的主笔记；额外既有笔记仍保留在资料库。",
        ));
    }
    out
}
fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
fn reserve_directory(parent: &Path) -> std::io::Result<PathBuf> {
    for n in 0..10_000 {
        let path = parent.join(if n == 0 {
            "ResearchClip-Export".into()
        } else {
            format!("ResearchClip-Export-{n:03}")
        });
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "No available export directory",
    ))
}
fn publish_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(destination.parent().unwrap())?;
    std::io::copy(&mut File::open(source)?, &mut file)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(destination)
        .map_err(|error| error.error)?;
    Ok(())
}

pub fn export_markdown(
    parent: &Path,
    snapshot: ExportSnapshot,
) -> Result<ExportReport, StorageError> {
    if !parent.is_absolute() || !parent.is_dir() {
        return Err(StorageError::Invalid("请选择可用的本地导出文件夹。"));
    }
    let parent = parent.canonicalize()?;
    files::ordinary(&parent)?;
    let stage = tempfile::Builder::new()
        .prefix(".researchclip-export-")
        .tempdir_in(&parent)?;
    fs::create_dir(stage.path().join("assets"))?;
    let mut report = ExportReport {
        directory: String::new(),
        exported_ids: Vec::new(),
        failures: snapshot.failures,
        warnings: Vec::new(),
    };
    let mut markdown = String::new();
    let mut names = Vec::new();
    for entry in snapshot.cards {
        let id = &entry.card.screenshot.id;
        let mut bytes = Vec::new();
        let valid = entry
            .original
            .take(MAX_PNG_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .is_ok()
            && bytes.len() <= MAX_PNG_BYTES
            && decode_png(&bytes).is_ok_and(|image| {
                image.width_px == entry.card.screenshot.width_px
                    && image.height_px == entry.card.screenshot.height_px
            });
        if !valid {
            report.failures.push(issue(
                id,
                "原图损坏、读取失败或尺寸不匹配，卡片未导出；资料库未修改。",
            ));
            continue;
        }
        let filename = image_name(report.exported_ids.len() + 1, &entry.card.screenshot.title);
        // A destination failure aborts the whole staged export; never report success.
        write_new(&stage.path().join("assets").join(&filename), &bytes)?;
        if !markdown.is_empty() {
            markdown.push_str("---\n\n");
        }
        markdown.push_str(&card_markdown(&entry.card, &filename, &mut report.warnings));
        names.push(filename);
        report.exported_ids.push(id.clone());
    }
    if report.exported_ids.is_empty() {
        return Err(StorageError::Invalid(
            "所选卡片均无法导出，请检查原图是否存在且可读取。资料库未修改。",
        ));
    }
    write_new(&stage.path().join("notes.md"), markdown.as_bytes())?;
    if !report.failures.is_empty() || !report.warnings.is_empty() {
        let issues = report
            .failures
            .iter()
            .chain(&report.warnings)
            .map(|issue| format!("{}：{}\n", issue.id, issue.message))
            .collect::<String>();
        write_new(&stage.path().join("export-report.txt"), issues.as_bytes())?;
    }
    // Portable no-clobber publication: reserve a fresh directory atomically,
    // transfer complete files, publish notes.md last, and retain a marker on error.
    // This does not claim an atomic multi-file directory transaction.
    let directory = reserve_directory(&parent)?;
    write_new(
        &directory.join("EXPORT-INCOMPLETE.txt"),
        "导出尚未完成。若没有notes.md，请保留资料库原件并重新导出。".as_bytes(),
    )?;
    let publish = || -> std::io::Result<()> {
        fs::create_dir(directory.join("assets"))?;
        for name in names {
            publish_file(
                &stage.path().join("assets").join(&name),
                &directory.join("assets").join(name),
            )?;
        }
        if stage.path().join("export-report.txt").exists() {
            publish_file(
                &stage.path().join("export-report.txt"),
                &directory.join("export-report.txt"),
            )?;
        }
        publish_file(&stage.path().join("notes.md"), &directory.join("notes.md"))?;
        fs::remove_file(directory.join("EXPORT-INCOMPLETE.txt"))?;
        Ok(())
    };
    if publish().is_err() {
        return Err(StorageError::Invalid(
            "导出提交失败，未覆盖已有文件。新导出目录可能保留EXPORT-INCOMPLETE.txt，请检查权限、磁盘空间并重新导出。",
        ));
    }
    report.directory = directory.to_string_lossy().into();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::RawFrame, services::screen_capture::encode_png};
    fn png() -> Vec<u8> {
        encode_png(RawFrame {
            width_px: 3,
            height_px: 2,
            rgba: [20, 40, 80, 128].repeat(6),
        })
        .unwrap()
    }
    fn create(storage: &mut Storage, title: &str) -> String {
        storage.create_screenshot(title,serde_json::json!({"schemaVersion":1,"image":{"widthPx":3,"heightPx":2},"objects":[]}),&png()).unwrap().id
    }
    fn contents(report: &ExportReport) -> String {
        fs::read_to_string(Path::new(&report.directory).join("notes.md")).unwrap()
    }
    fn assert_references(report: &ExportReport, expected: usize) {
        let text = contents(report);
        let paths: Vec<_> = text
            .lines()
            .filter(|line| line.starts_with("!["))
            .map(|line| {
                line.rsplit_once("](assets/")
                    .unwrap()
                    .1
                    .strip_suffix(')')
                    .unwrap()
            })
            .collect();
        assert_eq!(paths.len(), expected);
        for name in paths {
            assert!(!name.contains(['\\', '/', ':', ' ', '%', '(', ')']));
            let bytes = fs::read(Path::new(&report.directory).join("assets").join(name)).unwrap();
            assert_eq!(bytes, png());
            let frame = decode_png(&bytes).unwrap();
            assert_eq!((frame.width_px, frame.height_px), (3, 2));
            assert_eq!(&frame.rgba[..4], &[20, 40, 80, 128]);
        }
        assert!(
            !Path::new(&report.directory)
                .join("EXPORT-INCOMPLETE.txt")
                .exists()
        );
    }
    #[test]
    fn single_card_exports_markdown_fields_relative_png_and_never_internal_paths() {
        let root = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(root.path().join("工作区")).unwrap();
        let id = create(&mut storage, "Apollonius [Circle] <math>");
        let body = "## 观察\n\n公式 $x^2+y^2=r^2$\n\n```rust\nlet a = 1;\n```";
        storage
            .save_note(
                &id,
                None,
                None,
                body,
                Some("https://example.org/paper?q=%E5%87%A0%E4%BD%95"),
            )
            .unwrap();
        let tag = storage.save_tag(None, "几何方法").unwrap();
        storage.set_screenshot_tags(&id, 1, &[tag.id]).unwrap();
        let parent = root.path().join("导出 [研究] # 100%");
        fs::create_dir(&parent).unwrap();
        let report = export_markdown(
            &parent,
            storage
                .markdown_snapshot(std::slice::from_ref(&id))
                .unwrap(),
        )
        .unwrap();
        assert_references(&report, 1);
        let text = contents(&report);
        assert!(text.contains("# Apollonius \\[Circle\\] &lt;math&gt;"));
        assert!(text.contains(body));
        assert!(text.contains("## 来源\n\n<https://example.org/"));
        assert!(text.contains("## 标签\n\n#几何方法"));
        assert!(text.contains("创建时间：") && text.contains('Z'));
        assert!(!text.contains("originals/") && !text.contains("thumbnails/"));
        assert!(!text.contains(&root.path().to_string_lossy().to_string()));
        assert!(report.failures.is_empty());
        assert_eq!(storage.screenshot_image(&id, false).unwrap(), png());
    }
    #[test]
    fn multi_card_same_titles_and_windows_special_names_have_unique_safe_references() {
        let root = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(root.path().join("db")).unwrap();
        let ids: Vec<_> = ["CON", "CON", "..\\<科研>:/?*\" | #()", "中文"]
            .iter()
            .map(|title| create(&mut storage, title))
            .collect();
        let report =
            export_markdown(root.path(), storage.markdown_snapshot(&ids).unwrap()).unwrap();
        assert_eq!(report.exported_ids, ids);
        assert_references(&report, 4);
        assert_eq!(
            fs::read_dir(Path::new(&report.directory).join("assets"))
                .unwrap()
                .count(),
            4
        );
    }
    #[test]
    fn repeated_exports_reserve_fresh_folders_and_do_not_touch_existing_files() {
        let root = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(root.path().join("db")).unwrap();
        let id = create(&mut storage, "A");
        let existing = root.path().join("ResearchClip-Export");
        fs::create_dir(&existing).unwrap();
        write_new(&existing.join("notes.md"), b"user data").unwrap();
        write_new(&root.path().join("ResearchClip-Export-001"), b"user file").unwrap();
        let first = export_markdown(
            root.path(),
            storage
                .markdown_snapshot(std::slice::from_ref(&id))
                .unwrap(),
        )
        .unwrap();
        assert!(first.directory.ends_with("ResearchClip-Export-002"));
        let second =
            export_markdown(root.path(), storage.markdown_snapshot(&[id]).unwrap()).unwrap();
        assert!(second.directory.ends_with("ResearchClip-Export-003"));
        assert_eq!(fs::read(existing.join("notes.md")).unwrap(), b"user data");
        assert_eq!(
            fs::read(root.path().join("ResearchClip-Export-001")).unwrap(),
            b"user file"
        );
        assert_references(&first, 1);
        assert_references(&second, 1);
    }
    #[test]
    fn missing_deleted_and_corrupt_images_allow_explicit_partial_export_without_broken_links() {
        let root = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(root.path().join("db")).unwrap();
        let a = create(&mut storage, "good");
        let b = create(&mut storage, "missing");
        let c = create(&mut storage, "corrupt");
        fs::remove_file(storage.data_dir.join(format!("originals/{b}.png"))).unwrap();
        fs::write(
            storage.data_dir.join(format!("originals/{c}.png")),
            b"bad image",
        )
        .unwrap();
        let absent = "f".repeat(32);
        let report = export_markdown(
            root.path(),
            storage
                .markdown_snapshot(&[a.clone(), b.clone(), c.clone(), absent])
                .unwrap(),
        )
        .unwrap();
        assert_eq!(report.exported_ids, vec![a]);
        assert_eq!(report.failures.len(), 3);
        assert_references(&report, 1);
        let errors =
            fs::read_to_string(Path::new(&report.directory).join("export-report.txt")).unwrap();
        assert!(errors.contains(&b) && errors.contains(&c));
        assert!(!errors.contains(&storage.data_dir.to_string_lossy().to_string()));
        assert!(storage.knowledge_card(&b).is_ok());
        assert!(storage.knowledge_card(&c).is_ok());
    }
    #[test]
    fn all_failed_exports_publish_nothing_and_do_not_remove_records() {
        let root = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(root.path().join("db")).unwrap();
        let id = create(&mut storage, "missing");
        fs::remove_file(storage.data_dir.join(format!("originals/{id}.png"))).unwrap();
        assert!(
            export_markdown(
                root.path(),
                storage
                    .markdown_snapshot(std::slice::from_ref(&id))
                    .unwrap()
            )
            .is_err()
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        assert!(storage.knowledge_card(&id).is_ok());
    }
    #[test]
    fn snapshot_preserves_saved_metadata_and_original_handle_after_changes() {
        let root = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(root.path().join("db")).unwrap();
        let id = create(&mut storage, "Before");
        storage
            .save_note(&id, None, None, "old body", None)
            .unwrap();
        let snapshot = storage
            .markdown_snapshot(std::slice::from_ref(&id))
            .unwrap();
        let card = storage.knowledge_card(&id).unwrap();
        let note = card.note.unwrap();
        storage
            .save_note(&id, Some(&note.id), Some(note.revision), "new body", None)
            .unwrap();
        storage
            .delete_screenshot(&id, card.screenshot.revision)
            .unwrap();
        let report = export_markdown(root.path(), snapshot).unwrap();
        assert!(contents(&report).contains("old body"));
        assert!(!contents(&report).contains("new body"));
        assert_references(&report, 1);
    }
    #[test]
    fn invalid_legacy_source_and_extra_notes_are_reported_without_exporting_local_paths() {
        let root = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(root.path().join("db")).unwrap();
        let id = create(&mut storage, "source");
        let primary = storage.save_note(&id, None, None, "primary", None).unwrap();
        // Simulate legacy/corrupt data; the current business API rejects file URLs.
        storage
            .connection
            .execute(
                "UPDATE notes SET source_url=?1 WHERE id=?2",
                rusqlite::params!["file:///C:/private/paper.pdf", primary.id],
            )
            .unwrap();
        let additional = storage
            .save_note(&id, None, None, "additional", None)
            .unwrap();
        storage
            .connection
            .execute(
                "UPDATE notes SET created_at=?1 WHERE id=?2",
                rusqlite::params![primary.created_at + 10, additional.id],
            )
            .unwrap();
        let report =
            export_markdown(root.path(), storage.markdown_snapshot(&[id]).unwrap()).unwrap();
        assert_eq!(report.warnings.len(), 2);
        assert!(!contents(&report).contains("C:/private"));
        assert_references(&report, 1);
    }
    #[test]
    fn no_clobber_publication_refuses_existing_destination_and_keeps_original() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let destination = root.path().join("notes.md");
        write_new(&source, b"new data").unwrap();
        write_new(&destination, b"old data").unwrap();
        assert!(publish_file(&source, &destination).is_err());
        assert_eq!(fs::read(destination).unwrap(), b"old data");
        assert_eq!(fs::read(source).unwrap(), b"new data");
    }
    #[test]
    fn invalid_selection_and_unusable_destination_are_rejected_before_publication() {
        assert!(validate_ids(&[]).is_err());
        assert!(validate_ids(&["../x".into()]).is_err());
        assert!(validate_ids(&["a".repeat(32), "a".repeat(32)]).is_err());
        assert!(validate_ids(&(0..101).map(|n| format!("{n:032x}")).collect::<Vec<_>>()).is_err());
        let root = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(root.path().join("db")).unwrap();
        let id = create(&mut storage, "x");
        let file = root.path().join("file");
        write_new(&file, b"x").unwrap();
        assert!(export_markdown(&file, storage.markdown_snapshot(&[id]).unwrap()).is_err());
        assert_eq!(fs::read(file).unwrap(), b"x");
    }
    #[test]
    fn portable_names_and_utc_dates_cover_windows_devices_leap_days_and_limits() {
        for title in [
            "CON",
            "PRN.",
            "AUX",
            "COM1",
            "..",
            "D:\\资料\\*?<>|",
            "😀/研究",
            "# (A) [B] 100%",
        ] {
            let name = image_name(1, title);
            assert!(name.starts_with("screenshot-001-"));
            assert!(name.ends_with(".png"));
            assert!(name.len() < 70);
            assert!(
                name.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'.')
            );
        }
        assert_eq!(created_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(created_utc(951_782_400_000), "2000-02-29T00:00:00Z");
        assert_eq!(created_utc(1_709_251_199_999), "2024-02-29T23:59:59Z");
        assert_eq!(created_utc(253_402_300_799_999), "9999-12-31T23:59:59Z");
        assert!(created_utc(-1).contains("Unix"));
    }
}
