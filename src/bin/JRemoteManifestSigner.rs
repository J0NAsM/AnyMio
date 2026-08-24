use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "JRemoteManifestSigner",
    about = "Signs a JRemote update manifest"
)]
struct Args {
    /// JSON update manifest to sign in place.
    #[arg(long)]
    manifest: PathBuf,
    /// File containing the 32-byte Ed25519 private key as 64 hexadecimal characters.
    #[arg(long)]
    private_key_file: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct UpdateManifest {
    version: String,
    url: String,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let private_key = fs::read_to_string(&args.private_key_file)
        .context("could not read the manifest private key file")?;
    let private_key = decode_hex::<32>(private_key.trim())
        .context("the manifest private key must be 64 hexadecimal characters")?;
    let mut manifest: UpdateManifest = serde_json::from_slice(
        &fs::read(&args.manifest).context("could not read the update manifest")?,
    )
    .context("the update manifest is invalid")?;
    let signing_key = SigningKey::from_bytes(&private_key);
    let payload = manifest_signing_payload(
        &manifest.version,
        &manifest.url,
        manifest.sha256.as_deref(),
        manifest.notes.as_deref(),
    );
    manifest.signature = Some(encode_hex(&signing_key.sign(payload.as_bytes()).to_bytes()));
    fs::write(&args.manifest, serde_json::to_vec_pretty(&manifest)?)
        .context("could not write the signed update manifest")?;
    println!("Signed {}", args.manifest.display());
    Ok(())
}

fn manifest_signing_payload(
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

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signer_payload_is_deterministic() {
        assert_eq!(
            manifest_signing_payload("1.0.0", "https://example.test/app.exe", Some("ab"), None),
            "version:5:1.0.0\nurl:28:https://example.test/app.exe\nsha256:2:ab\nnotes:0:\n"
        );
    }
}
