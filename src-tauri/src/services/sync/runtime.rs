use super::{
    credentials,
    engine::{self, Progress},
    model::*,
    transport::{Credentials, Dav},
};
use crate::{
    commands::library::LibraryService,
    storage::{StorageError, sync::Conflict},
};
use serde::Serialize;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteProject {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub phase: String,
    pub message: String,
    pub progress: Progress,
    pub writable: bool,
    pub remote_projects: Vec<RemoteProject>,
}
impl Default for Job {
    fn default() -> Self {
        Self {
            phase: "idle".into(),
            message: "手动同步默认关闭；先配置并测试连接。".into(),
            progress: Progress::default(),
            writable: false,
            remote_projects: vec![],
        }
    }
}
#[derive(Default)]
pub struct SyncService {
    job: Mutex<Job>,
    cancel: Arc<AtomicBool>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub config: Option<Config>,
    pub credential_available: bool,
    pub credential_error: Option<String>,
    pub job: Job,
    pub conflicts: Vec<Conflict>,
    pub pending: usize,
}
pub fn message(error: StorageError) -> String {
    match error {
        StorageError::Invalid(s) => s.into(),
        StorageError::Conflict => "资料或远端已改变；未覆盖，请重新同步并处理冲突。".into(),
        StorageError::NotFound => "同步资料已不存在，请刷新。".into(),
        StorageError::NewerSchema(_) => "资料库来自更高版本，禁止写入。".into(),
        StorageError::Io(_) => {
            "文件读取或安全写入失败；请检查权限、完整性与磁盘空间，已有资料保留。".into()
        }
        StorageError::Database(_) => {
            "资料事务失败，可能存在名称或关联冲突；原记录保留，请查看冲突列表。".into()
        }
    }
}
impl SyncService {
    pub fn busy(&self) -> bool {
        self.job
            .lock()
            .map(|j| ["testing", "syncing", "stopping"].contains(&j.phase.as_str()))
            .unwrap_or(true)
    }
    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Release);
        if let Ok(mut j) = self.job.try_lock()
            && ["testing", "syncing"].contains(&j.phase.as_str())
        {
            j.phase = "stopping".into();
            j.message = "正在停止；当前请求最多30秒，未确认操作将在重试时核对。".into();
        }
    }
    pub fn status(&self, store: &LibraryService) -> Result<Status> {
        let config = store.access(|s| s.sync_config())?;
        let connection = config.as_ref().map(Config::id);
        let credential_error = config.as_ref().and_then(|c| {
            store
                .access(|s| Ok(s.sync_credential_id(c)))
                .and_then(|id| credentials::load(&id).map(|_| ()))
                .err()
                .map(message)
        });
        let credential_available = config.is_some() && credential_error.is_none();
        // Release the job mutex before querying SQLite; stop never waits on this lock.
        let job = self
            .job
            .lock()
            .map_err(|_| invalid("同步状态锁异常。"))?
            .clone();
        Ok(Status {
            config,
            credential_available,
            credential_error,
            job,
            conflicts: connection
                .as_ref()
                .map(|id| store.access(|s| s.sync_conflicts(id)))
                .transpose()?
                .unwrap_or_default(),
            pending: connection
                .as_ref()
                .map(|id| store.access(|s| Ok(s.sync_pending(id)?.len())))
                .transpose()?
                .unwrap_or_default(),
        })
    }
    pub fn configure(
        &self,
        store: &LibraryService,
        config: Config,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<()> {
        let mut job = self.job.lock().map_err(|_| invalid("同步状态锁异常。"))?;
        if ["testing", "syncing", "stopping"].contains(&job.phase.as_str()) {
            return Err(invalid("同步运行中，请先停止再更改连接。"));
        }
        config.validate()?;
        let id = store.access(|s| Ok(s.sync_credential_id(&config)))?;
        match (username, password) {
            (Some(username), Some(password)) if !username.is_empty() && !password.is_empty() => {
                let device = credentials::load(&id)
                    .map(|a| a.device.clone())
                    .unwrap_or(store.access(|s| s.sync_new_id())?);
                credentials::save(
                    &id,
                    Credentials {
                        username,
                        password,
                        device,
                    },
                )?;
            }
            (None, None) => {
                credentials::load(&id)?;
            }
            _ => return Err(invalid("更新凭据时必须同时填写用户名和应用授权密码。")),
        }
        let device = credentials::load(&id)?.device.clone();
        store.access(|s| {
            s.sync_save_config(&config)?;
            s.sync_bind_device(&device)
        })?;
        *job = Job {
            message: "配置已保存在本机；凭据位于Windows凭据管理器，尚未测试服务器能力。".into(),
            ..Job::default()
        };
        Ok(())
    }
    pub fn start(
        self: &Arc<Self>,
        store: Arc<LibraryService>,
        mode: &str,
        choice: Option<(String, String)>,
    ) -> Result<()> {
        let config = store
            .access(|s| s.sync_config())?
            .ok_or(invalid("请先保存WebDAV配置。"))?;
        config.validate()?;
        let auth = credentials::load(&store.access(|s| Ok(s.sync_credential_id(&config)))?)?;
        store.access(|s| s.sync_bind_device(&auth.device))?;
        if !["test", "sync", "resolve"].contains(&mode) {
            return Err(invalid("未知同步操作。"));
        }
        {
            let mut job = self.job.lock().map_err(|_| invalid("同步状态锁异常。"))?;
            if ["testing", "syncing", "stopping"].contains(&job.phase.as_str()) {
                return Err(invalid("已有同步任务运行；不会重复启动。"));
            }
            self.cancel.store(false, Ordering::Release);
            job.phase = if mode == "test" { "testing" } else { "syncing" }.into();
            job.message = "验证TLS、目录与条件写入能力…".into();
            job.progress = Progress::default();
            job.writable = false;
        }
        let service = Arc::clone(self);
        let mode = mode.to_owned();
        std::thread::Builder::new()
            .name("researchclip-webdav".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let device = auth.device.clone();
                    let dav = Dav::new(&config, auth, Arc::clone(&service.cancel))?;
                    let id = store.access(|s| s.sync_new_id())?;
                    dav.probe(&id)?;
                    let workspace = dav.initialize(&id)?;
                    {
                        let mut job = service
                            .job
                            .lock()
                            .map_err(|_| invalid("同步状态锁异常。"))?;
                        job.writable = true;
                        job.message = "条件创建、强ETag、过期/伪造拒绝及并发探针通过。".into();
                    }
                    if mode == "test" {
                        let catalog = dav.catalog(&workspace)?.0;
                        let mut projects = Vec::new();
                        for (key, head) in catalog
                            .entries
                            .iter()
                            .filter(|(key, _)| key.starts_with("project/"))
                        {
                            let m =
                                engine::manifest(store.as_ref(), &dav, &workspace, &head.manifest)?;
                            if m.snapshot.key != *key {
                                return Err(invalid("远端项目身份不一致。"));
                            }
                            if let Body::Project(p) = m.snapshot.body {
                                projects.push(RemoteProject {
                                    id: p.id,
                                    name: p.name,
                                });
                            }
                            if projects.len() > 500 {
                                return Err(invalid("远端项目超过500上限。"));
                            }
                        }
                        service
                            .job
                            .lock()
                            .map_err(|_| invalid("同步状态锁异常。"))?
                            .remote_projects = projects;
                        Ok(None)
                    } else if mode == "resolve" {
                        let (id, choice) = choice.ok_or(invalid("冲突选择缺失。"))?;
                        let conflict = store
                            .access(|s| s.sync_conflicts(&config.id()))?
                            .into_iter()
                            .find(|r| r.id == id)
                            .ok_or(invalid("冲突记录已改变，请刷新。"))?;
                        engine::resolve(
                            store.as_ref(),
                            &dav,
                            &config,
                            &workspace,
                            &device,
                            &conflict,
                            &choice,
                        )?;
                        Ok(None)
                    } else {
                        if config.projects.is_empty() && !config.unclassified {
                            return Err(invalid("请至少选择一个项目或未分类范围。"));
                        }
                        let stats = engine::run(
                            store.as_ref(),
                            &dav,
                            &config,
                            &workspace,
                            &device,
                            |progress| {
                                if let Ok(mut j) = service.job.lock() {
                                    j.progress = progress;
                                    j.message = "正在逐资源核验与同步；已保存内容才会参与。".into();
                                }
                            },
                        )?;
                        Ok(Some(stats))
                    }
                }))
                .unwrap_or_else(|_| Err(invalid("同步任务异常结束；队列及资料保留，请重试。")));
                if let Ok(mut job) = service.job.lock() {
                    match result {
                        Ok(stats) => {
                            if let Some(stats) = stats {
                                job.progress = stats;
                            }
                            job.phase = if job.progress.conflicts > 0 {
                                "conflicts"
                            } else {
                                "succeeded"
                            }
                            .into();
                            job.message = if job.progress.conflicts > 0 {
                                "本轮完成，仍有冲突需要逐项处理。"
                            } else if mode == "test" {
                                "安全连接测试通过；只验证该次连接，不保证所有服务器或网络配置。"
                            } else {
                                "操作已完成；请刷新素材库并重新打开已保存文档。"
                            }
                            .into();
                        }
                        Err(e) => {
                            job.phase = if service.cancel.load(Ordering::Acquire) {
                                "stopped"
                            } else {
                                "failed"
                            }
                            .into();
                            job.message = message(e);
                        }
                    }
                }
            })
            .map_err(|_| {
                if let Ok(mut j) = self.job.lock() {
                    j.phase = "failed".into();
                    j.message = "无法启动后台同步线程。".into();
                }
                invalid("无法启动后台同步线程。")
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod responsiveness_tests {
    use super::*;
    #[test]
    fn cancellation_does_not_wait_for_configuration_status_lock() {
        let service = Arc::new(SyncService::default());
        let other = Arc::clone(&service);
        let guard = service.job.lock().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let task = std::thread::spawn(move || {
            other.stop();
            sender.send(()).unwrap();
        });
        let completed = receiver.recv_timeout(std::time::Duration::from_millis(200));
        drop(guard);
        task.join().unwrap();
        assert!(
            completed.is_ok(),
            "stop must set cancellation without blocking on the job/configuration lock"
        );
        assert!(service.cancel.load(Ordering::Acquire));
    }
}
