use crate::{
    commands::{CommandError, library::LibraryService},
    storage::StorageError,
};
use serde::Serialize;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: String,
    pub total: u64,
    pub scanned: u64,
    pub computed: u64,
    pub skipped: u64,
    pub failed: u64,
    pub error: Option<String>,
    pub generation: u64,
}
#[derive(Default)]
pub struct SimilarityIndex {
    state: Mutex<Progress>,
    cancel: AtomicBool,
}
fn error() -> CommandError {
    CommandError {
        code: "SIMILARITY_INDEX",
        message: "相似索引任务不可用，请稍后重试；素材保留。".into(),
    }
}
impl SimilarityIndex {
    pub fn status(&self) -> Result<Progress, CommandError> {
        self.state.lock().map(|s| s.clone()).map_err(|_| error())
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
        if let Ok(mut s) = self.state.lock()
            && s.phase == "running"
        {
            s.phase = "cancelling".into();
        }
    }
    pub fn busy(&self) -> bool {
        self.status()
            .is_ok_and(|s| matches!(s.phase.as_str(), "running" | "cancelling"))
    }
    pub fn begin(
        self: &Arc<Self>,
        library: Arc<LibraryService>,
        id: Option<String>,
        rebuild: bool,
    ) -> Result<Progress, CommandError> {
        let (total, upper) = library.with(|s| {
            if let Some(id) = &id {
                s.get_screenshot(id)?;
                Ok((1, id.clone()))
            } else {
                Ok((s.feature_stats()?.total, s.feature_upper()?))
            }
        })?;
        if total > 100_000 {
            return Err(CommandError {
                code: "SIMILARITY_LIMIT",
                message: "首版每批最多10万张，请整理后再建索引；未修改素材。".into(),
            });
        }
        let initial = {
            let mut s = self.state.lock().map_err(|_| error())?;
            if matches!(s.phase.as_str(), "running" | "cancelling") {
                return Err(error());
            }
            *s = Progress {
                phase: "running".into(),
                total,
                generation: s.generation + 1,
                ..Default::default()
            };
            self.cancel.store(false, Ordering::Release);
            s.clone()
        };
        let service = Arc::clone(self);
        if std::thread::Builder::new()
            .name("researchclip-similarity".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || -> Result<(), StorageError> {
                        let mut after = String::new();
                        loop {
                            if service.cancel.load(Ordering::Acquire) {
                                break;
                            }
                            let next = if let Some(id) = &id {
                                if after.is_empty() {
                                    Some(id.clone())
                                } else {
                                    None
                                }
                            } else {
                                library.access(|s| s.next_feature_id(&after, &upper))?
                            };
                            let Some(current) = next else {
                                break;
                            };
                            after = current.clone();
                            let outcome = library.access(|s| s.feature_input(&current, rebuild));
                            let computed = match outcome {
                                Ok(None) => Ok(false),
                                Ok(Some(input)) => {
                                    let features = super::similarity::compute(&input.bytes);
                                    if service.cancel.load(Ordering::Acquire) {
                                        break;
                                    }
                                    features
                                        .and_then(|features| {
                                            library.access(|s| {
                                                s.save_features(&current, &input.stamp, &features)
                                            })
                                        })
                                        .map(|()| true)
                                }
                                Err(e) => Err(e),
                            };
                            if computed.is_err() {
                                library.access(|s| s.fail_features(&current))?;
                            }
                            let mut s = service
                                .state
                                .lock()
                                .map_err(|_| StorageError::Invalid("索引状态锁异常。"))?;
                            s.scanned += 1;
                            match computed {
                                Ok(true) => s.computed += 1,
                                Ok(false) => s.skipped += 1,
                                Err(_) => {
                                    s.failed += 1;
                                    s.error = Some(
                                        "部分原图无法索引，资料已保留；检查图片后重建索引。".into(),
                                    );
                                }
                            }
                        }
                        Ok(())
                    },
                ));
                if let Ok(mut s) = service.state.lock() {
                    s.phase = if service.cancel.load(Ordering::Acquire) {
                        "cancelled"
                    } else if matches!(result, Ok(Ok(()))) {
                        "completed"
                    } else {
                        "failed"
                    }
                    .into();
                    if !matches!(result, Ok(Ok(()))) {
                        s.error = Some("索引中止，已完成记录保留，请重试；未删除素材。".into());
                    }
                }
            })
            .is_err()
        {
            if let Ok(mut s) = self.state.lock() {
                s.phase = "failed".into();
            }
            return Err(error());
        }
        Ok(initial)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelling_is_idempotent_and_no_idle_work_is_started() {
        let s = SimilarityIndex::default();
        s.cancel();
        assert!(!s.busy());
        *s.state.lock().unwrap() = Progress {
            phase: "running".into(),
            generation: 2,
            ..Default::default()
        };
        s.cancel();
        s.cancel();
        assert_eq!(s.status().unwrap().phase, "cancelling");
        assert_eq!(s.status().unwrap().generation, 2);
        assert!(s.cancel.load(Ordering::Acquire));
    }
}
