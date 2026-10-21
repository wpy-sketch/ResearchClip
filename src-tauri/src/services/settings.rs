use crate::capture::{CaptureError, ErrorCode};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub schema_version: u32,
    pub theme: String,
    pub view_mode: String,
    pub font_family: String,
    pub font_size: u8,
    pub capture_shortcut: String,
    #[serde(default = "default_fullscreen_shortcut")]
    pub fullscreen_shortcut: String,
    pub save_directory: Option<String>,
    #[serde(default = "default_annotation_color")]
    pub annotation_color: String,
    #[serde(default = "default_annotation_width")]
    pub annotation_width: u8,
    #[serde(default)]
    pub close_to_tray: bool,
    #[serde(default = "default_markdown_split_percent")]
    pub markdown_split_percent: u8,
    #[serde(default)]
    pub workflows: Vec<WorkflowConfig>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowConfig {
    pub id: String,
    pub title: String,
    pub actions: Vec<String>,
}
fn validate_workflows(workflows: &[WorkflowConfig]) -> Result<(), CaptureError> {
    let mut ids = std::collections::BTreeSet::new();
    if workflows.len() > 16 {
        return Err(invalid());
    }
    for workflow in workflows {
        if !workflow
            .id
            .strip_prefix("custom-")
            .is_some_and(crate::storage::files::id_valid)
            || !ids.insert(&workflow.id)
            || workflow.title.trim().is_empty()
            || workflow.title.encode_utf16().count() > 80
            || workflow.title.chars().any(char::is_control)
            || !(2..=8).contains(&workflow.actions.len())
        {
            return Err(invalid());
        }
        let mut value = "none";
        let mut visited = std::collections::BTreeSet::new();
        for action in &workflow.actions {
            let (input, output) = match action.as_str() {
                "capture" | "select" => ("none", "image"),
                "card" => ("image", "result"),
                "ocr" | "formula" => ("image", "draft"),
                "confirm" => ("draft", "confirmed"),
                "insert" => ("confirmed", "result"),
                _ => return Err(invalid()),
            };
            if input != value || !visited.insert(action) {
                return Err(invalid());
            }
            value = output;
        }
        if value != "result" {
            return Err(invalid());
        }
    }
    Ok(())
}
fn default_markdown_split_percent() -> u8 {
    50
}
fn default_annotation_color() -> String {
    "#ef6262".into()
}
fn default_annotation_width() -> u8 {
    3
}
fn default_fullscreen_shortcut() -> String {
    "Ctrl+Shift+1".into()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutAction {
    Region,
    Fullscreen,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            theme: "dark".into(),
            view_mode: "grid".into(),
            font_family: "system".into(),
            font_size: 16,
            capture_shortcut: "Ctrl+Shift+2".into(),
            fullscreen_shortcut: default_fullscreen_shortcut(),
            save_directory: None,
            annotation_color: default_annotation_color(),
            annotation_width: default_annotation_width(),
            close_to_tray: false,
            markdown_split_percent: default_markdown_split_percent(),
            workflows: vec![],
        }
    }
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreferencesPatch {
    pub theme: Option<String>,
    pub view_mode: Option<String>,
    pub font_family: Option<String>,
    pub font_size: Option<u8>,
    pub capture_shortcut: Option<String>,
    pub fullscreen_shortcut: Option<String>,
    pub annotation_color: Option<String>,
    pub annotation_width: Option<u8>,
    pub close_to_tray: Option<bool>,
    pub markdown_split_percent: Option<u8>,
    pub workflows: Option<Vec<WorkflowConfig>>,
    // Paths can only be changed by the Rust folder-dialog command.
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub preferences: Preferences,
    pub registered_shortcut: Option<String>,
    pub shortcut_error: Option<String>,
    pub registered_fullscreen_shortcut: Option<String>,
    pub fullscreen_shortcut_error: Option<String>,
    pub storage_error: Option<String>,
    pub directory_error: Option<String>,
}
struct SettingsState {
    snapshot: SettingsSnapshot,
    registration_fault: bool,
}
pub struct SettingsService {
    path: PathBuf,
    state: Mutex<SettingsState>,
    active_ids: AtomicU64,
    close_to_tray: AtomicBool,
}
fn unavailable() -> CaptureError {
    CaptureError::new(
        ErrorCode::SettingsUnavailable,
        "无法读写设置文件，已有设置未被重置。请检查文件权限或设置文件版本。",
    )
}
fn invalid() -> CaptureError {
    CaptureError::new(
        ErrorCode::InvalidSettings,
        "设置值无效，请检查字体、主题与快捷键。",
    )
}
pub fn shortcut(value: &str) -> Result<Shortcut, CaptureError> {
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(invalid());
    }
    let parsed: Shortcut = value.parse().map_err(|_| {
        CaptureError::new(
            ErrorCode::InvalidSettings,
            "快捷键格式无效，例如 Ctrl+Shift+2。",
        )
    })?;
    if !parsed
        .mods
        .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
    {
        return Err(CaptureError::new(
            ErrorCode::InvalidSettings,
            "全局截图快捷键必须包含 Ctrl、Alt 或 Meta 修饰键。",
        ));
    }
    Ok(parsed)
}
fn validate(value: &Preferences) -> Result<(), CaptureError> {
    validate_workflows(&value.workflows)?;
    if value.schema_version != 1
        || !["dark", "light", "system"].contains(&value.theme.as_str())
        || !["grid", "list"].contains(&value.view_mode.as_str())
        || !["system", "sans", "serif"].contains(&value.font_family.as_str())
        || !(12..=20).contains(&value.font_size)
        || !(20..=80).contains(&value.markdown_split_percent)
        || !(1..=32).contains(&value.annotation_width)
        || value.annotation_color.len() != 7
        || !value.annotation_color.starts_with('#')
        || !value.annotation_color.as_bytes()[1..]
            .iter()
            .all(u8::is_ascii_hexdigit)
    {
        return Err(invalid());
    }
    shortcut(&value.capture_shortcut)?;
    shortcut(&value.fullscreen_shortcut)?;
    if let Some(dir) = &value.save_directory
        && (dir.len() > 32768 || !Path::new(dir).is_absolute())
    {
        return Err(invalid());
    }
    Ok(())
}
fn read_preferences(path: &Path) -> Result<Preferences, CaptureError> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Preferences::default()),
        Err(_) => return Err(unavailable()),
    };
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    if bytes.len() > 65536 {
        return Err(unavailable());
    }
    let preferences: Preferences = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
    validate(&preferences).map_err(|_| unavailable())?;
    Ok(preferences)
}
fn with_directory_warning(mut snapshot: SettingsSnapshot) -> SettingsSnapshot {
    snapshot.directory_error = snapshot
        .preferences
        .save_directory
        .as_ref()
        .filter(|dir| !Path::new(dir).is_dir())
        .map(|_| "默认截图保存目录不存在或不可访问，请重新选择；原配置已保留。".into());
    snapshot
}
fn write_preferences(path: &Path, value: &Preferences) -> Result<(), CaptureError> {
    validate(value)?;
    // Revalidate before writing: preserve externally replaced, corrupt or future-version files.
    read_preferences(path)?;
    let parent = path.parent().ok_or_else(unavailable)?;
    fs::create_dir_all(parent).map_err(|_| unavailable())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| unavailable())?;
    let mut stage = tempfile::NamedTempFile::new_in(parent).map_err(|_| unavailable())?;
    stage
        .write_all(&bytes)
        .and_then(|_| stage.as_file().sync_all())
        .map_err(|_| unavailable())?;
    stage.persist(path).map_err(|_| unavailable())?;
    Ok(())
}
fn same_key(a: &str, b: &str) -> bool {
    shortcut(a).ok().map(|s| s.id()) == shortcut(b).ok().map(|s| s.id())
}
fn palette_key(key: &str) -> bool {
    same_key(key, "Ctrl+Shift+P")
}
fn bindings(snapshot: &SettingsSnapshot) -> [Option<String>; 2] {
    [
        snapshot.registered_shortcut.clone(),
        snapshot.registered_fullscreen_shortcut.clone(),
    ]
}
impl SettingsService {
    pub fn load(path: PathBuf) -> Self {
        let (preferences, storage_error) = match read_preferences(&path) {
            Ok(p) => (p, None),
            Err(e) => (Preferences::default(), Some(e.message)),
        };
        let close_to_tray = preferences.close_to_tray;
        Self {
            path,
            state: Mutex::new(SettingsState {
                snapshot: SettingsSnapshot {
                    preferences,
                    registered_shortcut: None,
                    shortcut_error: None,
                    registered_fullscreen_shortcut: None,
                    fullscreen_shortcut_error: None,
                    storage_error,
                    directory_error: None,
                },
                registration_fault: false,
            }),
            active_ids: AtomicU64::new(0),
            close_to_tray: AtomicBool::new(close_to_tray),
        }
    }
    pub fn snapshot(&self) -> Result<SettingsSnapshot, CaptureError> {
        let snapshot = self
            .state
            .lock()
            .map_err(|_| unavailable())?
            .snapshot
            .clone();
        Ok(with_directory_warning(snapshot))
    }
    pub fn available_directory(&self) -> Result<Option<String>, CaptureError> {
        let snapshot = self.snapshot()?;
        if let Some(error) = snapshot.directory_error {
            return Err(CaptureError::new(ErrorCode::SaveFailed, &error));
        }
        Ok(snapshot.preferences.save_directory)
    }
    pub fn prepare_shutdown(&self) -> Result<(), CaptureError> {
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        state.registration_fault = true;
        self.active_ids.store(0, Ordering::Release);
        state.snapshot.registered_shortcut = None;
        state.snapshot.registered_fullscreen_shortcut = None;
        state.snapshot.shortcut_error = Some("应用正在退出，快捷键触发已暂停。".into());
        state.snapshot.fullscreen_shortcut_error = state.snapshot.shortcut_error.clone();
        Ok(())
    }
    fn publish(&self, snapshot: &SettingsSnapshot) {
        self.close_to_tray
            .store(snapshot.preferences.close_to_tray, Ordering::Release);
        let keys = bindings(snapshot);
        let id = |key: &Option<String>| {
            key.as_ref()
                .and_then(|s| shortcut(s).ok())
                .map_or(0, |s| s.id()) as u64
        };
        // The callback never takes the settings lock while registration waits on the UI thread.
        self.active_ids
            .store(id(&keys[0]) | (id(&keys[1]) << 32), Ordering::Release);
    }
    pub fn action_for(&self, id: u32) -> Option<ShortcutAction> {
        if id == 0 {
            return None;
        }
        let active = self.active_ids.load(Ordering::Acquire);
        if active as u32 == id {
            Some(ShortcutAction::Region)
        } else if (active >> 32) as u32 == id {
            Some(ShortcutAction::Fullscreen)
        } else {
            None
        }
    }
    pub fn close_to_tray(&self) -> bool {
        self.close_to_tray.load(Ordering::Acquire)
    }
    pub fn initialize(
        &self,
        mut register: impl FnMut(&str, bool) -> Result<(), CaptureError>,
    ) -> Result<(), CaptureError> {
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        if state.snapshot.storage_error.is_some() || state.registration_fault {
            return Ok(());
        }
        if state.snapshot.registered_shortcut.is_none() {
            let key = state.snapshot.preferences.capture_shortcut.clone();
            match if palette_key(&key) {
                Err(CaptureError::new(
                    ErrorCode::ShortcutConflict,
                    "Ctrl+Shift+P已保留给命令面板，请重新设置区域截图快捷键；原配置保留。",
                ))
            } else {
                register(&key, true)
            } {
                Ok(()) => {
                    state.snapshot.registered_shortcut = Some(key);
                    state.snapshot.shortcut_error = None;
                }
                Err(e) => state.snapshot.shortcut_error = Some(e.message),
            }
        }
        if state.snapshot.registered_fullscreen_shortcut.is_none() {
            let key = state.snapshot.preferences.fullscreen_shortcut.clone();
            if palette_key(&key) {
                state.snapshot.fullscreen_shortcut_error = Some(
                    "Ctrl+Shift+P已保留给命令面板，请重新设置全屏截图快捷键；原配置保留。".into(),
                );
            } else if same_key(&key, &state.snapshot.preferences.capture_shortcut) {
                state.snapshot.fullscreen_shortcut_error =
                    Some("全屏与区域截图快捷键相同，请为全屏选择其他组合。区域设置已保留。".into());
            } else {
                match register(&key, true) {
                    Ok(()) => {
                        state.snapshot.registered_fullscreen_shortcut = Some(key);
                        state.snapshot.fullscreen_shortcut_error = None;
                    }
                    Err(e) => state.snapshot.fullscreen_shortcut_error = Some(e.message),
                }
            }
        }
        self.publish(&state.snapshot);
        Ok(())
    }
    pub fn update(
        &self,
        patch: PreferencesPatch,
        directory: Option<String>,
        mut registration: impl FnMut(&str, bool) -> Result<(), CaptureError>,
    ) -> Result<SettingsSnapshot, CaptureError> {
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        if state.snapshot.storage_error.is_some() || state.registration_fault {
            return Err(unavailable());
        }
        let edit_region = patch.capture_shortcut.is_some();
        let edit_full = patch.fullscreen_shortcut.is_some();
        let mut next = state.snapshot.preferences.clone();
        if let Some(v) = patch.theme {
            next.theme = v;
        }
        if let Some(v) = patch.view_mode {
            next.view_mode = v;
        }
        if let Some(v) = patch.font_family {
            next.font_family = v;
        }
        if let Some(v) = patch.font_size {
            next.font_size = v;
        }
        if let Some(v) = patch.capture_shortcut {
            next.capture_shortcut = shortcut(&v)?.to_string();
        }
        if let Some(v) = patch.fullscreen_shortcut {
            next.fullscreen_shortcut = shortcut(&v)?.to_string();
        }
        if let Some(v) = patch.annotation_color {
            next.annotation_color = v.to_ascii_lowercase();
        }
        if let Some(v) = patch.annotation_width {
            next.annotation_width = v;
        }
        if let Some(v) = patch.markdown_split_percent {
            next.markdown_split_percent = v;
        }
        if let Some(v) = patch.close_to_tray {
            next.close_to_tray = v;
        }
        if let Some(v) = patch.workflows {
            next.workflows = v;
        }
        if (edit_region && palette_key(&next.capture_shortcut))
            || (edit_full && palette_key(&next.fullscreen_shortcut))
        {
            return Err(CaptureError::new(
                ErrorCode::ShortcutConflict,
                "Ctrl+Shift+P已保留给命令面板，请选择其他截图快捷键。原绑定未改变。",
            ));
        }
        if let Some(v) = directory {
            next.save_directory = Some(v);
        }
        validate(&next)?;
        if (edit_region || edit_full) && same_key(&next.capture_shortcut, &next.fullscreen_shortcut)
        {
            return Err(CaptureError::new(
                ErrorCode::ShortcutConflict,
                "全屏与区域截图不能使用相同快捷键，请选择其他组合。原绑定未改变。",
            ));
        }
        let old = bindings(&state.snapshot);
        let desired = [
            (edit_region || old[0].is_some()).then(|| next.capture_shortcut.clone()),
            (edit_full || old[1].is_some()).then(|| next.fullscreen_shortcut.clone()),
        ];
        let mut added = Vec::with_capacity(2);
        let mut removed = Vec::with_capacity(2);
        let result = (|| {
            for key in desired.iter().flatten() {
                if !old.iter().flatten().any(|o| same_key(o, key)) {
                    registration(key, true)?;
                    added.push(key.clone());
                }
            }
            for key in old.iter().flatten() {
                if !desired.iter().flatten().any(|n| same_key(n, key)) {
                    registration(key, false)?;
                    removed.push(key.clone());
                }
            }
            write_preferences(&self.path, &next)
        })();
        if let Err(error) = result {
            let mut restored = true;
            for key in added.iter().rev() {
                if registration(key, false).is_err() {
                    restored = false;
                }
            }
            for key in removed.iter().rev() {
                if registration(key, true).is_err() {
                    restored = false;
                }
            }
            if !restored {
                state.registration_fault = true;
                self.active_ids.store(0, Ordering::Release);
                state.snapshot.registered_shortcut = None;
                state.snapshot.registered_fullscreen_shortcut = None;
                let message =
                    "快捷键恢复失败，触发已暂停。请重新启动应用释放并恢复绑定。".to_string();
                state.snapshot.shortcut_error = Some(message.clone());
                state.snapshot.fullscreen_shortcut_error = Some(message.clone());
                return Err(CaptureError::new(ErrorCode::SettingsUnavailable, &message));
            }
            return Err(error);
        }
        state.snapshot.preferences = next;
        state.snapshot.registered_shortcut = desired[0].clone();
        state.snapshot.registered_fullscreen_shortcut = desired[1].clone();
        if desired[0].is_some() {
            state.snapshot.shortcut_error = None;
        }
        if desired[1].is_some() {
            state.snapshot.fullscreen_shortcut_error = None;
        }
        self.publish(&state.snapshot);
        Ok(with_directory_warning(state.snapshot.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn workflow(actions: &[&str]) -> WorkflowConfig {
        WorkflowConfig {
            id: format!("custom-{}", "a".repeat(32)),
            title: "校对工作流".into(),
            actions: actions.iter().map(|s| s.to_string()).collect(),
        }
    }
    #[test]
    fn workflows_persist_reload_and_reject_unknown_actions_cycles_and_missing_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let service = registered_service(path.clone());
        let value = workflow(&["capture", "ocr", "confirm", "insert"]);
        service
            .update(
                PreferencesPatch {
                    workflows: Some(vec![value.clone()]),
                    ..Default::default()
                },
                None,
                |_, _| panic!("workflow changes must not touch global keys"),
            )
            .unwrap();
        assert_eq!(
            SettingsService::load(path.clone())
                .snapshot()
                .unwrap()
                .preferences
                .workflows,
            vec![value]
        );
        let bytes = fs::read(&path).unwrap();
        for actions in [
            &["capture", "shell"][..],
            &["capture", "insert"],
            &["select", "formula", "insert"],
            &["capture", "ocr", "capture", "card"],
            &["select"],
        ] {
            assert!(
                service
                    .update(
                        PreferencesPatch {
                            workflows: Some(vec![workflow(actions)]),
                            ..Default::default()
                        },
                        None,
                        |_, _| panic!("invalid workflows must not register")
                    )
                    .is_err()
            );
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        assert!(validate_workflows(&vec![workflow(&["select", "card"]); 17]).is_err());
        assert!(
            validate_workflows(&[workflow(&["select", "card"]), workflow(&["select", "card"])])
                .is_err()
        );
    }
    #[test]
    fn workflow_write_failure_preserves_snapshot_and_old_preferences_without_replaying_actions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let service = registered_service(path.clone());
        fs::write(&path, b"future or corrupt external settings").unwrap();
        assert!(
            service
                .update(
                    PreferencesPatch {
                        workflows: Some(vec![workflow(&["select", "card"])]),
                        ..Default::default()
                    },
                    None,
                    |_, _| panic!("no shortcuts")
                )
                .is_err()
        );
        assert!(service.snapshot().unwrap().preferences.workflows.is_empty());
        assert_eq!(
            fs::read(&path).unwrap(),
            b"future or corrupt external settings"
        );
    }
    #[test]
    fn palette_shortcut_conflict_is_blocked_and_legacy_binding_is_preserved_but_not_registered() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let service = registered_service(path.clone());
        for patch in [
            PreferencesPatch {
                capture_shortcut: Some("shift+control+KeyP".into()),
                ..Default::default()
            },
            PreferencesPatch {
                fullscreen_shortcut: Some("Ctrl+Shift+P".into()),
                ..Default::default()
            },
        ] {
            assert!(
                service
                    .update(patch, None, |_, _| panic!("reserved key cannot register"))
                    .is_err()
            );
        }
        let value = Preferences {
            capture_shortcut: "Ctrl+Shift+P".into(),
            ..Default::default()
        };
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let legacy = SettingsService::load(path.clone());
        legacy
            .initialize(|key, _| {
                assert!(!palette_key(key));
                Ok(())
            })
            .unwrap();
        let snapshot = legacy.snapshot().unwrap();
        assert!(snapshot.registered_shortcut.is_none());
        assert!(snapshot.shortcut_error.unwrap().contains("命令面板"));
        assert_eq!(
            read_preferences(&path).unwrap().capture_shortcut,
            "Ctrl+Shift+P"
        );
    }
    fn registered_service(path: PathBuf) -> SettingsService {
        let s = SettingsService::load(path);
        s.initialize(|_, _| Ok(())).unwrap();
        s
    }
    #[test]
    fn markdown_split_persists_and_rejects_invalid_values_without_reregistering() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let service = registered_service(path.clone());
        assert_eq!(
            service
                .snapshot()
                .unwrap()
                .preferences
                .markdown_split_percent,
            50
        );
        service
            .update(
                PreferencesPatch {
                    markdown_split_percent: Some(62),
                    ..Default::default()
                },
                None,
                |_, _| panic!("split must not register shortcuts"),
            )
            .unwrap();
        assert_eq!(
            SettingsService::load(path.clone())
                .snapshot()
                .unwrap()
                .preferences
                .markdown_split_percent,
            62
        );
        let bytes = fs::read(&path).unwrap();
        for value in [0, 19, 81, 255] {
            assert!(
                service
                    .update(
                        PreferencesPatch {
                            markdown_split_percent: Some(value),
                            ..Default::default()
                        },
                        None,
                        |_, _| panic!("invalid split")
                    )
                    .is_err()
            );
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
    }
    #[test]
    fn desktop_defaults_persist_reload_without_reregistering_shortcuts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = registered_service(path.clone());
        let snapshot = s
            .update(
                PreferencesPatch {
                    theme: Some("system".into()),
                    annotation_color: Some("#12ABef".into()),
                    annotation_width: Some(9),
                    close_to_tray: Some(true),
                    ..Default::default()
                },
                None,
                |_, _| panic!("appearance must not register again"),
            )
            .unwrap();
        assert_eq!(snapshot.preferences.annotation_color, "#12abef");
        assert!(s.close_to_tray());
        let loaded = SettingsService::load(path);
        let p = loaded.snapshot().unwrap().preferences;
        assert_eq!(p.theme, "system");
        assert_eq!(p.annotation_width, 9);
        assert_eq!(p.annotation_color, "#12abef");
        assert!(loaded.close_to_tray());
    }
    #[test]
    fn old_preferences_receive_safe_defaults_without_rewriting_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut json = serde_json::to_value(Preferences::default()).unwrap();
        for key in [
            "annotationColor",
            "annotationWidth",
            "closeToTray",
            "markdownSplitPercent",
        ] {
            json.as_object_mut().unwrap().remove(key);
        }
        let bytes = serde_json::to_vec(&json).unwrap();
        fs::write(&path, &bytes).unwrap();
        let s = SettingsService::load(path.clone());
        let p = s.snapshot().unwrap().preferences;
        assert_eq!(p.annotation_color, "#ef6262");
        assert_eq!(p.annotation_width, 3);
        assert!(!s.close_to_tray());
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    #[test]
    fn invalid_annotation_defaults_and_failed_close_setting_preserve_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = registered_service(path.clone());
        s.update(
            PreferencesPatch {
                close_to_tray: Some(true),
                ..Default::default()
            },
            None,
            |_, _| Ok(()),
        )
        .unwrap();
        let bytes = fs::read(&path).unwrap();
        for patch in [
            PreferencesPatch {
                annotation_color: Some("red<script>".into()),
                ..Default::default()
            },
            PreferencesPatch {
                annotation_width: Some(0),
                ..Default::default()
            },
            PreferencesPatch {
                annotation_width: Some(33),
                ..Default::default()
            },
            PreferencesPatch {
                theme: Some("unknown".into()),
                ..Default::default()
            },
        ] {
            assert!(
                s.update(patch, None, |_, _| panic!("invalid data must not register"))
                    .is_err()
            );
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        fs::write(&path, b"corrupt").unwrap();
        assert!(
            s.update(
                PreferencesPatch {
                    close_to_tray: Some(false),
                    ..Default::default()
                },
                None,
                |_, _| Ok(())
            )
            .is_err()
        );
        assert!(s.close_to_tray());
        assert_eq!(fs::read(path).unwrap(), b"corrupt");
    }
    #[test]
    fn unavailable_directory_is_reported_and_kept_without_reset_or_silent_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("截图");
        fs::create_dir(&folder).unwrap();
        let path = dir.path().join("settings.json");
        let s = registered_service(path.clone());
        s.update(
            PreferencesPatch::default(),
            Some(folder.to_string_lossy().into()),
            |_, _| Ok(()),
        )
        .unwrap();
        let bytes = fs::read(&path).unwrap();
        fs::remove_dir(&folder).unwrap();
        let snapshot = s.snapshot().unwrap();
        assert!(snapshot.directory_error.is_some());
        assert_eq!(
            snapshot.preferences.save_directory.as_deref(),
            folder.to_str()
        );
        assert!(s.available_directory().is_err());
        assert_eq!(fs::read(path).unwrap(), bytes);
        fs::create_dir(folder).unwrap();
        assert!(s.snapshot().unwrap().directory_error.is_none());
    }
    #[test]
    fn shutdown_stops_shortcut_dispatch_and_prevents_late_registration() {
        let dir = tempfile::tempdir().unwrap();
        let s = registered_service(dir.path().join("settings.json"));
        let id = shortcut("Ctrl+Shift+2").unwrap().id();
        assert!(s.action_for(id).is_some());
        s.prepare_shutdown().unwrap();
        s.prepare_shutdown().unwrap();
        assert!(s.action_for(id).is_none());
        assert!(s.snapshot().unwrap().registered_shortcut.is_none());
        assert!(s.snapshot().unwrap().shortcut_error.is_some());
        s.initialize(|_, _| panic!("late initialization must not register"))
            .unwrap();
        assert!(
            s.update(PreferencesPatch::default(), None, |_, _| panic!(
                "late update must not register"
            ))
            .is_err()
        );
    }
    #[test]
    fn legacy_preferences_keep_region_and_initialize_two_independent_bindings_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut legacy = serde_json::to_value(Preferences::default()).unwrap();
        legacy.as_object_mut().unwrap().remove("fullscreenShortcut");
        legacy["captureShortcut"] = "Alt+Shift+R".into();
        let bytes = serde_json::to_vec(&legacy).unwrap();
        fs::write(&path, &bytes).unwrap();
        let s = SettingsService::load(path.clone());
        let mut calls = vec![];
        s.initialize(|key, enable| {
            calls.push((key.to_string(), enable));
            Ok(())
        })
        .unwrap();
        s.initialize(|_, _| panic!("must not register twice"))
            .unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(
            s.snapshot().unwrap().preferences.capture_shortcut,
            "Alt+Shift+R"
        );
        assert_eq!(
            s.snapshot().unwrap().preferences.fullscreen_shortcut,
            "Ctrl+Shift+1"
        );
        assert_eq!(fs::read(path).unwrap(), bytes);
        assert_eq!(
            s.action_for(shortcut("Alt+Shift+R").unwrap().id()),
            Some(ShortcutAction::Region)
        );
        assert_eq!(
            s.action_for(shortcut("Ctrl+Shift+1").unwrap().id()),
            Some(ShortcutAction::Fullscreen)
        );
        assert_eq!(s.action_for(0), None);
    }
    #[test]
    fn fullscreen_edit_persists_reloads_and_does_not_reregister_region() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = registered_service(path.clone());
        let mut calls = vec![];
        let result = s
            .update(
                PreferencesPatch {
                    fullscreen_shortcut: Some("Ctrl+Shift+4".into()),
                    ..Default::default()
                },
                None,
                |key, enable| {
                    calls.push((key.to_string(), enable));
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(calls.len(), 2);
        assert!(same_key(&calls[0].0, "Ctrl+Shift+4") && calls[0].1);
        assert!(same_key(&calls[1].0, "Ctrl+Shift+1") && !calls[1].1);
        assert_eq!(result.registered_shortcut.as_deref(), Some("Ctrl+Shift+2"));
        let loaded = registered_service(path);
        assert_eq!(
            loaded.action_for(shortcut("Ctrl+Shift+4").unwrap().id()),
            Some(ShortcutAction::Fullscreen)
        );
        assert_eq!(
            loaded.action_for(shortcut("Ctrl+Shift+2").unwrap().id()),
            Some(ShortcutAction::Region)
        );
        assert_eq!(s.action_for(shortcut("Ctrl+Shift+1").unwrap().id()), None);
    }
    #[test]
    fn same_key_alias_and_external_conflict_preserve_both_old_bindings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = registered_service(path);
        assert_eq!(
            s.update(
                PreferencesPatch {
                    fullscreen_shortcut: Some("shift+control+Digit2".into()),
                    ..Default::default()
                },
                None,
                |_, _| panic!("reject before OS operations")
            )
            .unwrap_err()
            .code,
            ErrorCode::ShortcutConflict
        );
        assert!(
            s.update(
                PreferencesPatch {
                    fullscreen_shortcut: Some("Ctrl+Shift+4".into()),
                    ..Default::default()
                },
                None,
                |_, _| Err(CaptureError::new(ErrorCode::ShortcutConflict, "occupied"))
            )
            .is_err()
        );
        assert_eq!(
            s.action_for(shortcut("Ctrl+Shift+1").unwrap().id()),
            Some(ShortcutAction::Fullscreen)
        );
        assert_eq!(
            s.action_for(shortcut("Ctrl+Shift+2").unwrap().id()),
            Some(ShortcutAction::Region)
        );
    }
    #[test]
    fn startup_fullscreen_conflict_does_not_block_region_or_appearance_and_can_be_repaired() {
        let dir = tempfile::tempdir().unwrap();
        let s = SettingsService::load(dir.path().join("settings.json"));
        s.initialize(|key, _| {
            if same_key(key, "Ctrl+Shift+1") {
                Err(CaptureError::new(ErrorCode::ShortcutConflict, "occupied"))
            } else {
                Ok(())
            }
        })
        .unwrap();
        assert_eq!(
            s.action_for(shortcut("Ctrl+Shift+2").unwrap().id()),
            Some(ShortcutAction::Region)
        );
        assert!(s.snapshot().unwrap().fullscreen_shortcut_error.is_some());
        s.update(
            PreferencesPatch {
                theme: Some("light".into()),
                ..Default::default()
            },
            None,
            |_, _| panic!("appearance does not retry occupied keys"),
        )
        .unwrap();
        s.update(
            PreferencesPatch {
                fullscreen_shortcut: Some("Ctrl+Shift+4".into()),
                ..Default::default()
            },
            None,
            |_, _| Ok(()),
        )
        .unwrap();
        assert!(s.snapshot().unwrap().fullscreen_shortcut_error.is_none());
    }
    #[test]
    fn legacy_default_collision_keeps_original_region_and_fullscreen_can_be_changed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut legacy = serde_json::to_value(Preferences::default()).unwrap();
        legacy.as_object_mut().unwrap().remove("fullscreenShortcut");
        legacy["captureShortcut"] = "Ctrl+Shift+1".into();
        fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        let s = SettingsService::load(path);
        let mut count = 0;
        s.initialize(|_, _| {
            count += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(count, 1);
        assert_eq!(
            s.action_for(shortcut("Ctrl+Shift+1").unwrap().id()),
            Some(ShortcutAction::Region)
        );
        assert!(
            s.snapshot()
                .unwrap()
                .registered_fullscreen_shortcut
                .is_none()
        );
        s.update(
            PreferencesPatch {
                fullscreen_shortcut: Some("Ctrl+Shift+4".into()),
                ..Default::default()
            },
            None,
            |_, _| Ok(()),
        )
        .unwrap();
        assert_eq!(
            s.action_for(shortcut("Ctrl+Shift+1").unwrap().id()),
            Some(ShortcutAction::Region)
        );
    }
    #[test]
    fn pair_changes_roll_back_on_partial_conflict_and_failed_cleanup_pauses_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = registered_service(path);
        let patch = || PreferencesPatch {
            capture_shortcut: Some("Ctrl+Shift+3".into()),
            fullscreen_shortcut: Some("Ctrl+Shift+4".into()),
            ..Default::default()
        };
        let mut calls = vec![];
        assert!(
            s.update(patch(), None, |key, enable| {
                calls.push((key.to_string(), enable));
                if same_key(key, "Ctrl+Shift+4") && enable {
                    Err(invalid())
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        assert_eq!(calls.len(), 3);
        assert!(!calls[2].1);
        assert_eq!(
            s.action_for(shortcut("Ctrl+Shift+2").unwrap().id()),
            Some(ShortcutAction::Region)
        );
        assert!(
            s.update(patch(), None, |key, enable| {
                if same_key(key, "Ctrl+Shift+4") || !enable {
                    Err(invalid())
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        assert_eq!(s.action_for(shortcut("Ctrl+Shift+2").unwrap().id()), None);
        assert!(
            s.snapshot()
                .unwrap()
                .shortcut_error
                .unwrap()
                .contains("重新启动")
        );
        s.initialize(|_, _| panic!("fault requires restart"))
            .unwrap();
    }
    #[test]
    fn switching_actions_on_two_registered_keys_changes_mapping_without_duplicate_registration() {
        let dir = tempfile::tempdir().unwrap();
        let s = registered_service(dir.path().join("settings.json"));
        s.update(
            PreferencesPatch {
                capture_shortcut: Some("Ctrl+Shift+1".into()),
                fullscreen_shortcut: Some("Ctrl+Shift+2".into()),
                ..Default::default()
            },
            None,
            |_, _| panic!("same OS key set"),
        )
        .unwrap();
        assert_eq!(
            s.action_for(shortcut("Ctrl+Shift+1").unwrap().id()),
            Some(ShortcutAction::Region)
        );
        assert_eq!(
            s.action_for(shortcut("Ctrl+Shift+2").unwrap().id()),
            Some(ShortcutAction::Fullscreen)
        );
    }
    #[test]
    fn persists_known_preferences_and_reloads_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = registered_service(path.clone());
        s.update(
            PreferencesPatch {
                theme: Some("light".into()),
                font_family: Some("serif".into()),
                font_size: Some(18),
                ..Default::default()
            },
            Some(dir.path().to_string_lossy().into()),
            |_, _| panic!("unchanged shortcut must not register twice"),
        )
        .unwrap();
        let loaded = SettingsService::load(path).snapshot().unwrap().preferences;
        assert_eq!(loaded.theme, "light");
        assert_eq!(loaded.font_family, "serif");
        assert_eq!(loaded.font_size, 18);
        assert!(loaded.save_directory.is_some());
    }
    #[test]
    fn shortcut_conflict_keeps_old_binding_and_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = registered_service(path.clone());
        let e = s
            .update(
                PreferencesPatch {
                    capture_shortcut: Some("Ctrl+Shift+3".into()),
                    ..Default::default()
                },
                None,
                |_, enable| {
                    assert!(enable);
                    Err(CaptureError::new(
                        ErrorCode::ShortcutConflict,
                        "快捷键已占用",
                    ))
                },
            )
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::ShortcutConflict);
        assert!(!path.exists());
        assert_eq!(
            s.snapshot().unwrap().registered_shortcut.as_deref(),
            Some("Ctrl+Shift+2")
        );
    }
    #[test]
    fn invalid_and_future_settings_preserve_existing_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let bytes = br#"{"schemaVersion":999,"privateData":"keep"}"#;
        fs::write(&path, bytes).unwrap();
        let s = SettingsService::load(path.clone());
        assert!(s.snapshot().unwrap().storage_error.is_some());
        assert!(
            s.update(PreferencesPatch::default(), None, |_, _| panic!(
                "must not register"
            ))
            .is_err()
        );
        assert_eq!(fs::read(path).unwrap(), bytes);
        assert!(shortcut("2").is_err());
        assert!(shortcut("Ctrl+NotAKey").is_err());
        assert!(
            serde_json::from_str::<PreferencesPatch>(r#"{"saveDirectory":"C:/arbitrary"}"#)
                .is_err()
        );
    }
    #[test]
    fn failed_write_rolls_back_registration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = registered_service(path.clone());
        fs::write(&path, b"corrupt").unwrap();
        let mut calls = vec![];
        assert!(
            s.update(
                PreferencesPatch {
                    capture_shortcut: Some("Ctrl+Shift+3".into()),
                    ..Default::default()
                },
                None,
                |key, enable| {
                    calls.push((key.to_string(), enable));
                    Ok(())
                }
            )
            .is_err()
        );
        assert_eq!(calls.len(), 4);
        assert!(calls[0].1);
        assert!(!calls[1].1);
        assert!(!calls[2].1);
        assert!(calls[3].1);
        assert_eq!(
            s.action_for(shortcut("Ctrl+Shift+2").unwrap().id()),
            Some(ShortcutAction::Region)
        );
        assert_eq!(fs::read(path).unwrap(), b"corrupt");
    }
}
