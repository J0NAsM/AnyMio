mod config;
mod identity;
mod protocol;
mod relay;
// Used by the future unattended-access flow; keep it compiled and covered by unit tests.
#[allow(dead_code)]
mod security;
mod ui;
mod update;

use std::{env, net::SocketAddr, path::PathBuf, process::Command};

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
    /// HTTPS URL of the release manifest. It overrides the environment setting.
    #[arg(long, value_name = "URL")]
    update_manifest_url: Option<String>,
    /// Download, verify and install an available update, then restart JRemote.
    #[arg(long)]
    install_update: bool,
    /// Open the visible local AnyMio interface.
    #[arg(long)]
    ui: bool,
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
        let data_dir = identity::application_data_dir(args.data_dir.clone())?;
        let config = config::AppConfig::load_or_create(&data_dir)?;
        if args.ui {
            let identity = DeviceIdentity::load_or_create(args.data_dir.clone())
                .context("could not load the local device identity")?;
            return ui::run(identity, config);
        }
        let available_update = if config.check_updates_at_startup || args.install_update {
            update::check(&configured_update_manifest_url(&args)).await
        } else {
            Ok(None)
        };
        if args.install_update {
            return install_update(available_update?);
        }

        let identity = DeviceIdentity::load_or_create(args.data_dir.clone())
            .context("could not load the local device identity")?;
        println!("JRemote {}", update::CURRENT_VERSION);
        println!("This device ID: {}", identity.public_id_formatted());
        println!("Known devices: {}", config.known_devices.len());
        println!("No remote session is active.");
        println!("The GUI/capture endpoint is not yet included in this build.");
        match available_update {
            Ok(Some(release)) => show_update_message(&release),
            Ok(None) => {}
            Err(error) => tracing::debug!(%error, "update check failed"),
        }
        Ok(())
    }
}

fn configured_update_manifest_url(args: &Args) -> String {
    args.update_manifest_url
        .clone()
        .or_else(|| {
            env::var("JREMOTE_UPDATE_MANIFEST_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| BUILT_IN_UPDATE_MANIFEST_URL.map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_UPDATE_MANIFEST_URL.to_owned())
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
    println!(
        "La actualizaci\u{00f3}n se descargar\u{00e1} y verificar\u{00e1} al cerrar esta ventana."
    );
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
            update_manifest_url: Some("https://example.com/update.json".into()),
            install_update: false,
            ui: false,
        };
        assert_eq!(
            configured_update_manifest_url(&args),
            "https://example.com/update.json"
        );
    }
}
