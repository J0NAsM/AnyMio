use std::time::Duration;

use anyhow::{Context, Result, bail};
use ed25519_dalek::{Signature, VerifyingKey};
use semver::Version;
use serde::Deserialize;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Optional 32-byte Ed25519 public key, encoded as 64 hexadecimal characters.
/// Publishers set this at build time with JREMOTE_UPDATE_MANIFEST_PUBLIC_KEY.
pub const MANIFEST_PUBLIC_KEY: Option<&str> = option_env!("JREMOTE_UPDATE_MANIFEST_PUBLIC_KEY");
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Deserialize)]
struct UpdateManifest {
    version: String,
    url: String,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    signature: Option<String>,
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
    parse_manifest_with_public_key(manifest_json, MANIFEST_PUBLIC_KEY)
}

fn parse_manifest_with_public_key(
    manifest_json: &str,
    public_key: Option<&str>,
) -> Result<Option<AvailableUpdate>> {
    let manifest: UpdateManifest =
        serde_json::from_str(manifest_json).context("the update manifest is invalid")?;
    verify_manifest_signature(&manifest, public_key)?;
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

/// Deterministic bytes signed by the manifest publisher. Length prefixes avoid
/// ambiguity if a value contains a newline or separator.
pub fn manifest_signing_payload(
    version: &str,
    url: &str,
    sha256: Option<&str>,
    notes: Option<&str>,
) -> String {
    let sha256 = sha256.unwrap_or_default();
    let notes = notes.unwrap_or_default();
    format!(
        "version:{}:{version}\nurl:{}:{url}\nsha256:{}:{sha256}\nnotes:{}:{notes}\n",
        version.len(),
        url.len(),
        sha256.len(),
        notes.len()
    )
}

fn verify_manifest_signature(manifest: &UpdateManifest, public_key: Option<&str>) -> Result<()> {
    let Some(public_key) = public_key.filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };
    let public_key =
        decode_hex::<32>(public_key).context("the configured manifest public key is invalid")?;
    let signature = manifest
        .signature
        .as_deref()
        .context("the update manifest is missing its Ed25519 signature")?;
    let signature = decode_hex::<64>(signature).context("the manifest signature is invalid")?;
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .context("the configured manifest public key is invalid")?;
    verifying_key
        .verify_strict(
            manifest_signing_payload(
                &manifest.version,
                &manifest.url,
                manifest.sha256.as_deref(),
                manifest.notes.as_deref(),
            )
            .as_bytes(),
            &Signature::from_bytes(&signature),
        )
        .context("the update manifest signature does not verify")
}

fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N]> {
    if value.len() != N * 2 {
        bail!("expected {} hexadecimal characters", N * 2);
    }
    let mut bytes = [0_u8; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16)
            .context("contains non-hexadecimal characters")?;
    }
    Ok(bytes)
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
    use ed25519_dalek::{Signer, SigningKey};

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

    #[test]
    fn configured_public_key_requires_a_valid_signature() {
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let public_key = signing_key.verifying_key();
        let payload = manifest_signing_payload(
            "999.0.0",
            "https://example.com/JRemote.exe",
            Some(HASH),
            Some("Signed release"),
        );
        let signature = signing_key.sign(payload.as_bytes());
        let manifest = format!(
            r#"{{"version":"999.0.0","url":"https://example.com/JRemote.exe","sha256":"{HASH}","notes":"Signed release","signature":"{}"}}"#,
            encode_hex(&signature.to_bytes())
        );
        assert!(
            parse_manifest_with_public_key(&manifest, Some(&encode_hex(public_key.as_bytes())))
                .unwrap()
                .is_some()
        );
        assert!(parse_manifest_with_public_key(
            r#"{"version":"999.0.0","url":"https://example.com/JRemote.exe","sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}"#,
            Some(&encode_hex(public_key.as_bytes()))
        )
        .is_err());
    }

    #[test]
    fn signing_payload_is_stable_for_external_signers() {
        assert_eq!(
            manifest_signing_payload("1.0.0", "https://example.test/app.exe", Some("ab"), None),
            "version:5:1.0.0\nurl:28:https://example.test/app.exe\nsha256:2:ab\nnotes:0:\n"
        );
    }

    fn encode_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
