mod config;
mod consent;
mod diagnostics;
mod events;
mod identity;
mod protocol;
// Policy used by the future E2E session transport; it does not create retries
// or remote access in the current signalling-only build.
#[allow(dead_code)]
mod reconnect;
mod relay;
// Used by the future unattended-access flow; keep it compiled and covered by unit tests.
#[allow(dead_code)]
mod security;
mod session_history;
mod ui;
mod update;
mod update_cache;

use std::{
    env,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use identity::DeviceIdentity;
use relay::Relay;

#[derive(Debug, Parser)]
#[command(
    name = "JRemote",
    version,
    about = "Secure personal remote-desktop endpoint and relay"
)]
struct Args {
    /// Run the in-memory signalling relay. The relay never captures or injects input.
    #[arg(long)]
    relay: bool,
    /// Relay TCP port (only used with --relay).
    #[arg(long, default_value_t = 4433)]
    port: u16,
    /// Store the generated local identity in this explicit directory.
    #[arg(long, hide = true)]
    data_dir: Option<PathBuf>,
    /// Keep identity, settings and logs beside the executable (for the portable edition).
    #[arg(long)]
    portable: bool,
    /// HTTPS URL of the release manifest. It overrides the environment setting.
    #[arg(long, value_name = "URL")]
    update_manifest_url: Option<String>,
    /// Download, verify and install an available update, then restart JRemote.
    #[arg(long)]
    install_update: bool,
    /// Restore the previous verified executable after an update.
    #[arg(long)]
    rollback_update: bool,
    /// Open the visible local AnyMio interface.
    #[arg(long)]
    ui: bool,
    /// Check update hosting and relay configuration without changing anything.
    #[arg(long)]
    diagnostics: bool,
    /// Create a local consent request for a future remote session.
    #[arg(long, value_name = "DEVICE_ID")]
    request_consent: Option<String>,
    /// Approve a displayed local consent request.
    #[arg(long, value_name = "REQUEST_ID")]
    approve_consent: Option<uuid::Uuid>,
    /// Deny a displayed local consent request.
    #[arg(long, value_name = "REQUEST_ID")]
    deny_consent: Option<uuid::Uuid>,
    /// List local consent requests and their expiry status.
    #[arg(long)]
    list_consents: bool,
    /// Show the newest local activity records (1 to 200 entries).
    #[arg(long, value_name = "LIMIT")]
    show_events: Option<usize>,
    /// Show the consent lifecycle audit history (1 to 200 entries).
    #[arg(long, value_name = "LIMIT")]
    show_access_history: Option<usize>,
    /// Export the non-secret local configuration to a new JSON file.
    #[arg(long, value_name = "PATH")]
    export_config: Option<PathBuf>,
    /// Validate and replace the local non-secret configuration from JSON.
    #[arg(long, value_name = "PATH")]
    import_config: Option<PathBuf>,
    /// Forget one known device by its public ID.
    #[arg(long, value_name = "DEVICE_ID")]
    forget_device: Option<String>,
}

