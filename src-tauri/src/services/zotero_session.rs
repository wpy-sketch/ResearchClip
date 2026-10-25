use super::{
    pdf_open::ephemeral,
    zotero_local::{ZoteroCandidate, ZoteroHealth, ZoteroLookup},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, String>;
struct Job {
    owner: String,
    cancel: Arc<AtomicBool>,
}
struct Preview {
    owner: String,
    document_id: String,
    content_hash: String,
    candidate: ZoteroCandidate,
    created: Instant,
}
#[derive(Default)]
pub struct ZoteroSessions {
    jobs: Mutex<HashMap<String, Job>>,
    previews: Mutex<HashMap<String, Preview>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewOffer {
    pub id: String,
    pub candidate: ZoteroCandidate,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResponse {
    pub health: ZoteroHealth,
    pub candidates: Vec<PreviewOffer>,
    pub message: String,
}
impl ZoteroSessions {
    pub fn begin(&self, id: &str, owner: &str) -> Result<Arc<AtomicBool>> {
        if id.len() != 32
            || !id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("查询标识无效。".into());
        }
        let mut jobs = self.jobs.lock().map_err(|_| "Zotero查询锁异常。")?;
        if jobs.len() >= 3 || jobs.contains_key(id) || jobs.values().any(|job| job.owner == owner) {
            return Err("同一PDF只能进行一项来源查询，请取消或等待当前查询。".into());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        jobs.insert(
            id.into(),
            Job {
                owner: owner.into(),
                cancel: cancel.clone(),
            },
        );
        Ok(cancel)
    }
    pub fn end(&self, id: &str) -> Result<()> {
        self.jobs
            .lock()
            .map_err(|_| "Zotero查询锁异常。")?
            .remove(id);
        Ok(())
    }
    pub fn cancel(&self, id: &str) -> Result<()> {
        if let Some(job) = self.jobs.lock().map_err(|_| "Zotero查询锁异常。")?.get(id) {
            job.cancel.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
    pub fn cancel_pdf(&self, owner: &str) -> Result<()> {
        for job in self.jobs.lock().map_err(|_| "Zotero查询锁异常。")?.values() {
            if job.owner == owner {
                job.cancel.store(true, Ordering::Relaxed);
            }
        }
        self.previews
            .lock()
            .map_err(|_| "来源预览锁异常。")?
            .retain(|_, p| p.owner != owner);
        Ok(())
    }
    pub fn store(
        &self,
        owner: &str,
        document_id: &str,
        hash: &str,
        result: ZoteroLookup,
        cancel: &AtomicBool,
    ) -> Result<PreviewResponse> {
        if cancel.load(Ordering::Relaxed) {
            return Err("Zotero查询已取消。".into());
        }
        let mut previews = self.previews.lock().map_err(|_| "来源预览锁异常。")?;
        previews.retain(|_, p| p.created.elapsed() < Duration::from_secs(300) && p.owner != owner);
        if previews.len() + result.candidates.len() > 24 {
            return Err("来源预览超过保护预算，请关闭其他来源面板后重试。".into());
        }
        let candidates = result
            .candidates
            .into_iter()
            .map(|candidate| {
                let id = ephemeral();
                previews.insert(
                    id.clone(),
                    Preview {
                        owner: owner.into(),
                        document_id: document_id.into(),
                        content_hash: hash.into(),
                        candidate: candidate.clone(),
                        created: Instant::now(),
                    },
                );
                PreviewOffer { id, candidate }
            })
            .collect();
        Ok(PreviewResponse {
            health: result.health,
            candidates,
            message: result.message,
        })
    }
    pub fn take(
        &self,
        id: &str,
        owner: &str,
        document_id: &str,
        hash: &str,
    ) -> Result<ZoteroCandidate> {
        let mut previews = self.previews.lock().map_err(|_| "来源预览锁异常。")?;
        let p = previews
            .get(id)
            .ok_or("来源预览已经过期或已使用，请重新查询。")?;
        if p.owner != owner || p.document_id != document_id || p.content_hash != hash {
            return Err("来源预览不属于当前PDF。".into());
        }
        if p.created.elapsed() >= Duration::from_secs(300) {
            previews.remove(id);
            return Err("来源预览已经过期，请重新查询。".into());
        }
        Ok(previews.remove(id).ok_or("来源预览已使用。")?.candidate)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_is_scoped_and_lookup_budget_is_enforced() {
        let s = ZoteroSessions::default();
        let cancel = s.begin(&"a".repeat(32), "pdfA").unwrap();
        assert!(s.begin(&"b".repeat(32), "pdfA").is_err());
        s.cancel(&"a".repeat(32)).unwrap();
        assert!(cancel.load(Ordering::Relaxed));
        s.end(&"a".repeat(32)).unwrap();
        let b = s.begin(&"b".repeat(32), "pdfB").unwrap();
        s.cancel_pdf("pdfA").unwrap();
        assert!(!b.load(Ordering::Relaxed));
        s.cancel_pdf("pdfB").unwrap();
        assert!(b.load(Ordering::Relaxed));
        assert!(s.begin("../../x", "pdfC").is_err());
    }
    #[test]
    fn preview_is_owner_bound_expires_and_cannot_be_replayed() {
        use crate::services::zotero_local::{ZoteroScope, instance_identity, metadata_candidate};
        use serde_json::json;
        let h = ZoteroHealth {
            api_version: 3,
            schema_version: Some(44),
            product_version: None,
            instance_id: instance_identity("FixtureServer420").unwrap(),
            server_id: "FixtureServer420".into(),
        };
        let scope = ZoteroScope {
            library_type: "user".into(),
            library_id: 0,
        };
        let a = json!({"key":"ATTACH01","version":1,"library":{"type":"user","id":0},"data":{"key":"ATTACH01","itemType":"attachment","parentItem":"PARENT01"}});
        let p = json!({"key":"PARENT01","version":1,"library":{"type":"user","id":0},"data":{"key":"PARENT01","itemType":"journalArticle","title":"Preview fixture"}});
        let c = metadata_candidate(&h, &scope, &a, &p).unwrap();
        let s = ZoteroSessions::default();
        let result = ZoteroLookup {
            health: h,
            candidates: vec![c.clone()],
            message: String::new(),
        };
        let response = s
            .store("owner", "document", "hash", result, &AtomicBool::new(false))
            .unwrap();
        let id = &response.candidates[0].id;
        assert!(s.take(id, "other", "document", "hash").is_err());
        assert!(s.take(id, "owner", "wrong", "hash").is_err());
        assert!(s.take(id, "owner", "document", "wrong").is_err());
        s.take(id, "owner", "document", "hash").unwrap();
        assert!(s.take(id, "owner", "document", "hash").is_err());
        s.previews.lock().unwrap().insert(
            "expired".into(),
            Preview {
                owner: "owner".into(),
                document_id: "document".into(),
                content_hash: "hash".into(),
                candidate: c,
                created: Instant::now() - Duration::from_secs(301),
            },
        );
        assert!(
            s.take("expired", "owner", "document", "hash")
                .unwrap_err()
                .contains("过期")
        );
        assert!(s.previews.lock().unwrap().is_empty());
    }
}
