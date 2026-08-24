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
    sha256: Option<String>,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    pub url: String,
    /// SHA-256 of the released executable, required before automatic installation.
    pub sha256: String,
    pub notes: Option<String>,
}

/// Returns an update only when the manifest has a newer valid semantic version.
/// The manifest and download endpoint must use HTTPS. New releases must include a
/// SHA-256 checksum so the updater can verify the downloaded executable.
pub async fn check(manifest_url: &str) -> Result<Option<AvailableUpdate>> {
    require_https(manifest_url, "the update manifest URL")?;

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(format!("JRemote/{CURRENT_VERSION}"))
        .build()
        .context("could not create the update client")?;
    let manifest_json = client
        .get(manifest_url)
        .send()
        .await
        .context("could not contact the update server")?
        .error_for_status()
        .context("the update server returned an error")?
        .text()
        .await
        .context("could not read the update manifest")?;

    parse_manifest(&manifest_json)
}

fn parse_manifest(manifest_json: &str) -> Result<Option<AvailableUpdate>> {
    let manifest: UpdateManifest =
        serde_json::from_str(manifest_json).context("the update manifest is invalid")?;
    let current = Version::parse(CURRENT_VERSION).context("the application version is invalid")?;
    let available = Version::parse(manifest.version.trim_start_matches('v'))
        .context("the update version is invalid")?;

    if available <= current {
        return Ok(None);
    }

    require_https(&manifest.url, "the update download URL")?;
    let sha256 = manifest
        .sha256
        .context("the update manifest has no SHA-256 checksum")?
        .to_ascii_lowercase();
    validate_sha256(&sha256)?;

    Ok(Some(AvailableUpdate {
        version: available.to_string(),
        url: manifest.url,
        sha256,
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

fn validate_sha256(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("the update SHA-256 checksum must contain exactly 64 hexadecimal characters");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

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

    #[test]
    fn accepts_a_newer_hashed_release() {
        let update = parse_manifest(&format!(
            r#"{{"version":"999.0.0","url":"https://example.com/JRemote.exe","sha256":"{HASH}"}}"#
        ))
        .unwrap()
        .unwrap();
        assert_eq!(update.version, "999.0.0");
        assert_eq!(update.sha256, HASH);
    }

    #[test]
    fn rejects_a_newer_release_without_a_checksum() {
        let error =
            parse_manifest(r#"{"version":"999.0.0","url":"https://example.com/JRemote.exe"}"#)
                .unwrap_err();
        assert!(error.to_string().contains("SHA-256"));
    }

    #[test]
    fn ignores_an_old_release_without_a_checksum() {
        assert!(
            parse_manifest(r#"{"version":"0.0.1","url":"https://example.com/JRemote.exe"}"#)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rejects_an_invalid_checksum() {
        let error = parse_manifest(
            r#"{"version":"999.0.0","url":"https://example.com/JRemote.exe","sha256":"bad"}"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("64 hexadecimal"));
    }
}
