use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const VERSION: u32 = 1;
pub const MAX_MESSAGE: usize = 768 * 1024;
pub const EXTENSION_ID: &str = include_str!("../extension-id.txt");
pub const HOST_NAME: &str = "io.researchclip.clipper";
pub fn origin_valid(origin: &str) -> bool {
    origin == format!("chrome-extension://{EXTENSION_ID}/")
}
pub fn id_valid(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn web_url(value: &str) -> bool {
    value.len() <= 4096
        && !value.chars().any(char::is_control)
        && url::Url::parse(value).is_ok_and(|u| {
            matches!(u.scheme(), "http" | "https")
                && u.host_str().is_some()
                && u.username().is_empty()
                && u.password().is_none()
        })
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn canonical_url(value: &str) -> Result<String, &'static str> {
    if !web_url(value) {
        return Err("网址无效。");
    }
    url::Url::parse(value)
        .map(|u| u.to_string())
        .map_err(|_| "网址无效。")
}
pub fn random_token() -> io::Result<String> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(io::Error::other)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClipImage {
    pub id: String,
    pub url: String,
    pub alt: String,
    pub png_base64: Option<String>,
    pub warning: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Clip {
    pub title: String,
    pub url: String,
    pub site_name: String,
    pub byline: String,
    pub mode: String,
    pub markdown: String,
    pub project_id: Option<String>,
    pub images: Vec<ClipImage>,
}
impl Clip {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.title.trim().is_empty()
            || self.title.chars().count() > 200
            || self.title.chars().any(char::is_control)
            || !web_url(&self.url)
            || !["link", "selection", "article", "images"].contains(&self.mode.as_str())
            || self.site_name.len() > 512
            || self.byline.len() > 512
            || self.markdown.len() > 192 * 1024
            || self.markdown.contains('\0')
            || self.images.len() > 32
            || self.project_id.as_deref().is_some_and(|p| !id_valid(p))
        {
            return Err("剪藏结构、网址或长度无效。");
        }
        let mut ids = std::collections::HashSet::new();
        let mut total = 0;
        let mut copies = 0;
        for img in &self.images {
            if !id_valid(&img.id)
                || !ids.insert(&img.id)
                || !web_url(&img.url)
                || img.alt.len() > 800
                || img.warning.as_ref().is_some_and(|s| s.len() > 512)
            {
                return Err("图片引用无效。");
            }
            if let Some(encoded) = &img.png_base64 {
                if encoded.len() > 350_000 {
                    return Err("单张图片超过256 KiB。");
                }
                let bytes = STANDARD.decode(encoded).map_err(|_| "图片编码无效。")?;
                if bytes.len() > 256 * 1024 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                    return Err("图片类型或大小无效。");
                }
                total += bytes.len();
                copies += 1;
            }
        }
        if total > 384 * 1024 || copies > 4 {
            return Err("最多复制4图，图片合计384 KiB。");
        }
        Ok(())
    }
    pub fn png(&self, image: &ClipImage) -> io::Result<Option<Vec<u8>>> {
        image
            .png_base64
            .as_ref()
            .map(|s| STANDARD.decode(s).map_err(io::Error::other))
            .transpose()
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub request_id: String,
    pub operation: String,
    pub clip: Option<Clip>,
}
impl Request {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != VERSION
            || !id_valid(&self.request_id)
            || !["ping", "projects", "save"].contains(&self.operation.as_str())
        {
            return Err("不支持的协议、请求ID或操作。");
        }
        if self.operation == "save" {
            self.clip.as_ref().ok_or("缺少剪藏内容。")?.validate()
        } else if self.clip.is_some() {
            Err("该操作不接受剪藏正文。")
        } else {
            Ok(())
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub token: String,
    pub origin: String,
    pub request: Request,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub version: u32,
    pub port: u16,
    pub token: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reply {
    pub ok: bool,
    pub code: String,
    pub message: String,
    pub data: serde_json::Value,
}
impl Reply {
    pub fn error(code: &str, message: &str) -> Self {
        Self {
            ok: false,
            code: code.into(),
            message: message.into(),
            data: serde_json::Value::Null,
        }
    }
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            ok: true,
            code: "OK".into(),
            message: "已完成".into(),
            data,
        }
    }
}
pub fn read_frame<T: DeserializeOwned>(input: &mut impl Read) -> io::Result<T> {
    let mut length = [0; 4];
    input.read_exact(&mut length)?;
    let size = u32::from_le_bytes(length) as usize;
    if size == 0 || size > MAX_MESSAGE {
        return Err(io::Error::other("message length rejected"));
    }
    let mut bytes = vec![0; size];
    input.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}
pub fn write_frame(output: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    if bytes.len() > MAX_MESSAGE {
        return Err(io::Error::other("response too large"));
    }
    output.write_all(&(bytes.len() as u32).to_le_bytes())?;
    output.write_all(&bytes)?;
    output.flush()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frames_are_byte_counted_and_bounded() {
        let mut bytes = Vec::new();
        write_frame(
            &mut bytes,
            &Reply::success(serde_json::json!({"text":"中文\nEnglish"})),
        )
        .unwrap();
        let r: Reply = read_frame(&mut bytes.as_slice()).unwrap();
        assert!(r.ok);
        assert!(
            read_frame::<Reply>(&mut (MAX_MESSAGE as u32 + 1).to_le_bytes().as_slice()).is_err()
        );
        assert!(read_frame::<Reply>(&mut &b"\x08\0\0\0{}"[..]).is_err());
    }
    #[test]
    fn origin_urls_and_unknown_fields_are_rejected() {
        assert!(origin_valid(&format!("chrome-extension://{EXTENSION_ID}/")));
        assert!(!origin_valid(
            "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/"
        ));
        for s in [
            "file:///C:/secret",
            "javascript:alert(1)",
            "https://user:password@example.org/",
            "https://example.org/\n",
        ] {
            assert!(!web_url(s));
        }
        assert!(web_url("https://example.org/中文?q=1"));
        assert!(serde_json::from_str::<Request>(r#"{"version":1,"requestId":"00000000000000000000000000000000","operation":"ping","clip":null,"path":"C:/"}"#).is_err());
    }
    #[test]
    fn protocol_version_and_save_shape() {
        let mut r = Request {
            version: 1,
            request_id: "a".repeat(32),
            operation: "save".into(),
            clip: None,
        };
        assert!(r.validate().is_err());
        r.operation = "ping".into();
        assert!(r.validate().is_ok());
        r.version = 2;
        assert!(r.validate().is_err());
        assert_eq!(random_token().unwrap().len(), 64);
    }
}
