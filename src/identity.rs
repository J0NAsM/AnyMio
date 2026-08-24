use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use ed25519_dalek::{Signer, SigningKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const IDENTITY_FILE: &str = "identity.json";
const IDENTITY_TEMP_FILE: &str = "identity.json.tmp";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub uuid: Uuid,
    /// Random-looking nine-digit locator derived from the device public key.
    /// It locates a registered endpoint and is never an authenticator.
    pub public_id: u32,
    pub created_at_unix: u64,
    pub display_name: Option<String>,
    #[serde(default = "new_signing_key")]
    signing_key: [u8; 32],
}

impl DeviceIdentity {
    pub fn load_or_create(explicit_dir: Option<PathBuf>) -> Result<Self> {
        let dir = application_data_dir(explicit_dir)?;
        fs::create_dir_all(&dir).context("could not create identity directory")?;
        let path = dir.join(IDENTITY_FILE);
        if path.exists() {
            let text = fs::read_to_string(&path).context("could not read identity")?;
            let mut identity: Self =
                serde_json::from_str(&text).context("identity file is malformed")?;
            // Migrate identities created before the signing key was introduced.
            let derived_id = device_id_from_public_key(&identity.public_key());
            if identity.public_id != derived_id {
                identity.public_id = derived_id;
                identity.save_in(&dir)?;
            }
            return Ok(identity);
        }
        Self::create_in(dir)
    }

    #[cfg(test)]
    pub fn reset(explicit_dir: Option<PathBuf>) -> Result<Self> {
        let dir = application_data_dir(explicit_dir)?;
        fs::create_dir_all(&dir).context("could not create identity directory")?;
        Self::create_in(dir)
    }

    pub fn public_id_formatted(&self) -> String {
        let value = format!("{:09}", self.public_id);
        format!("{} {} {}", &value[0..3], &value[3..6], &value[6..9])
    }

    pub fn public_key(&self) -> [u8; 32] {
        SigningKey::from_bytes(&self.signing_key)
            .verifying_key()
            .to_bytes()
    }

    #[allow(dead_code)]
    pub fn sign(&self, payload: &[u8]) -> [u8; 64] {
        SigningKey::from_bytes(&self.signing_key)
            .sign(payload)
            .to_bytes()
    }

    fn create_in(dir: PathBuf) -> Result<Self> {
        let signing_key = new_signing_key();
        let identity = Self {
            uuid: Uuid::new_v4(),
            public_id: device_id_from_public_key(
                &SigningKey::from_bytes(&signing_key)
                    .verifying_key()
                    .to_bytes(),
            ),
            created_at_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .context("system clock precedes Unix epoch")?
                .as_secs(),
            display_name: None,
            signing_key,
        };
        identity.save_in(&dir)?;
        Ok(identity)
    }

    fn save_in(&self, dir: &Path) -> Result<()> {
        let path = dir.join(IDENTITY_FILE);
        let temporary_path = dir.join(IDENTITY_TEMP_FILE);
        let content = serde_json::to_vec_pretty(self)?;
        fs::write(&temporary_path, content).context("could not write temporary identity")?;
        fs::rename(&temporary_path, &path).context("could not atomically save identity")?;
        Ok(())
    }
}

pub fn device_id_from_public_key(public_key: &[u8; 32]) -> u32 {
    let digest = Sha256::digest(public_key);
    let mut first_eight = [0_u8; 8];
    first_eight.copy_from_slice(&digest[..8]);
    (u64::from_be_bytes(first_eight) % 900_000_000) as u32 + 100_000_000
}

pub fn application_data_dir(explicit_dir: Option<PathBuf>) -> Result<PathBuf> {
    explicit_dir.map(Ok).unwrap_or_else(|| {
        ProjectDirs::from("org", "JRemote", "JRemote")
            .context("could not determine a user data directory")
            .map(|dirs| dirs.data_local_dir().to_path_buf())
    })
}

fn new_signing_key() -> [u8; 32] {
    SigningKey::generate(&mut OsRng).to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    #[test]
    fn persistent_identity_round_trips() {
        let temp = tempfile::tempdir().unwrap();
        let first = DeviceIdentity::load_or_create(Some(temp.path().to_owned())).unwrap();
        let second = DeviceIdentity::load_or_create(Some(temp.path().to_owned())).unwrap();
        assert_eq!(first.uuid, second.uuid);
        assert_eq!(first.public_id, second.public_id);
        assert_eq!(first.public_key(), second.public_key());
        assert_eq!(first.public_id_formatted().len(), 11);
    }

    #[test]
    fn reset_replaces_the_key_and_locator() {
        let temp = tempfile::tempdir().unwrap();
        let first = DeviceIdentity::load_or_create(Some(temp.path().to_owned())).unwrap();
        let second = DeviceIdentity::reset(Some(temp.path().to_owned())).unwrap();
        assert_ne!(first.public_key(), second.public_key());
        assert_ne!(first.public_id, second.public_id);
    }

    #[test]
    fn signatures_verify_with_the_persisted_public_key() {
        let temp = tempfile::tempdir().unwrap();
        let identity = DeviceIdentity::load_or_create(Some(temp.path().to_owned())).unwrap();
        let payload = b"JRemote identity test";
        let signature = Signature::from_bytes(&identity.sign(payload));
        VerifyingKey::from_bytes(&identity.public_key())
            .unwrap()
            .verify(payload, &signature)
            .unwrap();
    }
}
