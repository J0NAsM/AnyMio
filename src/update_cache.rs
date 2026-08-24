use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CACHE_FILE: &str = "update-check.json";
const CACHE_TEMP_FILE: &str = "update-check.json.tmp";
const CHECK_INTERVAL_SECONDS: u64 = 6 * 60 * 60;

#[derive(Deserialize, Serialize)]
struct UpdateCheckCache {
    checked_current_at_unix: u64,
}

/// Only caches a successful "already current" result. A newer available release
/// is never hidden by the cache, and manual checks always bypass this module.
pub fn checked_current_recently(dir: &Path) -> Result<bool> {
    let path = dir.join(CACHE_FILE);
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error).context("could not read the update check cache"),
    };
    let Ok(cache) = serde_json::from_slice::<UpdateCheckCache>(&bytes) else {
        return Ok(false);
    };
    Ok(is_recent(cache.checked_current_at_unix, now()?))
}

pub fn mark_current(dir: &Path) -> Result<()> {
    let temporary = dir.join(CACHE_TEMP_FILE);
    let cache = UpdateCheckCache {
        checked_current_at_unix: now()?,
    };
    fs::write(&temporary, serde_json::to_vec(&cache)?)
        .context("could not write the update check cache")?;
    fs::rename(temporary, dir.join(CACHE_FILE))
        .context("could not save the update check cache atomically")
}

fn is_recent(checked_at: u64, timestamp: u64) -> bool {
    timestamp.saturating_sub(checked_at) < CHECK_INTERVAL_SECONDS
}

fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_expires_after_six_hours() {
        assert!(is_recent(100, 100 + CHECK_INTERVAL_SECONDS - 1));
        assert!(!is_recent(100, 100 + CHECK_INTERVAL_SECONDS));
    }

    #[test]
    fn malformed_cache_never_suppresses_a_check() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join(CACHE_FILE), "not json").unwrap();
        assert!(!checked_current_recently(temp.path()).unwrap());
    }
}
