use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const HISTORY_FILE: &str = "session-history.json";
const HISTORY_TEMP_FILE: &str = "session-history.json.tmp";
const MAX_HISTORY_ENTRIES: usize = 200;

/// Local audit events for the consent lifecycle. They are deliberately not a
/// claim that a desktop session was established.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessAttemptStatus {
    Requested,
    Approved,
    Denied,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AccessAttempt {
    pub consent_id: Uuid,
    pub requester_device_id: String,
    pub status: AccessAttemptStatus,
    pub timestamp_unix: u64,
}

#[derive(Default, Deserialize, Serialize)]
pub struct SessionHistory {
    pub entries: Vec<AccessAttempt>,
}

impl SessionHistory {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(HISTORY_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let mut history: Self = serde_json::from_slice(
            &fs::read(path).context("could not read the access attempt history")?,
        )
        .context("access attempt history is malformed")?;
        history.trim();
        Ok(history)
    }

    pub fn record(
        &mut self,
        consent_id: Uuid,
        requester_device_id: impl Into<String>,
        status: AccessAttemptStatus,
    ) -> Result<()> {
        self.entries.push(AccessAttempt {
            consent_id,
            requester_device_id: requester_device_id.into(),
            status,
            timestamp_unix: now()?,
        });
        self.trim();
        Ok(())
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        let temporary = dir.join(HISTORY_TEMP_FILE);
        fs::write(&temporary, serde_json::to_vec_pretty(self)?)
            .context("could not write the access attempt history")?;
        fs::rename(temporary, dir.join(HISTORY_FILE))
            .context("could not save the access attempt history atomically")
    }

    pub fn recent(&self, limit: usize) -> impl Iterator<Item = &AccessAttempt> {
        self.entries.iter().rev().take(limit)
    }

    fn trim(&mut self) {
        let excess = self.entries.len().saturating_sub(MAX_HISTORY_ENTRIES);
        if excess > 0 {
            self.entries.drain(..excess);
        }
    }
}

pub fn record(
    dir: &Path,
    consent_id: Uuid,
    requester_device_id: impl Into<String>,
    status: AccessAttemptStatus,
) -> Result<()> {
    let mut history = SessionHistory::load(dir)?;
    history.record(consent_id, requester_device_id, status)?;
    history.save(dir)
}

fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_is_persistent_and_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let mut history = SessionHistory::default();
        for _ in 0..=MAX_HISTORY_ENTRIES {
            history
                .record(
                    Uuid::new_v4(),
                    "123 456 789",
                    AccessAttemptStatus::Requested,
                )
                .unwrap();
        }
        history.save(temp.path()).unwrap();
        let loaded = SessionHistory::load(temp.path()).unwrap();
        assert_eq!(loaded.entries.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(loaded.recent(1).count(), 1);
    }
}
