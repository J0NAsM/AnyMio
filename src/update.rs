use std::time::Duration;

use anyhow::{Context, Result, bail};
use semver::Version;
use serde::Deserialize;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Deserialize)]
struct UpdateManifest {
    version: String,
    url: String,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    pub url: String,
    pub notes: Option<String>,
}

/// Returns an update only when the manifest has a newer valid semantic version.
/// Both the manifest and download endpoints must use HTTPS.
pub async fn check(manifest_url: &str) -> Result<Option<AvailableUpdate>> {
    require_https(manifest_url, "the update manifest URL")?;

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(format!("JRemote/{CURRENT_VERSION}"))
        .build()
        .context("could not create the update client")?;
    let manifest = client
        .get(manifest_url)
        .send()
        .await
        .context("could not contact the update server")?
        .error_for_status()
        .context("the update server returned an error")?
        .json::<UpdateManifest>()
        .await
        .context("the update manifest is invalid")?;

    require_https(&manifest.url, "the update download URL")?;
    let current = Version::parse(CURRENT_VERSION).context("the application version is invalid")?;
    let available = Version::parse(manifest.version.trim_start_matches('v'))
        .context("the update version is invalid")?;

    Ok((available > current).then_some(AvailableUpdate {
        version: available.to_string(),
        url: manifest.url,
        notes: manifest.notes.filter(|notes| !notes.trim().is_empty()),
    }))
}

fn require_https(value: &str, description: &str) -> Result<()> {
    let url = reqwest::Url::parse(value).with_context(|| format!("{description} is malformed"))?;
    if url.scheme() != "https" {
        bail!("{description} must use HTTPS");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_https_urls() {
        assert!(require_https("https://updates.example.com/release.json", "URL").is_ok());
        assert!(require_https("http://updates.example.com/release.json", "URL").is_err());
        assert!(require_https("not a URL", "URL").is_err());
    }

    #[test]
    fn current_version_is_valid_semver() {
        assert!(Version::parse(CURRENT_VERSION).is_ok());
    }
}
