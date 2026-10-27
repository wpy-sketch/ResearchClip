//! Official local write API. Mutation transport never retries or follows redirects.
use super::*;
use crate::services::{
    zotero_annotations::{AnnotationSource, ZoteroAnnotation, parse_annotation},
    zotero_write_model::{
        AnnotationWriteCapabilities, AnnotationWriteInput, WRITE_TYPES, tested_write_version,
    },
};
type WriteResult<T> = std::result::Result<T, ZoteroWriteError>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZoteroWriteError {
    pub code: &'static str,
    pub message: &'static str,
}
fn fail(code: &'static str, message: &'static str) -> ZoteroWriteError {
    ZoteroWriteError { code, message }
}
fn read_failure(error: String) -> ZoteroWriteError {
    if error.contains("取消") {
        return fail("ZOTERO_WRITE_CANCELLED", "Zotero操作发送前已取消。");
    }
    if error.contains("实例") || error == crate::services::zotero_annotations::SOURCE_MISMATCH {
        fail(
            "ZOTERO_WRITE_SOURCE_MISMATCH",
            "Zotero实例、库或附件不一致，写入已停止，请核对来源。",
        )
    } else {
        fail(
            "ZOTERO_WRITE_UNAVAILABLE",
            "Zotero读取或来源核对失败；未自动重试写入，请检查连接及原件授权。",
        )
    }
}
// Never Serialize or Debug a grant: the key stays in native memory.
#[derive(Clone)]
pub struct LocalWriteGrant {
    source: AnnotationSource,
    server_id: String,
    key: String,
    remembered: bool,
}
impl LocalWriteGrant {
    pub fn matches(&self, source: &AnnotationSource) -> bool {
        &self.source == source
    }
    pub fn remembered(&self) -> bool {
        self.remembered
    }
}
#[derive(Clone)]
pub struct WriteIntent {
    source: AnnotationSource,
    key: String,
    method: &'static str,
    body: Option<Value>,
    version: Option<u64>,
    expected: Option<ZoteroAnnotation>,
}
impl WriteIntent {
    pub fn source(&self) -> &AnnotationSource {
        &self.source
    }
    pub fn annotation_key(&self) -> &str {
        &self.key
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoteroWriteOutcome {
    pub verified: bool,
    pub annotation_key: String,
    pub annotation: Option<ZoteroAnnotation>,
    pub message: &'static str,
}
impl ZoteroClient {
    pub fn annotation_write_capabilities(
        &self,
        source: &AnnotationSource,
        original: &Path,
        cancel: &AtomicBool,
    ) -> WriteResult<AnnotationWriteCapabilities> {
        let (health, _) = self
            .annotation_context(source, original, cancel, Instant::now())
            .map_err(read_failure)?;
        let supported = tested_write_version(health.product_version.as_deref());
        Ok(AnnotationWriteCapabilities { product_version: health.product_version, supported_types: if supported { WRITE_TYPES.iter().map(|s| (*s).into()).collect() } else { vec![] }, authorized: false, remembered: false, message: if supported { "已核实此客户端的高亮、下划线和普通文字便签接口；写入仍需当前来源授权，权限由Zotero最终检查。" } else { "此客户端版本未完成写入验证，所有写入类型禁用；仍可只读查看。" }.into() })
    }
    pub fn authorize_annotation_writes(
        &self,
        source: &AnnotationSource,
        original: &Path,
        cancel: &AtomicBool,
    ) -> WriteResult<LocalWriteGrant> {
        let (health, _) = self
            .annotation_context(source, original, cancel, Instant::now())
            .map_err(read_failure)?;
        if !tested_write_version(health.product_version.as_deref()) {
            return Err(fail(
                "ZOTERO_WRITE_UNSUPPORTED",
                "此Zotero版本尚未验证写入，未请求授权。",
            ));
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(fail("ZOTERO_WRITE_CANCELLED", "授权等待已取消。"));
        }
        let (_, value) = self.annotation_write_request(
            "POST",
            "local/authorize",
            Some(&json!({"appName":"ResearchClip"})),
            None,
            None,
            &health.server_id,
            true,
        )?;
        if cancel.load(Ordering::Relaxed) {
            return Err(fail(
                "ZOTERO_WRITE_CANCELLED",
                "授权等待已取消，返回的密钥未保留。",
            ));
        }
        let value = value.ok_or(fail("ZOTERO_WRITE_PROTOCOL", "Zotero授权响应缺失。"))?;
        let key = value["key"]
            .as_str()
            .filter(|key| key.len() == 32 && key.bytes().all(|b| b.is_ascii_alphanumeric()))
            .ok_or(fail("ZOTERO_WRITE_PROTOCOL", "Zotero未返回有效本机授权。"))?;
        let remembered = value["remember"]
            .as_bool()
            .ok_or(fail("ZOTERO_WRITE_PROTOCOL", "Zotero授权类型缺失。"))?;
        Ok(LocalWriteGrant {
            source: source.clone(),
            server_id: health.server_id,
            key: key.into(),
            remembered,
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn annotation_write_request(
        &self,
        method: &str,
        path: &str,
        value: Option<&Value>,
        version: Option<u64>,
        key: Option<&str>,
        server: &str,
        authorization: bool,
    ) -> WriteResult<(u16, Option<Value>)> {
        let url = self
            .base
            .join(path)
            .map_err(|_| fail("ZOTERO_WRITE_INVALID", "写入路径无效。"))?;
        if url.origin() != self.base.origin()
            || !url.path().starts_with("/api/")
            || !matches!(method, "POST" | "PATCH" | "DELETE")
        {
            return Err(fail("ZOTERO_WRITE_INVALID", "写入请求超出受控本机接口。"));
        }
        let method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| fail("ZOTERO_WRITE_INVALID", "写入方法无效。"))?;
        let mut request = self
            .client
            .request(method.clone(), url)
            .header("Zotero-API-Version", "3")
            .header("Zotero-Server-ID", server)
            .header("User-Agent", "ResearchClip-Scoped-Annotations")
            .timeout(Duration::from_secs(if authorization { 90 } else { 8 }));
        if let Some(key) = key {
            request = request.header("Zotero-API-Key", key);
        }
        if let Some(version) = version {
            request = request.header("If-Unmodified-Since-Version", version);
        }
        if let Some(value) = value {
            // Zotero10.0.6 imports annotation properties in JSON field order.
            // serde_json::Value uses sorted maps; explicitly serialize the type first.
            #[derive(Serialize)]
            struct OrderedAnnotation<'a> {
                #[serde(rename = "annotationType")]
                kind: &'a str,
                #[serde(flatten)]
                rest: Value,
            }
            let encoded = if !authorization && method == reqwest::Method::POST {
                let item = value
                    .as_array()
                    .filter(|a| a.len() == 1)
                    .and_then(|a| a.first())
                    .ok_or(fail("ZOTERO_WRITE_INVALID", "创建只允许一个标准批注。"))?;
                let kind = item["annotationType"]
                    .as_str()
                    .ok_or(fail("ZOTERO_WRITE_INVALID", "批注类型缺失。"))?;
                let mut rest = item.clone();
                rest.as_object_mut()
                    .ok_or(fail("ZOTERO_WRITE_INVALID", "批注对象无效。"))?
                    .remove("annotationType");
                serde_json::to_vec(&[OrderedAnnotation { kind, rest }])
            } else {
                serde_json::to_vec(value)
            };
            let bytes = encoded.map_err(|_| fail("ZOTERO_WRITE_INVALID", "批注参数无法编码。"))?;
            if bytes.len() > 512 * 1024 {
                return Err(fail("ZOTERO_WRITE_INVALID", "批注请求超过512KiB预算。"));
            }
            request = request
                .header("Content-Type", "application/json")
                .body(bytes);
        }
        let response = request.send().map_err(|_| {
            if authorization {
                fail(
                    "ZOTERO_WRITE_AUTH_WAIT",
                    "未完成Zotero授权等待；请检查Zotero中的对话框。取消等待不会代替关闭授权框。",
                )
            } else {
                fail(
                    "ZOTERO_WRITE_UNKNOWN",
                    "写入响应未收到，可能已经提交；请复核，未自动重试。",
                )
            }
        })?;
        let status = response.status().as_u16();
        let headers = response.headers();
        // Installed Zotero10.0.6 can return early authentication failures without
        // LocalAPI response headers. Such denials never prove a mutation succeeded.
        // Successful or ambiguous responses still require both identity headers.
        let response_server = headers
            .get("Zotero-Server-ID")
            .and_then(|v| v.to_str().ok());
        let early_denial = matches!(
            status,
            400 | 401 | 403 | 404 | 405 | 409 | 412 | 413 | 428 | 429 | 501
        );
        if response_server.is_some_and(|value| value != server)
            || response_server.is_none() && !early_denial
        {
            return Err(fail(
                "ZOTERO_WRITE_SOURCE_MISMATCH",
                "Zotero实例响应不一致，请核对来源并重新授权。",
            ));
        }
        if headers
            .get("Zotero-API-Version")
            .and_then(|v| v.to_str().ok())
            != Some("3")
            && !early_denial
        {
            return Err(fail(
                "ZOTERO_WRITE_PROTOCOL",
                "写入API版本不匹配，请复核状态。",
            ));
        }
        match status {
            200 | 204 => {}
            401 => {
                return Err(fail(
                    "ZOTERO_WRITE_AUTH_REQUIRED",
                    "本机写入授权不存在、已使用或被撤销，请主动重新授权。",
                ));
            }
            403 => {
                return Err(fail(
                    "ZOTERO_WRITE_FORBIDDEN",
                    "Zotero拒绝授权或资料库/批注只读；未放宽权限，请在Zotero核对。",
                ));
            }
            409 | 412 => {
                return Err(fail(
                    "ZOTERO_WRITE_CONFLICT",
                    "Zotero对象版本或资料库状态已改变，请先重新读取，未覆盖新修改。",
                ));
            }
            428 => {
                return Err(fail(
                    "ZOTERO_WRITE_PRECONDITION",
                    "Zotero要求有效的实例和对象版本前提。",
                ));
            }
            429 => {
                return Err(fail(
                    "ZOTERO_WRITE_RATE_LIMIT",
                    "Zotero授权请求过于频繁，请等待至少一分钟后主动重试。",
                ));
            }
            404 if !authorization && path.contains("/items/") => {
                return Err(fail(
                    "ZOTERO_WRITE_NOT_FOUND",
                    "目标批注已删除或不可用，请先读取并解决冲突。",
                ));
            }
            404 | 405 | 501 => {
                return Err(fail(
                    "ZOTERO_WRITE_UNSUPPORTED",
                    "Zotero未提供此写入接口或目标已不存在；没有伪装成功。",
                ));
            }
            400 | 413 => {
                return Err(fail(
                    "ZOTERO_WRITE_INVALID",
                    "Zotero拒绝批注参数或大小，请检查内容。",
                ));
            }
            _ => {
                return Err(fail(
                    "ZOTERO_WRITE_UNKNOWN",
                    "Zotero写入状态未知，请重新读取核实；未自动重试。",
                ));
            }
        }
        if response
            .content_length()
            .is_some_and(|v| v > BODY_LIMIT as u64)
        {
            return Err(fail(
                "ZOTERO_WRITE_UNKNOWN",
                "写入响应超过保护预算，请复核结果。",
            ));
        }
        let mut bytes = Vec::new();
        response
            .take((BODY_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| fail("ZOTERO_WRITE_UNKNOWN", "写入响应中断，请复核结果。"))?;
        if bytes.len() > BODY_LIMIT {
            return Err(fail(
                "ZOTERO_WRITE_UNKNOWN",
                "写入响应超过保护预算，请复核结果。",
            ));
        }
        let value =
            if bytes.is_empty() {
                None
            } else {
                Some(serde_json::from_slice(&bytes).map_err(|_| {
                    fail("ZOTERO_WRITE_UNKNOWN", "写入响应格式无效，请复核实际结果。")
                })?)
            };
        Ok((status, value))
    }
    pub fn prepare_annotation_write(
        &self,
        source: &AnnotationSource,
        original: &Path,
        input: &AnnotationWriteInput,
        cancel: &AtomicBool,
    ) -> WriteResult<WriteIntent> {
        self.prepare_annotation_write_reserved(source, original, input, None, cancel)
    }
    pub(crate) fn read_annotation_for_sync(
        &self,
        source: &AnnotationSource,
        original: &Path,
        key: &str,
        cancel: &AtomicBool,
    ) -> WriteResult<Option<ZoteroAnnotation>> {
        if !valid_item_key(key) {
            return Err(fail("ZOTERO_WRITE_INVALID", "批注原始键无效。"));
        }
        let start = Instant::now();
        let (health, scope) = self
            .annotation_context(source, original, cancel, start)
            .map_err(read_failure)?;
        self.item(&scope, key, &health, cancel, start)
            .map_err(read_failure)?
            .map(|v| parse_annotation(&v, source).map_err(read_failure))
            .transpose()
    }
    pub(crate) fn prepare_annotation_write_reserved(
        &self,
        source: &AnnotationSource,
        original: &Path,
        input: &AnnotationWriteInput,
        reserved: Option<&str>,
        cancel: &AtomicBool,
    ) -> WriteResult<WriteIntent> {
        input
            .validate()
            .map_err(|message| fail("ZOTERO_WRITE_INVALID", message))?;
        let start = Instant::now();
        let (health, scope) = self
            .annotation_context(source, original, cancel, start)
            .map_err(read_failure)?;
        if !tested_write_version(health.product_version.as_deref()) {
            return Err(fail("ZOTERO_WRITE_UNSUPPORTED", "此客户端尚未验证写入。"));
        }
        match input {
            AnnotationWriteInput::Create {
                annotation_type,
                color,
                text,
                comment,
                geometry,
            } => {
                let mut key = String::new();
                if let Some(reserved) = reserved {
                    if !valid_item_key(reserved) {
                        return Err(fail("ZOTERO_WRITE_INVALID", "固定批注局部键无效。"));
                    }
                    if self
                        .item(&scope, reserved, &health, cancel, start)
                        .map_err(read_failure)?
                        .is_some()
                    {
                        return Err(fail(
                            "ZOTERO_WRITE_CONFLICT",
                            "固定批注键已存在，请复核，未自动生成新键或覆盖。",
                        ));
                    }
                    key = reserved.into();
                } else {
                    for _ in 0..3 {
                        key = crate::services::zotero_write_model::new_foreign_annotation_key()
                            .map_err(|m| fail("ZOTERO_WRITE_INVALID", m))?;
                        if self
                            .item(&scope, &key, &health, cancel, start)
                            .map_err(read_failure)?
                            .is_none()
                        {
                            break;
                        }
                        key.clear();
                    }
                }
                if key.is_empty() {
                    return Err(fail("ZOTERO_WRITE_CONFLICT", "新批注键冲突，请重新创建。"));
                }
                let raw = geometry
                    .raw_rectangles()
                    .map_err(|message| fail("ZOTERO_WRITE_INVALID", message))?;
                let mut data = json!({"key":key,"version":0,"itemType":"annotation","parentItem":source.attachment_key,"annotationType":annotation_type,"annotationColor":color.to_lowercase(),"annotationComment":comment,"annotationPageLabel":(geometry.page_index+1).to_string(),"annotationSortIndex":format!("{:05}|000000|00000",geometry.page_index),"annotationPosition":serde_json::to_string(&json!({"pageIndex":geometry.page_index,"rects":raw})).map_err(|_|fail("ZOTERO_WRITE_INVALID", "坐标无法编码。"))?});
                if annotation_type != "note" {
                    data["annotationText"] = json!(text);
                }
                let expected=parse_annotation(&json!({"key":key,"version":0,"library":{"type":source.library_type,"id":source.library_id},"data":data}),source).map_err(read_failure)?;
                Ok(WriteIntent {
                    source: source.clone(),
                    key,
                    method: "POST",
                    body: Some(json!([data])),
                    version: None,
                    expected: Some(expected),
                })
            }
            AnnotationWriteInput::Update {
                annotation_key,
                expected_version,
                annotation_type,
                color,
                text,
                comment,
            } => {
                let mut old = self.editable_annotation(
                    source,
                    &scope,
                    &health,
                    annotation_key,
                    *expected_version,
                    cancel,
                    start,
                )?;
                if old.annotation_type != *annotation_type {
                    return Err(fail("ZOTERO_WRITE_INVALID", "不允许通过更新改变批注类型。"));
                }
                old.color = color.to_lowercase();
                old.text = text.clone();
                old.comment = comment.clone();
                let mut body = json!({"annotationColor":old.color,"annotationComment":comment});
                if annotation_type != "note" {
                    body["annotationText"] = json!(text);
                }
                Ok(WriteIntent {
                    source: source.clone(),
                    key: annotation_key.clone(),
                    method: "PATCH",
                    body: Some(body),
                    version: Some(*expected_version),
                    expected: Some(old),
                })
            }
            AnnotationWriteInput::Delete {
                annotation_key,
                expected_version,
            } => {
                self.editable_annotation(
                    source,
                    &scope,
                    &health,
                    annotation_key,
                    *expected_version,
                    cancel,
                    start,
                )?;
                Ok(WriteIntent {
                    source: source.clone(),
                    key: annotation_key.clone(),
                    method: "DELETE",
                    body: None,
                    version: Some(*expected_version),
                    expected: None,
                })
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn editable_annotation(
        &self,
        source: &AnnotationSource,
        scope: &ZoteroScope,
        health: &ZoteroHealth,
        key: &str,
        version: u64,
        cancel: &AtomicBool,
        start: Instant,
    ) -> WriteResult<ZoteroAnnotation> {
        let value = self
            .item(scope, key, health, cancel, start)
            .map_err(read_failure)?
            .ok_or(fail(
                "ZOTERO_WRITE_NOT_FOUND",
                "目标批注已经不存在，请重新读取。",
            ))?;
        let annotation = parse_annotation(&value, source).map_err(read_failure)?;
        if !WRITE_TYPES.contains(&annotation.annotation_type.as_str())
            || annotation.position.is_none()
        {
            return Err(fail(
                "ZOTERO_WRITE_UNSUPPORTED",
                "本版不修改或删除未验证类型及不支持的位置。",
            ));
        }
        if value["data"]["annotationIsExternal"] == true || value["data"]["isExternal"] == true {
            return Err(fail(
                "ZOTERO_WRITE_FORBIDDEN",
                "外部批注标记为只读，请在Zotero核对。",
            ));
        }
        if annotation.version != version {
            return Err(fail(
                "ZOTERO_WRITE_CONFLICT",
                "批注版本已改变，未覆盖新修改。",
            ));
        }
        Ok(annotation)
    }
    pub fn perform_annotation_write(
        &self,
        intent: &WriteIntent,
        grant: &LocalWriteGrant,
        original: &Path,
        cancel: &AtomicBool,
    ) -> WriteResult<ZoteroWriteOutcome> {
        let (health, scope) = self
            .annotation_context(&intent.source, original, cancel, Instant::now())
            .map_err(read_failure)?;
        if !grant.matches(&intent.source) || grant.server_id != health.server_id {
            return Err(fail(
                "ZOTERO_WRITE_SOURCE_MISMATCH",
                "本次授权不属于当前完整来源。",
            ));
        }
        if !tested_write_version(health.product_version.as_deref()) {
            return Err(fail(
                "ZOTERO_WRITE_UNSUPPORTED",
                "写入前客户端版本已变化，请重新核实能力。",
            ));
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(fail("ZOTERO_WRITE_CANCELLED", "写入发送前已取消。"));
        }
        let path = if intent.method == "POST" {
            format!("{}/items", scope.prefix())
        } else {
            format!("{}/items/{}", scope.prefix(), intent.key)
        };
        match self.annotation_write_request(
            intent.method,
            &path,
            intent.body.as_ref(),
            intent.version,
            Some(&grant.key),
            &health.server_id,
            false,
        ) {
            Ok((status, value)) => {
                if intent.method == "POST" {
                    let valid = value
                        .as_ref()
                        .and_then(|v| v["successful"]["0"]["key"].as_str())
                        == Some(intent.key.as_str())
                        && value
                            .as_ref()
                            .and_then(|v| v["failed"].as_object())
                            .is_some_and(|v| v.is_empty());
                    if (status != 200 || !valid)
                        && let Some(code) = value
                            .as_ref()
                            .and_then(|v| v["failed"]["0"]["code"].as_u64())
                            .filter(|code| *code < 500)
                    {
                        return Err(match code {
                            403 => fail(
                                "ZOTERO_WRITE_FORBIDDEN",
                                "Zotero拒绝批注写入，请核对库及对象权限。",
                            ),
                            409 | 412 => {
                                fail("ZOTERO_WRITE_CONFLICT", "Zotero批注版本冲突，未自动覆盖。")
                            }
                            _ => fail(
                                "ZOTERO_WRITE_INVALID",
                                "Zotero逐项结果拒绝此批注，请检查参数。",
                            ),
                        });
                    }
                } else if status != 204 {
                    return Ok(unknown(&intent.key));
                }
            }
            Err(error) if error.code == "ZOTERO_WRITE_INVALID" && intent.method != "POST" => {
                // Zotero10 may answer400 (not404) when an item was deleted after preparation.
                // Only a separate scoped GET404 proves absence; other invalid inputs stay invalid.
                if self
                    .read_annotation_for_sync(
                        &intent.source,
                        original,
                        &intent.key,
                        &AtomicBool::new(false),
                    )
                    .is_ok_and(|v| v.is_none())
                {
                    return Err(fail(
                        "ZOTERO_WRITE_NOT_FOUND",
                        "写入竞争期间目标已删除，已逐项复读确认；本地修改保留。",
                    ));
                }
                return Err(error);
            }
            Err(error) if error.code == "ZOTERO_WRITE_UNKNOWN" => {}
            Err(error) => return Err(error),
        }
        // Once sent, cancellation cannot undo Zotero. Always try an unauthenticated reread.
        Ok(self
            .verify_annotation_write(intent, original, &AtomicBool::new(false))
            .unwrap_or_else(|_| unknown(&intent.key)))
    }
    pub fn verify_annotation_write(
        &self,
        intent: &WriteIntent,
        original: &Path,
        cancel: &AtomicBool,
    ) -> WriteResult<ZoteroWriteOutcome> {
        let start = Instant::now();
        let (health, scope) = self
            .annotation_context(&intent.source, original, cancel, start)
            .map_err(read_failure)?;
        let value = self
            .item(&scope, &intent.key, &health, cancel, start)
            .map_err(read_failure)?;
        match (&intent.expected, value) {
            (None, None) => Ok(ZoteroWriteOutcome {
                verified: true,
                annotation_key: intent.key.clone(),
                annotation: None,
                message: "已从Zotero复读确认批注不存在；原PDF未修改。",
            }),
            (Some(expected), Some(value)) => {
                let actual = parse_annotation(&value, &intent.source).map_err(read_failure)?;
                if actual.version > 0
                    && actual.annotation_type == expected.annotation_type
                    && actual.color == expected.color
                    && actual.text == expected.text
                    && actual.comment == expected.comment
                    && actual.position == expected.position
                {
                    Ok(ZoteroWriteOutcome {
                        verified: true,
                        annotation_key: intent.key.clone(),
                        annotation: Some(actual),
                        message: "已从Zotero重新读取并核实批注、来源、位置、文字及颜色；云端同步仍由Zotero负责。",
                    })
                } else {
                    Ok(unknown(&intent.key))
                }
            }
            _ => Ok(unknown(&intent.key)),
        }
    }
}
fn unknown(key: &str) -> ZoteroWriteOutcome {
    ZoteroWriteOutcome {
        verified: false,
        annotation_key: key.into(),
        annotation: None,
        message: "写入可能已经发生，但复读尚未确认。请复核此键，未自动重发或声称成功。",
    }
}
#[cfg(test)]
#[path = "zotero_write_tests.rs"]
mod tests;