// The publisher can set this at build time. A command-line URL or runtime
// environment variable is useful for testing and takes precedence.
const BUILT_IN_UPDATE_MANIFEST_URL: Option<&str> = option_env!("JREMOTE_UPDATE_MANIFEST_URL");
const DEFAULT_UPDATE_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/J0NAsM/AnyMio/main/update.json";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();
    let args = Args::parse();

    if args.relay {
        let address = SocketAddr::from(([0, 0, 0, 0], args.port));
        println!("JRemote relay listening on {address}");
        println!(
            "This relay only routes bounded signalling messages; it does not inspect desktop data."
        );
        Relay::bind(address).await?.serve().await
    } else {
        let data_dir = selected_data_dir(&args)?;
        let mut config = config::AppConfig::load_or_create(&data_dir)?;
        let _ = events::append(
            &data_dir,
            "application_started",
            "JRemote was opened locally",
        );
        if let Some(path) = args.export_config.as_deref() {
            config.export_to(path)?;
            let _ = events::append(&data_dir, "config_exported", &path.display().to_string());
            println!("Configuración exportada a {}", path.display());
            return Ok(());
        }
        if let Some(path) = args.import_config.as_deref() {
            config = config::AppConfig::import_from(path)?;
            config.save(&data_dir)?;
            let _ = events::append(&data_dir, "config_imported", &path.display().to_string());
            println!("Configuración importada desde {}", path.display());
            return Ok(());
        }
        if let Some(device_id) = args.forget_device.as_deref() {
            config.remove_device(device_id)?;
            config.save(&data_dir)?;
            let _ = events::append(&data_dir, "known_device_forgotten", device_id);
            println!("Dispositivo eliminado: {device_id}");
            return Ok(());
        }
        if let Some(device_id) = args.request_consent.as_deref() {
            let mut store = consent::ConsentStore::load(&data_dir)?;
            let request_id = store.request(device_id.to_owned())?;
            store.save(&data_dir)?;
            session_history::record(
                &data_dir,
                request_id,
                device_id,
                session_history::AccessAttemptStatus::Requested,
            )?;
            let _ = events::append(&data_dir, "consent_requested", &request_id.to_string());
            println!("Solicitud local creada: {request_id}");
            return Ok(());
        }
        if let Some(request_id) = args.approve_consent.or(args.deny_consent) {
            let approved = args.approve_consent.is_some();
            let mut store = consent::ConsentStore::load(&data_dir)?;
            let request = store.resolve(request_id, approved)?;
            store.save(&data_dir)?;
            let status = if approved { "approved" } else { "denied" };
            session_history::record(
                &data_dir,
                request.id,
                request.requester_device_id,
                if approved {
                    session_history::AccessAttemptStatus::Approved
                } else {
                    session_history::AccessAttemptStatus::Denied
                },
            )?;
            let _ = events::append(
                &data_dir,
                "consent_resolved",
                &format!("{request_id}: {status}"),
            );
            println!("Solicitud {request_id}: {status}");
            return Ok(());
        }
        if args.list_consents {
            for request in consent::ConsentStore::load(&data_dir)?.requests {
                println!(
                    "{} | {} | {:?} | vence {}",
                    request.id,
                    request.requester_device_id,
                    request.status,
                    request.expires_at_unix
                );
            }
            return Ok(());
        }
        if let Some(limit) = args.show_events {
            for event in events::recent(&data_dir, limit.clamp(1, 200))? {
                println!(
                    "{} | {} | {}",
                    event.timestamp_unix, event.kind, event.detail
                );
            }
            return Ok(());
        }
        if let Some(limit) = args.show_access_history {
            for attempt in
                session_history::SessionHistory::load(&data_dir)?.recent(limit.clamp(1, 200))
            {
                println!(
                    "{} | {} | {:?} | {}",
                    attempt.timestamp_unix,
                    attempt.requester_device_id,
                    attempt.status,
                    attempt.consent_id
                );
            }
            return Ok(());
        }
        if args.diagnostics {
            for result in diagnostics::run(
                &configured_update_manifest_url(&args, &config.update_channel),
                config.relay_url.as_deref(),
            )
            .await
            {
                let state = if result.ok { "OK" } else { "ERROR" };
                println!("[{state}] {}: {}", result.name, result.detail);
                let _ = events::append(
                    &data_dir,
                    "diagnostic",
                    &format!("{}: {}", result.name, result.detail),
                );
            }
            return Ok(());
        }
        if args.ui {
            let identity = DeviceIdentity::load_or_create(Some(data_dir.clone()))
                .context("could not load the local device identity")?;
            let manifest_url = configured_update_manifest_url(&args, &config.update_channel);
            return ui::run(identity, config, data_dir, manifest_url);
        }
        let check_at_startup = config.check_updates_at_startup
            && !update_cache::checked_current_recently(&data_dir).unwrap_or(false);
        let available_update = if check_at_startup || args.install_update {
            let result = update::check(&configured_update_manifest_url(
                &args,
                &config.update_channel,
            ))
            .await;
            if result.as_ref().is_ok_and(Option::is_none) {
                let _ = update_cache::mark_current(&data_dir);
            }
            result
        } else {
            Ok(None)
        };
        if args.install_update {
            return install_update(available_update?);
        }
        if args.rollback_update {
            return rollback_update();
        }

        let identity = DeviceIdentity::load_or_create(Some(data_dir.clone()))
            .context("could not load the local device identity")?;
        println!("JRemote {}", update::CURRENT_VERSION);
        println!("This device ID: {}", identity.public_id_formatted());
        println!("Known devices: {}", config.known_devices.len());
        println!("No remote session is active.");
        println!("The GUI/capture endpoint is not yet included in this build.");
        match available_update {
            Ok(Some(release)) => {
                let _ = events::append(&data_dir, "update_available", &release.version);
                show_update_message(&release)
            }
            Ok(None) => {}
            Err(error) => {
                let _ = events::append(&data_dir, "update_check_failed", &error.to_string());
                tracing::debug!(%error, "update check failed")
            }
        }
        Ok(())
    }
}

fn selected_data_dir(args: &Args) -> Result<PathBuf> {
    if let Some(dir) = args.data_dir.clone() {
        return Ok(dir);
    }
    let executable = env::current_exe().context("could not locate the executable")?;
    let is_portable_package = executable
        .parent()
        .is_some_and(|directory| directory.join("portable.json").is_file());
    if args.portable || is_portable_package {
        return portable_data_dir(&executable);
    }
    identity::application_data_dir(None)
}

