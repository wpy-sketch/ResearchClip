//! Debug-only native acceptance runner. Uses a newly created test directory, never user data.
use crate::{
    commands::library::LibraryService,
    services::{ocr::OcrService, ocr_index::OcrIndexService},
    storage::{
        Storage,
        library::LibrarySort,
        search::{LibraryFilter, SearchSource},
    },
};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
pub fn run(fixtures: &Path, root: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if root.exists() {
        return Err("Acceptance directory must be new".into());
    }
    std::fs::create_dir_all(root)?;
    let manifest: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(fixtures.join("manifest.json"))?)?;
    if manifest.len() != 30 {
        return Err("Expected thirty owned PNG fixtures".into());
    }
    let status = OcrService::status()?;
    let language = status
        .languages
        .iter()
        .find(|l| l.tag.starts_with("zh"))
        .or(status.languages.first())
        .ok_or("No OCR language")?
        .tag
        .clone();
    let mut s = Storage::open(root.join("workspace"))?;
    let mut ids = Vec::new();
    for (i, entry) in manifest.iter().enumerate() {
        let filename = entry["file"].as_str().ok_or("filename")?;
        if filename != format!("text-{:02}.png", i + 1) {
            return Err("Unsafe fixture reference".into());
        }
        let png = std::fs::read(fixtures.join(filename))?;
        let frame = crate::services::workflow::decode_png(&png)?;
        let shot=s.create_screenshot(&format!("验收图{:02}",i+1),serde_json::json!({"schemaVersion":1,"image":{"widthPx":frame.width_px,"heightPx":frame.height_px},"objects":[]}),&png)?;
        ids.push(shot.id);
    }
    drop(s);
    let library = Arc::new(LibraryService::new(root.join("workspace")));
    let jobs = Arc::new(OcrIndexService::default());
    let start = Instant::now();
    jobs.begin(Arc::clone(&library), ids.clone(), language.clone())
        .map_err(|e| e.message)?;
    // A genuine worker is paused/cancelled, then a new explicit batch resumes missing/failed.
    jobs.control("pause").map_err(|e| e.message)?;
    std::thread::sleep(Duration::from_millis(250));
    let paused = jobs.status().map_err(|e| e.message)?;
    if paused.phase != "paused" {
        return Err("Pause not retained".into());
    }
    jobs.control("resume").map_err(|e| e.message)?;
    jobs.control("cancel").map_err(|e| e.message)?;
    while jobs.status().map_err(|e| e.message)?.phase != "cancelled" {
        if start.elapsed() > Duration::from_secs(45) {
            return Err("Cancel timed out".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let pending = library
        .with(|s| s.pending_ocr(false))
        .map_err(|e| e.message)?;
    jobs.begin(Arc::clone(&library), pending, language.clone())
        .map_err(|e| e.message)?;
    loop {
        let p = jobs.status().map_err(|e| e.message)?;
        if p.phase == "completed" {
            if p.failed > 0 {
                return Err(format!("{} OCR failures", p.failed).into());
            }
            break;
        }
        if start.elapsed() > Duration::from_secs(240) {
            jobs.cancel();
            return Err("Batch deadline".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let mut cases = Vec::new();
    for (entry, id) in manifest.iter().zip(&ids) {
        let r = library
            .with(|s| s.ocr_content(id))
            .map_err(|e| e.message)?
            .ok_or("Missing OCR record")?;
        if r.recognition_status != "completed" || r.recognized_text.is_empty() {
            return Err("OCR empty or unfinished".into());
        }
        let keyword = entry["keyword"].as_str().ok_or("keyword")?;
        let matched = r
            .recognized_text
            .to_ascii_lowercase()
            .contains(&keyword.to_ascii_lowercase());
        let results = library
            .with(|s| {
                s.search_screenshot_page(
                    0,
                    24,
                    false,
                    LibrarySort::CreatedDesc,
                    &LibraryFilter {
                        query: keyword.into(),
                        source: SearchSource::Ocr,
                        ..Default::default()
                    },
                )
            })
            .map_err(|e| e.message)?;
        cases.push(serde_json::json!({"file":entry["file"],"kind":entry["kind"],"nonempty":true,"keywordMatched":matched,"searchCount":results.total,"characters":r.recognized_text.chars().count()}));
    }
    library
        .with(|s| s.rebuild_search())
        .map_err(|e| e.message)?;
    let count = library
        .with(|s| {
            Ok(ids
                .iter()
                .filter(|id| {
                    s.ocr_content(id)
                        .ok()
                        .flatten()
                        .is_some_and(|r| r.recognition_status == "completed")
                })
                .count())
        })
        .map_err(|e| e.message)?;
    drop(library);
    drop(jobs);
    let s = Storage::open(root.join("workspace"))?;
    if ids
        .iter()
        .any(|id| s.ocr_content(id).ok().flatten().is_none())
    {
        return Err("Restart lost OCR records".into());
    }
    Ok(
        serde_json::json!({"mode":"actual Windows.Media.Ocr, thirty owned text images, real SQLite/FTS and worker; no mock or desktop input","result":"PASS","language":language,"installedLanguages":status.languages,"completed":count,"seconds":start.elapsed().as_secs_f64(),"pauseResumeCancel":"PASS","rebuild":"PASS","restart":"PASS","cases":cases}),
    )
}
