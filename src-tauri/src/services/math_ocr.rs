//! Local, single-job MathOCR boundary. The WebView never supplies commands or filesystem paths.
use crate::{
    capture::{CaptureError, ErrorCode},
    storage::files,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
pub const WORKER: &str = include_str!("../../../math-ocr/worker.py");
pub const MODELS: &str = include_str!("../../../math-ocr/models.json");
const MAX_OUTPUT: u64 = 65536;
fn failure(message: &str) -> CaptureError {
    CaptureError::new(ErrorCode::CaptureFailed, message)
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MathJob {
    pub id: u64,
    pub phase: String,
    pub kind: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub cancel_requested: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MathStatus {
    pub configured: bool,
    pub models_present: bool,
    pub model_bytes: u64,
    pub job: MathJob,
    pub error: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Config {
    schema_version: u32,
    python: PathBuf,
}
pub struct MathOcrService {
    root: PathBuf,
    python: Mutex<Option<PathBuf>>,
    job: Mutex<MathJob>,
    initial_error: Option<String>,
}
impl MathOcrService {
    pub fn new(root: PathBuf) -> Result<Self, CaptureError> {
        files::directory(&root).map_err(|_| failure("无法准备公式识别配置目录。"))?;
        let config = root.join("runtime.json");
        files::existing_ordinary(&config).map_err(|_| failure("公式识别配置路径不安全。"))?;
        let python = if config.exists() {
            let mut bytes = Vec::new();
            fs::File::open(&config)
                .and_then(|f| f.take(8193).read_to_end(&mut bytes))
                .map_err(|_| failure("无法读取公式识别配置，已有配置保留。"))?;
            if bytes.len() > 8192 {
                return Err(failure("公式识别配置过大。"));
            }
            let c: Config = serde_json::from_slice(&bytes)
                .map_err(|_| failure("公式识别配置无效，未重置。"))?;
            if c.schema_version != 1 {
                return Err(failure("公式识别配置版本不受支持。"));
            }
            if !c.python.is_absolute()
                || c.python
                    .file_name()
                    .is_none_or(|n| !n.to_string_lossy().eq_ignore_ascii_case("python.exe"))
            {
                return Err(failure("保存的Python路径无效，配置已保留。"));
            }
            Some(c.python)
        } else {
            None
        };
        Ok(Self {
            root,
            python: Mutex::new(python),
            job: Mutex::new(MathJob::default()),
            initial_error: None,
        })
    }
    pub fn unavailable(root: PathBuf, error: String) -> Self {
        Self {
            root,
            python: Mutex::new(None),
            job: Mutex::new(MathJob::default()),
            initial_error: Some(error),
        }
    }
    pub fn status(&self) -> Result<MathStatus, CaptureError> {
        let manifest: serde_json::Value =
            serde_json::from_str(MODELS).map_err(|_| failure("模型清单异常。"))?;
        let models = self.root.join("models-v1");
        let models_present = files::ordinary(&models).is_ok()
            && manifest["files"].as_array().is_some_and(|items| {
                items.iter().all(|v| {
                    let p = models.join(v["name"].as_str().unwrap_or(""));
                    files::ordinary(&p).is_ok()
                        && p.metadata()
                            .is_ok_and(|m| m.is_file() && Some(m.len()) == v["size"].as_u64())
                })
            });
        Ok(MathStatus {
            configured: self
                .python
                .lock()
                .map_err(|_| failure("公式运行时状态异常。"))?
                .is_some(),
            models_present,
            model_bytes: 178_952_787,
            job: self
                .job
                .lock()
                .map_err(|_| failure("公式任务状态异常。"))?
                .clone(),
            error: self.initial_error.clone(),
        })
    }
    pub(crate) fn runtime_python(&self) -> Result<PathBuf, CaptureError> {
        self.python
            .lock()
            .map_err(|_| failure("本地运行时状态异常。"))?
            .clone()
            .ok_or_else(|| failure("请先选择本地Python运行时。"))
    }
    pub fn configure(&self, python: PathBuf) -> Result<(), CaptureError> {
        if let Some(error) = &self.initial_error {
            return Err(failure(error));
        }
        if self.status()?.job.phase == "running" {
            return Err(failure("请先取消或等待公式任务完成。"));
        }
        files::ordinary(&python).map_err(|_| failure("Python运行时不可访问或路径不安全。"))?;
        if !python.is_absolute()
            || !python.is_file()
            || python
                .file_name()
                .is_none_or(|n| !n.to_string_lossy().eq_ignore_ascii_case("python.exe"))
        {
            return Err(failure("请选择已安装依赖的Python3.12及以上python.exe。"));
        }
        // Probe only the fixed code, never execute user/model output as a command.
        let response = execute(
            &python,
            "probe",
            &self.root.join("models-v1"),
            None,
            Duration::from_secs(20),
            || false,
        )?;
        if !response
            .get("python")
            .and_then(|v| v.as_str())
            .is_some_and(|s| {
                s.split('.')
                    .take(2)
                    .collect::<Vec<_>>()
                    .as_slice()
                    .get(1)
                    .and_then(|v| v.parse::<u32>().ok())
                    .is_some_and(|minor| s.starts_with("3.") && minor >= 12)
            })
        {
            return Err(failure("需要Python3.12及以上及指南列出的推理依赖。"));
        }
        let config = self.root.join("runtime.json");
        files::existing_ordinary(&config).map_err(|_| failure("配置路径不安全。"))?;
        let mut stage = tempfile::NamedTempFile::new_in(&self.root)
            .map_err(|_| failure("无法创建运行时配置。"))?;
        stage
            .write_all(
                &serde_json::to_vec(&Config {
                    schema_version: 1,
                    python: python.clone(),
                })
                .map_err(|_| failure("配置编码失败。"))?,
            )
            .map_err(|_| failure("配置写入失败。"))?;
        stage
            .as_file()
            .sync_all()
            .map_err(|_| failure("配置同步失败。"))?;
        stage
            .persist(config)
            .map_err(|_| failure("配置提交失败，原配置保留。"))?;
        *self
            .python
            .lock()
            .map_err(|_| failure("公式运行时状态异常。"))? = Some(python);
        Ok(())
    }
    pub fn start(
        self: &Arc<Self>,
        kind: &str,
        png: Option<Vec<u8>>,
    ) -> Result<MathJob, CaptureError> {
        if !matches!(kind, "recognize" | "download") {
            return Err(failure("公式操作无效。"));
        }
        let python = self
            .python
            .lock()
            .map_err(|_| failure("运行时状态异常。"))?
            .clone()
            .ok_or_else(|| {
                failure("未配置本地公式识别运行时，请先按指南安装依赖并在设置中选择python.exe。")
            })?;
        let mut state = self.job.lock().map_err(|_| failure("公式任务状态异常。"))?;
        if state.phase == "running" {
            return Err(failure("已有公式任务，请先取消或等待完成。"));
        }
        *state = MathJob {
            id: state.id + 1,
            phase: "running".into(),
            kind: kind.into(),
            ..Default::default()
        };
        let started = state.clone();
        drop(state);
        let service = Arc::clone(self);
        let kind = kind.to_string();
        std::thread::spawn(move || {
            let result = (|| {
                let stage = if let Some(png) = png {
                    let mut f = tempfile::NamedTempFile::new_in(&service.root)
                        .map_err(|_| failure("无法创建受控公式图片。"))?;
                    f.write_all(&png)
                        .map_err(|_| failure("无法准备公式图片。"))?;
                    Some(f)
                } else {
                    None
                };
                let downloads = if kind == "download" {
                    Some(
                        tempfile::Builder::new()
                            .prefix("model-download-")
                            .tempdir_in(&service.root)
                            .map_err(|_| failure("无法准备模型下载临时目录。"))?,
                    )
                } else {
                    None
                };
                let result = execute(
                    &python,
                    &kind,
                    &service.root.join("models-v1"),
                    stage
                        .as_ref()
                        .map(|f| f.path())
                        .or_else(|| downloads.as_ref().map(|d| d.path())),
                    if kind == "download" {
                        Duration::from_secs(300)
                    } else {
                        Duration::from_secs(60)
                    },
                    || service.job.lock().is_ok_and(|s| s.cancel_requested),
                );
                if let Some(file) = stage {
                    file.close()
                        .map_err(|_| failure("公式临时图片清理失败，请重启后检查配置目录。"))?;
                }
                if let Some(directory) = downloads {
                    directory
                        .close()
                        .map_err(|_| failure("模型临时下载清理失败，请检查配置目录。"))?;
                }
                result
            })();
            if let Ok(mut s) = service.job.lock() {
                if s.cancel_requested {
                    s.phase = "cancelled".into();
                    s.result = None;
                    s.error = None;
                } else {
                    match result {
                        Ok(v) => {
                            s.phase = "completed".into();
                            s.result = Some(v);
                        }
                        Err(e) => {
                            s.phase = "error".into();
                            s.error = Some(e.message);
                        }
                    }
                }
            }
        });
        Ok(started)
    }
    pub fn cancel(&self) -> Result<(), CaptureError> {
        let mut s = self.job.lock().map_err(|_| failure("公式任务状态异常。"))?;
        if s.phase == "running" {
            s.cancel_requested = true;
        }
        Ok(())
    }
    pub fn import_models(&self, source: &Path) -> Result<(), CaptureError> {
        if self.status()?.job.phase == "running" {
            return Err(failure("请先取消或等待公式任务。"));
        }
        files::ordinary(source).map_err(|_| failure("模型目录不可访问。"))?;
        let dest = self.root.join("models-v1");
        files::directory(&dest).map_err(|_| failure("模型目录无法准备。"))?;
        let manifest: serde_json::Value =
            serde_json::from_str(MODELS).map_err(|_| failure("清单无效。"))?;
        for entry in manifest["files"]
            .as_array()
            .ok_or_else(|| failure("清单无效。"))?
        {
            let name = entry["name"]
                .as_str()
                .ok_or_else(|| failure("清单无效。"))?;
            let src = source.join(name);
            files::ordinary(&src).map_err(|_| failure("模型文件缺失或路径不安全。"))?;
            if src
                .metadata()
                .map_err(|_| failure("模型文件无法读取。"))?
                .len()
                != entry["size"].as_u64().unwrap_or(0)
            {
                return Err(failure("模型文件大小不匹配。"));
            }
            let target = dest.join(name);
            files::existing_ordinary(&target).map_err(|_| failure("模型目标路径不安全。"))?;
            if target.exists() {
                if target
                    .metadata()
                    .is_ok_and(|m| Some(m.len()) == entry["size"].as_u64())
                    && researchclip_clipper_protocol::hash(&read_model(
                        &target,
                        entry["size"].as_u64().unwrap_or(0),
                    )?) == entry["sha256"].as_str().unwrap_or("")
                {
                    continue;
                }
                return Err(failure("模型目标已有不同文件，未覆盖。"));
            }
            let mut stage =
                tempfile::NamedTempFile::new_in(&dest).map_err(|_| failure("模型暂存失败。"))?;
            let expected = entry["size"].as_u64().unwrap_or(0);
            let copied = std::io::copy(
                &mut fs::File::open(src)
                    .map_err(|_| failure("模型读取失败。"))?
                    .take(expected + 1),
                &mut stage,
            )
            .map_err(|_| failure("模型复制失败。"))?;
            if copied != expected {
                return Err(failure("复制期间模型大小发生变化，未安装该文件。"));
            }
            stage
                .as_file()
                .sync_all()
                .map_err(|_| failure("模型同步失败。"))?;
            if researchclip_clipper_protocol::hash(&read_model(stage.path(), expected)?)
                != entry["sha256"].as_str().unwrap_or("")
            {
                return Err(failure("模型摘要不匹配，未安装该文件。"));
            }
            stage
                .persist_noclobber(target)
                .map_err(|_| failure("模型提交失败或已存在，未覆盖。"))?;
        }
        Ok(())
    }
}
fn read_model(path: &Path, expected: u64) -> Result<Vec<u8>, CaptureError> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|f| f.take(expected + 1).read_to_end(&mut bytes))
        .map_err(|_| failure("模型读取失败。"))?;
    if bytes.len() as u64 != expected {
        return Err(failure("模型读取期间大小发生变化。"));
    }
    Ok(bytes)
}

fn execute(
    python: &Path,
    kind: &str,
    models: &Path,
    image: Option<&Path>,
    timeout: Duration,
    cancelled: impl Fn() -> bool,
) -> Result<serde_json::Value, CaptureError> {
    execute_script(python, WORKER, kind, models, image, timeout, cancelled)
}
fn execute_script(
    python: &Path,
    script: &str,
    kind: &str,
    models: &Path,
    image: Option<&Path>,
    timeout: Duration,
    cancelled: impl Fn() -> bool,
) -> Result<serde_json::Value, CaptureError> {
    execute_worker(
        python, script, MODELS, kind, models, image, timeout, cancelled,
    )
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_worker(
    python: &Path,
    script: &str,
    manifest: &str,
    kind: &str,
    models: &Path,
    image: Option<&Path>,
    timeout: Duration,
    cancelled: impl Fn() -> bool,
) -> Result<serde_json::Value, CaptureError> {
    files::ordinary(python).map_err(|_| failure("Python运行时不可用。"))?;
    let mut command = Command::new(python);
    command
        .args(["-I", "-c", script, kind])
        .arg(models)
        .arg(manifest)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env("HF_HUB_OFFLINE", "1")
        .env("TOKENIZERS_PARALLELISM", "false")
        .env_remove("PYTHONPATH")
        .env_remove("PYTHONHOME");
    if let Some(image) = image {
        command.arg(image);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|_| failure("本地Python启动失败，请重新配置运行时。"))?;
    #[cfg(windows)]
    let _job = match child_job(&child) {
        Ok(job) => job,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| failure("公式进程输出不可用。"))?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(MAX_OUTPUT + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let start = Instant::now();
    let code = loop {
        if cancelled() || start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(failure(if cancelled() {
                "公式任务已取消。"
            } else {
                "公式任务超时，进程已结束；请缩小选区或检查模型与网络后重试。"
            }));
        }
        match child.try_wait() {
            Ok(Some(code)) => break code,
            Ok(None) => std::thread::sleep(Duration::from_millis(30)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(failure("公式进程状态读取失败，进程已清理。"));
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| failure("公式输出读取异常。"))?
        .map_err(|_| failure("公式输出无法读取。"))?;
    if !code.success() || bytes.len() as u64 > MAX_OUTPUT {
        return Err(failure("公式进程异常结束或输出超限，请重试。"));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| failure("公式进程返回格式无效。"))?;
    if value["ok"] != true {
        return Err(failure(match value["code"].as_str().unwrap_or("") {
            "MODEL_MISSING" => "本地公式模型缺失，请在设置中主动下载或导入。",
            "MODEL_INVALID" => "模型摘要校验失败，未执行推理。请保留并检查模型文件。",
            "MODEL_CONFLICT" => "模型目录已有不同文件，未覆盖，请检查或选择离线模型。",
            "QUERY_INVALID" => "查询无效，请输入1至500个可见字符。",
            "EMBEDDING_FAILED" => {
                "本地图文推理失败，请核对运行时依赖与CPU兼容性；普通搜索仍可使用。"
            }
            "NO_FORMULA" => "未检测到足够的图像内容，请框选清晰公式。",
            "TOKEN_LIMIT" => "公式过长或模型未正常结束，请缩小选区并人工核对。",
            _ => "本地公式识别失败。请检查Python依赖、模型和图片；可重新识别。",
        }));
    }
    let result = value["result"].clone();
    if kind == "recognize" {
        let text = result["latex"]
            .as_str()
            .ok_or_else(|| failure("公式返回格式无效。"))?;
        if text.is_empty()
            || text.chars().count() > 8192
            || text.contains('\0')
            || result["verified"] != false
        {
            return Err(failure("公式结果无效或过长。"));
        }
    }
    Ok(result)
}

#[cfg(windows)]
struct ChildJob(windows::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Drop for ChildJob {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
#[cfg(windows)]
fn child_job(child: &std::process::Child) -> Result<ChildJob, CaptureError> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{Foundation::HANDLE, System::JobObjects::*};
    let handle =
        unsafe { CreateJobObjectW(None, None) }.map_err(|_| failure("无法创建受控公式进程组。"))?;
    let job = ChildJob(handle);
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
    info.ProcessMemoryLimit = 2 * 1024 * 1024 * 1024;
    unsafe {
        SetInformationJobObject(
            handle,
            JobObjectExtendedLimitInformation,
            &info as *const _ as _,
            std::mem::size_of_val(&info) as u32,
        )
        .and_then(|()| AssignProcessToJobObject(handle, HANDLE(child.as_raw_handle())))
    }
    .map_err(|_| failure("无法限制公式进程资源，识别未开始。"))?;
    Ok(job)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    region: Option<crate::services::ocr::OcrRegion>,
}
pub fn prepare_input(bytes: &[u8]) -> Result<Vec<u8>, CaptureError> {
    if bytes.len() < 5 || bytes.len() > crate::capture::MAX_PNG_BYTES + 1028 {
        return Err(failure("公式图片数据无效或过大。"));
    }
    let size = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    if size == 0 || size > 1024 || size + 4 >= bytes.len() {
        return Err(failure("公式选区数据无效。"));
    }
    let input: Input =
        serde_json::from_slice(&bytes[4..4 + size]).map_err(|_| failure("公式选区无效。"))?;
    let frame = crate::services::workflow::decode_png(&bytes[4 + size..])?;
    let frame = if let Some(r) = input.region {
        crate::capture::geometry::crop_frame(
            frame,
            crate::capture::geometry::RectPx {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
            },
        )?
    } else {
        frame
    };
    if u64::from(frame.width_px) * u64::from(frame.height_px) > 4_194_304
        || frame.width_px.max(frame.height_px) > 4096
    {
        return Err(failure("公式选区最多419万像素、单边4096，请框选较小公式。"));
    }
    crate::services::screen_capture::encode_png(frame)
}

#[cfg(debug_assertions)]
pub fn qa_process_limits(python: &Path, root: &Path) -> Result<(), CaptureError> {
    let start = Instant::now();
    let timeout = execute_script(
        python,
        "import time;time.sleep(20)",
        "probe",
        root,
        None,
        Duration::from_millis(250),
        || false,
    );
    if timeout.is_ok() || start.elapsed() > Duration::from_secs(5) {
        return Err(failure("实际进程超时控制未通过。"));
    }
    let output = execute_script(
        python,
        "print('x'*70000)",
        "probe",
        root,
        None,
        Duration::from_secs(5),
        || false,
    );
    if output.is_ok() {
        return Err(failure("实际进程输出限制未通过。"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn packet(region: serde_json::Value) -> Vec<u8> {
        let png = crate::services::screen_capture::encode_png(crate::capture::RawFrame {
            width_px: 4,
            height_px: 3,
            rgba: (0..48).collect(),
        })
        .unwrap();
        let json = serde_json::to_vec(&serde_json::json!({"region":region})).unwrap();
        let mut b = (json.len() as u32).to_le_bytes().to_vec();
        b.extend(json);
        b.extend(png);
        b
    }
    #[test]
    fn crop_uses_original_pixels_and_rejects_zero_outside_and_unknown_data() {
        let result = prepare_input(&packet(
            serde_json::json!({"x":1,"y":1,"width":2,"height":1}),
        ))
        .unwrap();
        let f = crate::services::workflow::decode_png(&result).unwrap();
        assert_eq!((f.width_px, f.height_px), (2, 1));
        assert_eq!(f.rgba, (20..28).collect::<Vec<u8>>());
        for r in [
            serde_json::json!({"x":0,"y":0,"width":0,"height":1}),
            serde_json::json!({"x":3,"y":2,"width":2,"height":2}),
            serde_json::json!({"x":0,"y":0,"width":1,"height":1,"path":"C:/other"}),
        ] {
            assert!(prepare_input(&packet(r)).is_err());
        }
        for bytes in [vec![], vec![255; 10], vec![0; 5]] {
            assert!(prepare_input(&bytes).is_err());
        }
    }
    #[test]
    fn no_runtime_no_automatic_download_and_future_config_is_preserved() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("math");
        let s = Arc::new(MathOcrService::new(root.clone()).unwrap());
        let status = s.status().unwrap();
        assert!(!status.configured && !status.models_present);
        assert!(s.start("recognize", Some(vec![1])).is_err());
        assert!(!root.join("models-v1").exists());
        let cfg = root.join("runtime.json");
        let text = b"{\"schemaVersion\":99,\"python\":\"python.exe\"}";
        fs::write(&cfg, text).unwrap();
        assert!(MathOcrService::new(root).is_err());
        assert_eq!(fs::read(cfg).unwrap(), text);
    }
    #[test]
    fn malformed_models_do_not_replace_existing_files() {
        let t = tempfile::tempdir().unwrap();
        let s = MathOcrService::new(t.path().join("math")).unwrap();
        let models = t.path().join("external");
        fs::create_dir(&models).unwrap();
        fs::write(models.join("decoder.onnx"), b"not a model").unwrap();
        assert!(s.import_models(&models).is_err());
        assert_eq!(
            fs::read(models.join("decoder.onnx")).unwrap(),
            b"not a model"
        );
        assert!(!s.root.join("models-v1/decoder.onnx").exists());
    }
}
