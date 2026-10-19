//! Debug-only, real Python/ONNX acceptance. All data is owned fixtures in a new directory.
use crate::services::math_ocr::{MathOcrService, prepare_input};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
fn wait(
    s: &MathOcrService,
) -> Result<crate::services::math_ocr::MathJob, Box<dyn std::error::Error>> {
    let start = Instant::now();
    loop {
        let job = s.status()?.job;
        if job.phase != "running" {
            return Ok(job);
        }
        if start.elapsed() > Duration::from_secs(if job.kind == "download" { 310 } else { 70 }) {
            s.cancel()?;
            return Err("Math acceptance deadline".into());
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}
pub fn run(
    python: &Path,
    models: &Path,
    fixtures: &Path,
    root: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if root.exists() {
        return Err("Use a new test root".into());
    }
    std::fs::create_dir_all(root)?;
    crate::services::math_ocr::qa_process_limits(python, root)?;
    let service = Arc::new(MathOcrService::new(root.join("math"))?);
    service.configure(python.to_path_buf())?;
    service.start(
        "recognize",
        Some(std::fs::read(fixtures.join("quadratic.png"))?),
    )?;
    let missing = wait(&service)?;
    if missing.phase != "error"
        || !missing
            .error
            .as_deref()
            .is_some_and(|s| s.contains("模型缺失"))
    {
        return Err("Missing model not reported safely".into());
    }
    if std::env::var_os("RESEARCHCLIP_QA_DOWNLOAD_MODELS").is_some() {
        service.start("download", None)?;
        std::thread::sleep(Duration::from_millis(350));
        service.cancel()?;
        if wait(&service)?.phase != "cancelled" {
            return Err("Download cancel failed".into());
        }
        if std::fs::read_dir(root.join("math"))?.any(|e| {
            e.ok().is_some_and(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("model-download-")
            })
        }) {
            return Err("Cancelled download retained temporary directory".into());
        }
        service.start("download", None)?;
        let downloaded = wait(&service)?;
        if downloaded.phase != "completed" {
            return Err(format!("Download failed: {:?}", downloaded.error).into());
        }
    }
    service.import_models(models)?;
    service.import_models(models)?;
    let tokenizer = root.join("math/models-v1/tokenizer.json");
    let mut bad = std::fs::read(&tokenizer)?;
    bad[0] ^= 1;
    std::fs::write(&tokenizer, &bad)?;
    service.start(
        "recognize",
        Some(std::fs::read(fixtures.join("quadratic.png"))?),
    )?;
    let invalid = wait(&service)?;
    if invalid.phase != "error"
        || !invalid
            .error
            .as_deref()
            .is_some_and(|s| s.contains("摘要校验"))
    {
        return Err("Tampered model accepted".into());
    }
    // Restore only this newly created acceptance copy; never modify downloaded source or user data.
    std::fs::copy(models.join("tokenizer.json"), tokenizer)?;
    let fixtures_list: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(fixtures.join("manifest.json"))?)?;
    let mut cases = Vec::new();
    let start = Instant::now();
    let handles_before = handles();
    for item in &fixtures_list {
        let file = item["file"].as_str().ok_or("fixture")?;
        if file.contains(['/', '\\']) || !file.ends_with(".png") {
            return Err("Unsafe fixture".into());
        }
        let png = std::fs::read(fixtures.join(file))?;
        let header = serde_json::to_vec(&serde_json::json!({"region":null}))?;
        let mut bytes = (header.len() as u32).to_le_bytes().to_vec();
        bytes.extend(header);
        bytes.extend(png);
        let png = prepare_input(&bytes)?;
        let before = Instant::now();
        service.start("recognize", Some(png))?;
        let job = wait(&service)?;
        if job.phase != "completed" {
            return Err(format!("{} failed: {:?}", file, job.error).into());
        }
        let output = job.result.ok_or("No model output")?;
        let text = output["latex"].as_str().ok_or("No LaTex")?;
        if output["verified"] != false {
            return Err("Missing scientific warning".into());
        }
        cases.push(serde_json::json!({"kind":item["kind"],"expectedLatex":item["latex"],"latex":text,"seconds":before.elapsed().as_secs_f64(),"verified":false}));
    }
    service.start(
        "recognize",
        Some(std::fs::read(fixtures.join("quadratic.png"))?),
    )?;
    service.cancel()?;
    if wait(&service)?.phase != "cancelled" {
        return Err("Cancel did not clean up".into());
    }
    service.start("recognize", Some(vec![1, 2, 3]))?;
    if wait(&service)?.phase != "error" {
        return Err("Corrupt image not rejected".into());
    }
    service.start(
        "recognize",
        Some(std::fs::read(fixtures.join("quadratic.png"))?),
    )?;
    let recovered = wait(&service)?;
    if recovered.phase != "completed" {
        return Err("Failed job cannot recover".into());
    }
    let blank = crate::services::screen_capture::encode_png(crate::capture::RawFrame {
        width_px: 64,
        height_px: 32,
        rgba: vec![255; 64 * 32 * 4],
    })?;
    service.start("recognize", Some(blank))?;
    let no_formula = wait(&service)?;
    if no_formula.phase != "error"
        || !no_formula
            .error
            .as_deref()
            .is_some_and(|s| s.contains("未检测"))
    {
        return Err("Blank image not handled".into());
    }
    std::thread::sleep(Duration::from_millis(100));
    let handles_after = handles();
    if handles_after > handles_before + 4 {
        return Err("Parent process handles grew after repeated inference".into());
    }
    drop(service);
    let reopened = MathOcrService::new(root.join("math"))?;
    if !reopened.status()?.configured || !reopened.status()?.models_present {
        return Err("Runtime/model settings not persistent".into());
    }
    // Confirmed source is stored as plain note text and survives reopening; no model output is executed.
    let mut storage = crate::storage::Storage::open(root.join("workspace"))?;
    let png = std::fs::read(fixtures.join("quadratic.png"))?;
    let frame = crate::services::workflow::decode_png(&png)?;
    let shot=storage.create_screenshot("公式验收",serde_json::json!({"schemaVersion":1,"image":{"widthPx":frame.width_px,"heightPx":frame.height_px},"objects":[]}),&png)?;
    let text = cases[0]["latex"].as_str().ok_or("source")?;
    let note = format!("原笔记\n\n$$\n{text}\n$$\n");
    let md = root.join("公式.md");
    let input = crate::services::markdown_document::SaveInput {
        token: None,
        markdown: note.clone(),
        assets: vec![],
        save_as: false,
        file_name: None,
    };
    let mut documents = crate::services::markdown_document::DocumentFiles::default();
    let saved = documents.save(Some(&md), &input, &[])?;
    documents.release(&saved.token);
    let _ = documents.open(&md)?;
    if std::fs::read_to_string(md)? != note {
        return Err("Markdown LaTex source changed".into());
    }
    storage.save_note(&shot.id, None, None, &note, None)?;
    drop(storage);
    let storage = crate::storage::Storage::open(root.join("workspace"))?;
    if storage.list_notes(&shot.id)?[0].content != note {
        return Err("Saved LaTex changed".into());
    }
    Ok(
        serde_json::json!({"result":"PASS","scope":"actual Windows Rust service to isolated Python ONNX processes, fixed source and pinned models; not UI or mathematical accuracy acceptance","cases":cases,"seconds":start.elapsed().as_secs_f64(),"cancel":"PASS","corruptRecovery":"PASS","settingsReload":"PASS","noteReload":"PASS","markdownReload":"PASS","timeoutAndOutputLimit":"PASS","handlesBefore":handles_before,"handlesAfter":handles_after,"downloadAndCancel":if std::env::var_os("RESEARCHCLIP_QA_DOWNLOAD_MODELS").is_some(){"PASS"}else{"NOT_RUN"},"duplicateImport":"PASS","missingModel":"PASS","invalidModel":"PASS","blankImage":"PASS","mathCorrectness":"NOT_VERIFIED"}),
    )
}
fn handles() -> u32 {
    #[cfg(windows)]
    {
        let mut count = 0;
        unsafe {
            let _ = windows::Win32::System::Threading::GetProcessHandleCount(
                windows::Win32::System::Threading::GetCurrentProcess(),
                &mut count,
            );
        }
        count
    }
    #[cfg(not(windows))]
    {
        0
    }
}
