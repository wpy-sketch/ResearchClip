//! Explicit, finite offline embedding jobs. Reuses the existing bounded Python process boundary.
use crate::{
    commands::{CommandError, library::LibraryService},
    storage::{
        StorageError,
        embeddings::{SemanticResults, SemanticStats},
        files,
        similarity::SearchScope,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
pub const MODEL_ID: &str = "clip-b32-multilingual-int8-v1";
pub const MODEL_VERSION: i64 = 1;
pub const MANIFEST: &str = include_str!("../../../semantic-search/models.json");
const WORKER: &str = include_str!("../../../semantic-search/worker.py");
fn error(message: impl Into<String>) -> CommandError {
    CommandError {
        code: "SEMANTIC",
        message: message.into(),
    }
}
pub fn validate_vector(v: &[f32]) -> Result<(), StorageError> {
    let norm = v.iter().map(|n| n * n).sum::<f32>();
    if v.len() != 512
        || !v.iter().all(|n| n.is_finite())
        || !norm.is_finite()
        || (norm - 1.).abs() > 0.005
    {
        return Err(StorageError::Invalid(
            "语义向量无效，请检查模型并重建索引。",
        ));
    }
    Ok(())
}
pub fn validate_query(query: &str) -> Result<(), CommandError> {
    if query.trim().is_empty() || query.chars().count() > 500 || query.chars().any(char::is_control)
    {
        return Err(error("请输入1至500个可见字符。"));
    }
    Ok(())
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticJob {
    pub phase: String,
    pub kind: String,
    pub generation: u64,
    pub total: u64,
    pub scanned: u64,
    pub computed: u64,
    pub skipped: u64,
    pub error: Option<String>,
    pub results: Option<SemanticResults>,
    pub query: Option<String>,
    pub scope: Option<SearchScope>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticStatus {
    pub model_id: &'static str,
    pub model_bytes: u64,
    pub models_present: bool,
    pub configured: bool,
    pub stats: SemanticStats,
    pub job: SemanticJob,
}
#[derive(Deserialize)]
struct Manifest {
    files: Vec<ModelFile>,
}
#[derive(Deserialize)]
struct ModelFile {
    name: String,
    size: u64,
    sha256: String,
}
pub struct EmbeddingService {
    root: PathBuf,
    state: Mutex<SemanticJob>,
    cancel: AtomicBool,
}
impl EmbeddingService {
    pub fn busy(&self) -> bool {
        self.state
            .lock()
            .is_ok_and(|s| matches!(s.phase.as_str(), "running" | "cancelling"))
    }
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            state: Mutex::new(SemanticJob::default()),
            cancel: AtomicBool::new(false),
        }
    }
    fn models(&self) -> PathBuf {
        self.root.join("models")
    }
    fn manifest() -> Result<Manifest, CommandError> {
        serde_json::from_str(MANIFEST).map_err(|_| error("语义模型清单损坏。"))
    }
    pub fn status(
        &self,
        library: &LibraryService,
        configured: bool,
    ) -> Result<SemanticStatus, CommandError> {
        let manifest = Self::manifest()?;
        let models = self.models();
        Ok(SemanticStatus {
            model_id: MODEL_ID,
            model_bytes: manifest.files.iter().map(|f| f.size).sum(),
            models_present: files::ordinary(&models).is_ok()
                && manifest.files.iter().all(|f| {
                    let p = models.join(&f.name);
                    files::ordinary(&p).is_ok()
                        && p.metadata().is_ok_and(|m| m.is_file() && m.len() == f.size)
                }),
            configured,
            stats: library.with(|s| s.semantic_stats())?,
            job: self
                .state
                .lock()
                .map_err(|_| error("语义任务锁异常。"))?
                .clone(),
        })
    }
    pub fn cancel(&self) -> Result<(), CommandError> {
        let mut s = self.state.lock().map_err(|_| error("语义任务锁异常。"))?;
        if matches!(s.phase.as_str(), "running" | "cancelling") {
            self.cancel.store(true, Ordering::Release);
            s.phase = "cancelling".into();
        }
        Ok(())
    }
    pub fn clear(&self, library: &LibraryService) -> Result<(), CommandError> {
        let mut s = self.state.lock().map_err(|_| error("语义任务锁异常。"))?;
        if matches!(s.phase.as_str(), "running" | "cancelling") {
            return Err(error("请先取消并等待当前语义任务结束。"));
        }
        library.with(|db| db.clear_embeddings())?;
        *s = SemanticJob {
            generation: s.generation + 1,
            ..Default::default()
        };
        Ok(())
    }
    fn execute(
        &self,
        python: &Path,
        kind: &str,
        input: Option<&Path>,
        models: &Path,
    ) -> Result<serde_json::Value, CommandError> {
        super::math_ocr::execute_worker(
            python,
            WORKER,
            MANIFEST,
            kind,
            models,
            input,
            Duration::from_secs(if kind == "download" { 1800 } else { 90 }),
            || self.cancel.load(Ordering::Acquire),
        )
        .map_err(|e| error(e.message.replace("公式", "语义")))
    }
    #[allow(clippy::too_many_arguments)] // Fixed local job request; never a generic script interface.
    pub fn begin(
        self: &Arc<Self>,
        library: Arc<LibraryService>,
        python: PathBuf,
        kind: String,
        query: Option<String>,
        rebuild: bool,
        scope: SearchScope,
        source: Option<PathBuf>,
    ) -> Result<SemanticJob, CommandError> {
        if !matches!(kind.as_str(), "index" | "query" | "download" | "import") {
            return Err(error("未知语义操作。"));
        }
        if kind == "query" {
            validate_query(query.as_deref().unwrap_or(""))?;
        }
        let (total, upper) = if kind == "index" {
            library.with(|s| Ok((s.semantic_stats()?.total, s.feature_upper()?)))?
        } else {
            (0, String::new())
        };
        if total > 2000 {
            return Err(error(
                "实验版最多索引2000张，请减少素材后重试；普通搜索不受影响。",
            ));
        }
        let initial = {
            let mut state = self.state.lock().map_err(|_| error("语义任务锁异常。"))?;
            if matches!(state.phase.as_str(), "running" | "cancelling") {
                return Err(error("已有语义任务正在执行。"));
            }
            files::directory(&self.root).map_err(|_| error("语义工作目录不可用。"))?;
            *state = SemanticJob {
                phase: "running".into(),
                kind: kind.clone(),
                generation: state.generation + 1,
                total,
                query: query.clone(),
                scope: if kind == "query" {
                    Some(scope.clone())
                } else {
                    None
                },
                ..Default::default()
            };
            self.cancel.store(false, Ordering::Release);
            state.clone()
        };
        let service = Arc::clone(self);
        let started = std::thread::Builder::new()
            .name("researchclip-semantic".into())
            .spawn(move || {
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || -> Result<(), CommandError> {
                        match kind.as_str() {
                            "download" => {
                                let stage = tempfile::tempdir_in(&service.root)
                                    .map_err(|_| error("无法准备下载暂存目录。"))?;
                                service.execute(&python, "download", None, stage.path())?;
                                service.import_models(stage.path())?;
                            }
                            "import" => service.import_models(
                                source
                                    .as_deref()
                                    .ok_or_else(|| error("未选择离线模型目录。"))?,
                            )?,
                            "query" => {
                                let mut input = tempfile::NamedTempFile::new_in(&service.root)
                                    .map_err(|_| error("无法暂存查询。"))?;
                                input
                                    .write_all(
                                        serde_json::json!({"query":query}).to_string().as_bytes(),
                                    )
                                    .map_err(|_| error("查询暂存写入失败。"))?;
                                let result = service.execute(
                                    &python,
                                    "text",
                                    Some(input.path()),
                                    &service.models(),
                                )?;
                                let vector = decode(&result)?;
                                if !service.cancel.load(Ordering::Acquire) {
                                    let results =
                                        library.with(|s| s.semantic_search(&vector, &scope))?;
                                    service
                                        .state
                                        .lock()
                                        .map_err(|_| error("语义任务锁异常。"))?
                                        .results = Some(results);
                                }
                            }
                            "index" => {
                                let mut after = String::new();
                                loop {
                                    if service.cancel.load(Ordering::Acquire) {
                                        break;
                                    }
                                    let Some(id) =
                                        library.with(|s| s.next_feature_id(&after, &upper))?
                                    else {
                                        break;
                                    };
                                    after = id.clone();
                                    let input = library.with(|s| s.semantic_input(&id, rebuild))?;
                                    let computed = if let Some((bytes, hash, stamp)) = input {
                                        let mut input =
                                            tempfile::NamedTempFile::new_in(&service.root)
                                                .map_err(|_| error("无法暂存原图。"))?;
                                        input
                                            .write_all(&bytes)
                                            .map_err(|_| error("原图暂存写入失败。"))?;
                                        let result = service.execute(
                                            &python,
                                            "image",
                                            Some(input.path()),
                                            &service.models(),
                                        )?;
                                        if service.cancel.load(Ordering::Acquire) {
                                            break;
                                        }
                                        let vector = decode(&result)?;
                                        library.with(|s| {
                                            s.save_embedding(&id, &hash, &stamp, &vector)
                                        })?;
                                        true
                                    } else {
                                        false
                                    };
                                    let mut state = service
                                        .state
                                        .lock()
                                        .map_err(|_| error("语义任务锁异常。"))?;
                                    state.scanned += 1;
                                    if computed {
                                        state.computed += 1;
                                    } else {
                                        state.skipped += 1;
                                    }
                                }
                            }
                            _ => unreachable!(),
                        }
                        Ok(())
                    },
                ));
                if let Ok(mut state) = service.state.lock() {
                    state.phase = if service.cancel.load(Ordering::Acquire) {
                        "cancelled"
                    } else if matches!(outcome, Ok(Ok(()))) {
                        "completed"
                    } else {
                        "failed"
                    }
                    .into();
                    state.error = match outcome {
                        Ok(Err(e)) => Some(e.message),
                        Err(_) => Some("语义任务异常结束，已完成索引保留，请重试。".into()),
                        _ => None,
                    };
                    if state.phase == "cancelled" {
                        state.results = None;
                        if state.error.as_deref() == Some("语义任务已取消。") {
                            state.error = None;
                        }
                    }
                }
            });
        if started.is_err() {
            if let Ok(mut state) = self.state.lock() {
                state.phase = "failed".into();
                state.error = Some("无法启动语义线程。".into());
            }
            return Err(error("无法启动语义线程。"));
        }
        Ok(initial)
    }
    fn import_models(&self, source: &Path) -> Result<(), CommandError> {
        files::ordinary(source).map_err(|_| error("模型目录不是普通本地目录。"))?;
        files::directory(&self.models()).map_err(|_| error("无法准备模型目录。"))?;
        for item in Self::manifest()?.files {
            if self.cancel.load(Ordering::Acquire) {
                return Err(error("语义任务已取消。"));
            }
            let input = source.join(&item.name);
            verify_file(&input, &item)?;
            let dest = self.models().join(&item.name);
            files::existing_ordinary(&dest).map_err(|_| error("模型目标不安全。"))?;
            if dest.exists() {
                verify_file(&dest, &item)?;
                continue;
            }
            let mut stage = tempfile::NamedTempFile::new_in(self.models())
                .map_err(|_| error("无法暂存模型。"))?;
            std::io::copy(
                &mut fs::File::open(input).map_err(|_| error("模型无法读取。"))?,
                &mut stage,
            )
            .map_err(|_| error("模型复制失败。"))?;
            stage
                .as_file()
                .sync_all()
                .map_err(|_| error("模型同步失败。"))?;
            verify_file(stage.path(), &item)?;
            stage
                .persist_noclobber(dest)
                .map_err(|_| error("模型目标已存在，未覆盖；请检查目录。"))?;
        }
        Ok(())
    }
}
fn verify_file(path: &Path, item: &ModelFile) -> Result<(), CommandError> {
    files::ordinary(path).map_err(|_| error("模型缺失或路径不安全。"))?;
    let mut file = fs::File::open(path).map_err(|_| error("模型无法读取。"))?;
    if file.metadata().map_err(|_| error("模型无法读取。"))?.len() != item.size {
        return Err(error("模型大小不符，已有文件未覆盖。"));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| error("模型摘要读取失败。"))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if format!("{:x}", hash.finalize()) != item.sha256 {
        return Err(error("模型摘要不符，已有文件未覆盖。"));
    }
    Ok(())
}
fn decode(result: &serde_json::Value) -> Result<Vec<f32>, CommandError> {
    if result["model"] != MODEL_ID || result["version"] != MODEL_VERSION {
        return Err(error("编码器与索引模型版本不匹配。"));
    }
    let vector: Vec<f32> = serde_json::from_value(result["vector"].clone())
        .map_err(|_| error("编码器返回无效向量。"))?;
    validate_vector(&vector).map_err(|_| error("编码器返回无效向量。"))?;
    Ok(vector)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_queries_invalid_models_and_vectors() {
        for q in ["".into(), " ".into(), "\nsecret".into(), "a".repeat(501)] {
            assert!(validate_query(&q).is_err());
        }
        assert!(validate_query("神经网络 diagram").is_ok());
        assert!(
            decode(&serde_json::json!({"model":"other","version":1,"vector":vec![0.;512]}))
                .is_err()
        );
    }
    #[test]
    fn idle_cancel_never_starts_work_and_import_never_overwrites() {
        let temp = tempfile::tempdir().unwrap();
        let service = EmbeddingService::new(temp.path().join("semantic"));
        service.cancel().unwrap();
        assert_eq!(service.state.lock().unwrap().phase, "");
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("text.onnx"), b"invalid").unwrap();
        assert!(service.import_models(source.path()).is_err());
        assert_eq!(
            fs::read(source.path().join("text.onnx")).unwrap(),
            b"invalid"
        );
    }
}
