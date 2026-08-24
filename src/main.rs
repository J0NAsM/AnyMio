mod identity;
mod protocol;
mod relay;
// Used by the future unattended-access flow; keep it compiled and covered by unit tests.
#[allow(dead_code)]
mod security;
mod update;

use std::{env, net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};
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
}

// The publisher can set this at build time. A command-line URL or runtime
// environment variable is useful for testing and takes precedence.
const BUILT_IN_UPDATE_MANIFEST_URL: Option<&str> = option_env!("JREMOTE_UPDATE_MANIFEST_URL");

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
        let identity = DeviceIdentity::load_or_create(args.data_dir.clone())
            .context("could not load the local device identity")?;
        println!("JRemote {}", update::CURRENT_VERSION);
        println!("This device ID: {}", identity.public_id_formatted());
        println!("No remote session is active.");
        println!("The GUI/capture endpoint is not yet included in this build.");
        if let Some(manifest_url) = configured_update_manifest_url(&args) {
            match update::check(&manifest_url).await {
                Ok(Some(release)) => show_update_message(&release),
                Ok(None) => {}
                Err(error) => tracing::debug!(%error, "update check failed"),
            }
        }
        Ok(())
    }
}

fn configured_update_manifest_url(args: &Args) -> Option<String> {
    args.update_manifest_url
        .clone()
        .or_else(|| {
            env::var("JREMOTE_UPDATE_MANIFEST_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| BUILT_IN_UPDATE_MANIFEST_URL.map(str::to_owned))
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
        };
        assert_eq!(
            configured_update_manifest_url(&args).as_deref(),
            Some("https://example.com/update.json")
        );
    }
}
