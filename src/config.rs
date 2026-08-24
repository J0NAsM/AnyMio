use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.json";
const CONFIG_TEMP_FILE: &str = "config.json.tmp";
const CONFIG_VERSION: u32 = 1;
const MAX_KNOWN_DEVICES: usize = 500;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    Spanish,
    English,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct KnownDevice {
    pub public_id: String,
    pub display_name: String,
    pub public_key_fingerprint: Option<String>,
    pub last_seen_unix: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppConfig {
    pub version: u32,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub update_channel: UpdateChannel,
    #[serde(default = "default_true")]
    pub check_updates_at_startup: bool,
    #[serde(default)]
    pub relay_url: Option<String>,
    #[serde(default)]
    pub known_devices: Vec<KnownDevice>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            language: Language::default(),
            update_channel: UpdateChannel::default(),
            check_updates_at_startup: true,
            relay_url: None,
            known_devices: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn load_or_create(dir: &Path) -> Result<Self> {
        fs::create_dir_all(dir).context("could not create the configuration directory")?;
        let path = dir.join(CONFIG_FILE);
        if !path.exists() {
            let config = Self::default();
            config.save(dir)?;
            return Ok(config);
        }
        let config: Self =
            serde_json::from_slice(&fs::read(&path).context("could not read config")?)
                .context("config file is malformed")?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        self.validate()?;
        let temporary = dir.join(CONFIG_TEMP_FILE);
        fs::write(&temporary, serde_json::to_vec_pretty(self)?)
            .context("could not write temporary config")?;
        fs::rename(temporary, dir.join(CONFIG_FILE)).context("could not save config atomically")
    }

    pub fn add_or_update_device(&mut self, device: KnownDevice) -> Result<()> {
        if device.public_id.trim().is_empty() || device.display_name.trim().is_empty() {
            bail!("a known device requires an ID and display name");
        }
        if let Some(existing) = self
            .known_devices
            .iter_mut()
            .find(|known| known.public_id == device.public_id)
        {
            *existing = device;
        } else {
            self.known_devices.push(device);
        }
        self.validate()
    }

    fn validate(&self) -> Result<()> {
        if self.version > CONFIG_VERSION {
            bail!("the configuration was created by a newer JRemote version");
        }
        if self.known_devices.len() > MAX_KNOWN_DEVICES {
            bail!("too many known devices in the configuration");
        }
        if let Some(relay_url) = &self.relay_url {
            let parsed = reqwest::Url::parse(relay_url).context("relay URL is malformed")?;
            if !matches!(parsed.scheme(), "https" | "wss") {
                bail!("relay URL must use HTTPS or WSS");
            }
        }
        Ok(())
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_and_replaces_devices_by_id() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = AppConfig::load_or_create(temp.path()).unwrap();
        config
            .add_or_update_device(KnownDevice {
                public_id: "123 456 789".into(),
                display_name: "Oficina".into(),
                public_key_fingerprint: None,
                last_seen_unix: None,
            })
            .unwrap();
        config
            .add_or_update_device(KnownDevice {
                public_id: "123 456 789".into(),
                display_name: "Oficina central".into(),
                public_key_fingerprint: None,
                last_seen_unix: Some(1),
            })
            .unwrap();
        config.save(temp.path()).unwrap();
        let loaded = AppConfig::load_or_create(temp.path()).unwrap();
        assert_eq!(loaded.known_devices.len(), 1);
        assert_eq!(loaded.known_devices[0].display_name, "Oficina central");
    }

    #[test]
    fn rejects_an_insecure_relay_url() {
        let config = AppConfig {
            relay_url: Some("http://relay.example".into()),
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }
}
