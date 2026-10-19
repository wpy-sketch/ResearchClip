//! Passwords never enter SQLite or a serialized settings response.
use super::{model::*, transport::Credentials};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Secret {
    username: String,
    password: String,
    device: String,
}
pub fn save(id: &str, auth: Credentials) -> Result<()> {
    if !crate::storage::files::id_valid(id)
        || auth.username.is_empty()
        || auth.username.len() > 512
        || auth.username.contains(['\0', '\r', '\n'])
        || auth.password.is_empty()
        || auth.password.len() > 1536
        || !crate::storage::files::id_valid(&auth.device)
    {
        return Err(invalid("用户名或应用授权密码无效/超过限制。"));
    }
    let bytes = json(&Secret {
        username: auth.username.clone(),
        password: auth.password.clone(),
        device: auth.device.clone(),
    })?;
    if bytes.len() > 2560 {
        return Err(invalid("凭据超过系统安全存储大小限制。"));
    }
    write(id, &bytes)
}
pub fn load(id: &str) -> Result<Credentials> {
    if !crate::storage::files::id_valid(id) {
        return Err(invalid("凭据标识无效。"));
    }
    let value: Secret = parse(&read(id)?, 2560)?;
    if !crate::storage::files::id_valid(&value.device) {
        return Err(invalid("本机设备身份无效，请重新保存连接。"));
    }
    Ok(Credentials {
        username: value.username,
        password: value.password,
        device: value.device,
    })
}
#[cfg(target_os = "windows")]
fn target(id: &str) -> Vec<u16> {
    format!("ResearchClip/WebDAV/{id}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
#[cfg(target_os = "windows")]
fn write(id: &str, bytes: &[u8]) -> Result<()> {
    use windows::{Win32::Security::Credentials::*, core::PWSTR};
    let mut name = target(id);
    let mut account: Vec<u16> = "ResearchClip".encode_utf16().chain(Some(0)).collect();
    let mut data = bytes.to_vec();
    let credential = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(name.as_mut_ptr()),
        CredentialBlobSize: data.len() as u32,
        CredentialBlob: data.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: PWSTR(account.as_mut_ptr()),
        ..Default::default()
    };
    let result = unsafe { CredWriteW(&credential, 0) }
        .map_err(|_| invalid("Windows凭据管理器保存失败；没有回退明文存储。"));
    data.fill(0);
    result
}
#[cfg(target_os = "windows")]
fn read(id: &str) -> Result<Vec<u8>> {
    use windows::{Win32::Security::Credentials::*, core::PCWSTR};
    let name = target(id);
    let mut pointer = std::ptr::null_mut();
    unsafe { CredReadW(PCWSTR(name.as_ptr()), CRED_TYPE_GENERIC, None, &mut pointer) }
        .map_err(|_| invalid("系统凭据不可用，请在WebDAV设置中重新输入账号及应用授权密码。"))?;
    if pointer.is_null() {
        return Err(invalid("系统返回空凭据。"));
    }
    let result = unsafe {
        let value = &*pointer;
        if value.CredentialBlobSize > 2560 || value.CredentialBlob.is_null() {
            Err(invalid("系统凭据格式无效。"))
        } else {
            Ok(
                std::slice::from_raw_parts(value.CredentialBlob, value.CredentialBlobSize as usize)
                    .to_vec(),
            )
        }
    };
    unsafe { CredFree(pointer.cast()) };
    result
}
#[cfg(not(target_os = "windows"))]
fn write(_: &str, _: &[u8]) -> Result<()> {
    Err(invalid(
        "当前平台尚未适配系统安全凭据存储；WebDAV暂仅支持Windows。",
    ))
}
#[cfg(not(target_os = "windows"))]
fn read(_: &str) -> Result<Vec<u8>> {
    Err(invalid("当前平台尚未适配系统安全凭据存储。"))
}
#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    #[test]
    fn isolated_windows_credential_roundtrip() {
        let root = tempfile::tempdir().unwrap();
        let storage = crate::storage::Storage::open(root.path().into()).unwrap();
        let id = storage.sync_id_for_test().unwrap();
        save(
            &id,
            Credentials {
                username: "test-fixture".into(),
                password: "not-a-user-password".into(),
                device: "e".repeat(32),
            },
        )
        .unwrap();
        let actual = load(&id).unwrap();
        assert_eq!(actual.password, "not-a-user-password");
        assert_eq!(actual.device, "e".repeat(32));
        let name = target(&id);
        unsafe {
            windows::Win32::Security::Credentials::CredDeleteW(
                windows::core::PCWSTR(name.as_ptr()),
                windows::Win32::Security::Credentials::CRED_TYPE_GENERIC,
                None,
            )
        }
        .unwrap();
        assert!(load(&id).is_err());
    }
}
