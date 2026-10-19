//! One worker, at most one decoded image; no automatic scans on launch.
use crate::commands::{CommandError, library::LibraryService};
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub phase: String,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub current: Option<String>,
    pub error: Option<String>,
    pub generation: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_resume_cancel_and_new_generation_do_not_duplicate_work() {
        let service = OcrIndexService::default();
        assert!(service.control("pause").is_err());
        *service.state.lock().unwrap() = IndexProgress {
            phase: "running".into(),
            total: 30,
            generation: 1,
            ..Default::default()
        };
        assert_eq!(service.control("pause").unwrap().phase, "paused");
        assert!(service.control("pause").is_err());
        assert_eq!(service.control("resume").unwrap().phase, "running");
        assert_eq!(service.control("cancel").unwrap().phase, "cancelling");
        assert!(service.control("resume").is_err());
        service.cancel();
        assert_eq!(service.status().unwrap().generation, 1);
    }
}
#[derive(Default)]
pub struct OcrIndexService {
    state: Mutex<IndexProgress>,
}
fn failure(message: &str) -> CommandError {
    CommandError {
        code: "OCR_INDEX",
        message: message.into(),
    }
}
impl OcrIndexService {
    pub fn status(&self) -> Result<IndexProgress, CommandError> {
        self.state
            .lock()
            .map(|s| s.clone())
            .map_err(|_| failure("OCR任务状态异常，请重启应用。"))
    }
    pub fn control(&self, action: &str) -> Result<IndexProgress, CommandError> {
        let mut s = self
            .state
            .lock()
            .map_err(|_| failure("OCR任务状态异常。"))?;
        match (action, s.phase.as_str()) {
            ("pause", "running") => s.phase = "paused".into(),
            ("resume", "paused") => s.phase = "running".into(),
            ("cancel", "running" | "paused") => s.phase = "cancelling".into(),
            _ => return Err(failure("任务状态已改变，请刷新后重试。")),
        }
        Ok(s.clone())
    }
    pub fn cancel(&self) {
        if let Ok(mut s) = self.state.lock()
            && matches!(s.phase.as_str(), "running" | "paused")
        {
            s.phase = "cancelling".into();
        }
    }
    pub fn begin(
        self: &Arc<Self>,
        library: Arc<LibraryService>,
        ids: Vec<String>,
        language: String,
    ) -> Result<IndexProgress, CommandError> {
        if ids.len() > 5000 {
            return Err(failure("每批最多5000张图片。"));
        }
        let languages = super::ocr::OcrService::status().map_err(|e| failure(&e.message))?;
        if !languages.languages.iter().any(|l| l.tag == language) {
            return Err(failure("请选择本机已安装的OCR语言。"));
        }
        let mut s = self
            .state
            .lock()
            .map_err(|_| failure("OCR任务状态异常。"))?;
        if matches!(s.phase.as_str(), "running" | "paused" | "cancelling") {
            return Err(failure("已有OCR索引任务，请先暂停或取消。"));
        }
        *s = IndexProgress {
            phase: "running".into(),
            total: ids.len(),
            generation: s.generation + 1,
            ..Default::default()
        };
        let initial = s.clone();
        drop(s);
        let service = Arc::clone(self);
        std::thread::spawn(move || {
            for id in ids {
                loop {
                    let phase = match service.status() {
                        Ok(s) => s.phase,
                        Err(_) => return,
                    };
                    if phase == "paused" {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        continue;
                    }
                    if phase == "cancelling" {
                        if let Ok(mut s) = service.state.lock() {
                            s.phase = "cancelled".into();
                            s.current = None;
                        }
                        return;
                    }
                    break;
                }
                if let Ok(mut s) = service.state.lock() {
                    s.current = Some(id.clone());
                }
                // Read/decode outside the storage mutex. No originals are modified.
                let result = library.with(|s| {
                    let revision = s.start_ocr(&id, &language)?;
                    match s.screenshot_image(&id, false) {
                        Ok(png) => Ok((png, revision)),
                        Err(e) => {
                            s.finish_ocr(&id, revision, Err("原图缺失、损坏或无法读取。"))?;
                            Err(e)
                        }
                    }
                });
                let result = match result {
                    Ok((png, revision)) => {
                        let options = serde_json::to_vec(
                            &serde_json::json!({"language":language,"region":null}),
                        )
                        .expect("fixed OCR options");
                        let mut packet = (options.len() as u32).to_le_bytes().to_vec();
                        packet.extend(options);
                        packet.extend(png);
                        let recognized =
                            super::ocr::OcrService::recognize_controlled(&packet, || {
                                service.status().is_ok_and(|s| s.phase == "cancelling")
                            });
                        let cancelled = service.status().is_ok_and(|s| s.phase == "cancelling");
                        let saved = library.with(|s| {
                            s.finish_ocr(
                                &id,
                                revision,
                                if cancelled {
                                    Err("识别已取消，已有文本保留。")
                                } else {
                                    recognized.as_ref().map(|r| r.text.as_str()).map_err(
                                        |_| "本地OCR失败，请检查语言、图片尺寸或稍后重试。",
                                    )
                                },
                            )
                        });
                        saved.and_then(|()| {
                            if cancelled || recognized.is_err() {
                                Err(failure(if cancelled {
                                    "识别已取消，已有文本保留。"
                                } else {
                                    "本地OCR失败，请检查语言与图片尺寸。"
                                }))
                            } else {
                                Ok(())
                            }
                        })
                    }
                    Err(e) => Err(e),
                };
                if let Ok(mut s) = service.state.lock() {
                    s.current = None;
                    if let Err(e) = result {
                        s.failed += 1;
                        s.error = Some(e.message);
                    } else {
                        s.completed += 1;
                    }
                }
            }
            if let Ok(mut s) = service.state.lock() {
                s.phase = if s.phase == "cancelling" {
                    "cancelled"
                } else {
                    "completed"
                }
                .into();
                s.current = None;
            }
        });
        Ok(initial)
    }
}
