use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const CONSENT_FILE: &str = "consents.json";
const CONSENT_TEMP_FILE: &str = "consents.json.tmp";
const CONSENT_TTL_SECONDS: u64 = 60;
const MAX_REQUESTS: usize = 200;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsentStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConsentRequest {
    pub id: Uuid,
    pub requester_device_id: String,
    pub created_at_unix: u64,
    pub expires_at_unix: u64,
    pub status: ConsentStatus,
}

#[derive(Default, Deserialize, Serialize)]
pub struct ConsentStore {
    pub requests: Vec<ConsentRequest>,
}

impl ConsentStore {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(CONSENT_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let mut store: Self =
            serde_json::from_slice(&fs::read(path).context("could not read consent records")?)
                .context("consent records are malformed")?;
        store.expire_old(now()?);
        Ok(store)
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        let temporary = dir.join(CONSENT_TEMP_FILE);
        fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        fs::rename(temporary, dir.join(CONSENT_FILE))
            .context("could not save consent records atomically")
    }

    pub fn request(&mut self, requester_device_id: String) -> Result<Uuid> {
        if requester_device_id.trim().is_empty() {
            bail!("a consent request requires a requester device ID");
        }
        self.expire_old(now()?);
        if self.requests.len() >= MAX_REQUESTS {
            self.requests.remove(0);
        }
        let created = now()?;
        let id = Uuid::new_v4();
        self.requests.push(ConsentRequest {
            id,
            requester_device_id,
            created_at_unix: created,
            expires_at_unix: created + CONSENT_TTL_SECONDS,
            status: ConsentStatus::Pending,
        });
        Ok(id)
    }

    pub fn resolve(&mut self, id: Uuid, approved: bool) -> Result<()> {
        self.expire_old(now()?);
        let request = self
            .requests
            .iter_mut()
            .find(|request| request.id == id)
            .context("unknown consent request")?;
        if request.status != ConsentStatus::Pending {
            bail!("the consent request is no longer pending");
        }
        request.status = if approved {
            ConsentStatus::Approved
        } else {
            ConsentStatus::Denied
        };
        Ok(())
    }

    fn expire_old(&mut self, timestamp: u64) {
        for request in &mut self.requests {
            if request.status == ConsentStatus::Pending && request.expires_at_unix <= timestamp {
                request.status = ConsentStatus::Expired;
            }
        }
    }
}

fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approval_is_explicit_and_persistent() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = ConsentStore::load(temp.path()).unwrap();
        let id = store.request("123 456 789".into()).unwrap();
        store.resolve(id, true).unwrap();
        store.save(temp.path()).unwrap();
        assert_eq!(
            ConsentStore::load(temp.path()).unwrap().requests[0].status,
            ConsentStatus::Approved
        );
    }
}