fn portable_data_dir(executable: &Path) -> Result<PathBuf> {
    executable
        .parent()
        .context("portable executable has no parent directory")
        .map(|directory| directory.join("data"))
}

fn configured_update_manifest_url(args: &Args, channel: &config::UpdateChannel) -> String {
    args.update_manifest_url
        .clone()
        .or_else(|| {
            env::var("JREMOTE_UPDATE_MANIFEST_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| BUILT_IN_UPDATE_MANIFEST_URL.map(str::to_owned))
        .unwrap_or_else(|| match channel {
            config::UpdateChannel::Stable => DEFAULT_UPDATE_MANIFEST_URL.to_owned(),
            config::UpdateChannel::Beta => {
                DEFAULT_UPDATE_MANIFEST_URL.replace("update.json", "update-beta.json")
            }
        })
}

fn show_update_message(release: &update::AvailableUpdate) {
    println!();
    println!("\u{00a1}Hay una actualizaci\u{00f3}n de JRemote disponible!");
    println!("Versi\u{00f3}n actual: {}", update::CURRENT_VERSION);
    println!("Nueva versi\u{00f3}n: {}", release.version);
    println!("Desc\u{00e1}rgala aqu\u{00ed}: {}", release.url);
    if let Some(notes) = &release.notes {
        println!("Cambios: {notes}");
    }
    println!("Para instalarla: JRemote.exe --install-update");
}

fn install_update(release: Option<update::AvailableUpdate>) -> Result<()> {
    let Some(release) = release else {
        println!("JRemote ya est\u{00e1} actualizado.");
        return Ok(());
    };
    start_update(&release)?;
    println!(
        "La actualizaci\u{00f3}n se descargar\u{00e1} y verificar\u{00e1} al cerrar esta ventana."
    );
    Ok(())
}

pub(crate) fn start_update(release: &update::AvailableUpdate) -> Result<()> {
    let executable =
        env::current_exe().context("could not determine the JRemote executable path")?;
    let updater = executable
        .parent()
        .context("the JRemote executable has no parent directory")?
        .join("JRemoteUpdater.exe");
    if !updater.is_file() {
        bail!("JRemoteUpdater.exe is missing; reinstall JRemote from its official release");
    }
    Command::new(updater)
        .arg("--target")
        .arg(&executable)
        .arg("--url")
        .arg(&release.url)
        .arg("--sha256")
        .arg(&release.sha256)
        .spawn()
        .context("could not start the update helper")?;
    Ok(())
}

fn rollback_update() -> Result<()> {
    let executable =
        env::current_exe().context("could not determine the JRemote executable path")?;
    let updater = executable
        .parent()
        .context("the JRemote executable has no parent directory")?
        .join("JRemoteUpdater.exe");
    if !updater.is_file() {
        bail!("JRemoteUpdater.exe is missing; reinstall JRemote from its official release");
    }
    Command::new(updater)
        .arg("--target")
        .arg(executable)
        .arg("--rollback")
        .spawn()
        .context("could not start the rollback helper")?;
    println!("La versión anterior se restaurará al cerrar esta ventana.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_manifest_url_has_priority() {
        let args = Args {
            relay: false,
            port: 4433,
            data_dir: None,
            portable: false,
            update_manifest_url: Some("https://example.com/update.json".into()),
            install_update: false,
            rollback_update: false,
            ui: false,
            diagnostics: false,
            request_consent: None,
            approve_consent: None,
            deny_consent: None,
            list_consents: false,
            show_events: None,
            show_access_history: None,
            export_config: None,
            import_config: None,
            forget_device: None,
        };
        assert_eq!(
            configured_update_manifest_url(&args, &config::UpdateChannel::Stable),
            "https://example.com/update.json"
        );
    }

    #[test]
    fn beta_channel_uses_its_own_default_manifest() {
        let args = Args {
            relay: false,
            port: 4433,
            data_dir: None,
            portable: false,
            update_manifest_url: None,
            install_update: false,
            rollback_update: false,
            ui: false,
            diagnostics: false,
            request_consent: None,
            approve_consent: None,
            deny_consent: None,
            list_consents: false,
            show_events: None,
            show_access_history: None,
            export_config: None,
            import_config: None,
            forget_device: None,
        };
        assert!(
            configured_update_manifest_url(&args, &config::UpdateChannel::Beta)
                .ends_with("update-beta.json")
        );
    }

    #[test]
    fn portable_data_is_kept_next_to_the_executable() {
        let executable = Path::new("C:/Portable/AnyMio/JRemote.exe");
        assert_eq!(
            portable_data_dir(executable).unwrap(),
            PathBuf::from("C:/Portable/AnyMio/data")
        );
    }
}
