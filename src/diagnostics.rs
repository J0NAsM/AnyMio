use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::{net::TcpStream, time::timeout};

#[derive(Debug)]
pub struct CheckResult {
    pub name: &'static str,
    pub detail: String,
    pub ok: bool,
}

pub async fn run(update_url: &str, relay_url: Option<&str>) -> Vec<CheckResult> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(error) => return vec![failure("HTTP", error.to_string())],
    };
    let mut results = vec![check_https(&client, "Manifiesto", update_url).await];
    match relay_url {
        Some(url) if url.starts_with("https://") => {
            results.push(check_https(&client, "Relay", url).await)
        }
        Some(url) if url.starts_with("wss://") => results.push(check_wss(url).await),
        Some(url) => results.push(failure("Relay", format!("URL no segura: {url}"))),
        None => results.push(failure("Relay", "No configurado".into())),
    }
    results
}

/// Opens only a TCP connection to make relay diagnostics non-intrusive. It does
/// not send a WebSocket handshake, relay message, or authentication proof.
async fn check_wss(url: &str) -> CheckResult {
    let address = match wss_address(url) {
        Ok(address) => address,
        Err(error) => return failure("Relay", error.to_string()),
    };
    match timeout(Duration::from_secs(5), TcpStream::connect(&address)).await {
        Ok(Ok(_stream)) => CheckResult {
            name: "Relay",
            detail: format!("Puerto WSS accesible: {address}"),
            ok: true,
        },
        Ok(Err(error)) => failure("Relay", format!("No se pudo conectar a {address}: {error}")),
        Err(_) => failure("Relay", format!("Tiempo de espera al conectar a {address}")),
    }
}

fn wss_address(url: &str) -> Result<String> {
    let parsed = reqwest::Url::parse(url).context("la URL WSS no es válida")?;
    if parsed.scheme() != "wss" {
        bail!("la URL del relay debe usar WSS");
    }
    let host = parsed.host_str().context("la URL WSS no tiene host")?;
    let host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    Ok(format!(
        "{host}:{}",
        parsed.port_or_known_default().unwrap_or(443)
    ))
}

async fn check_https(client: &reqwest::Client, name: &'static str, url: &str) -> CheckResult {
    if !url.starts_with("https://") {
        return failure(name, "La URL debe usar HTTPS".into());
    }
    match client.head(url).send().await {
        Ok(response) if response.status().is_success() => CheckResult {
            name,
            detail: format!("Disponible ({})", response.status()),
            ok: true,
        },
        Ok(response) => failure(name, format!("Respondió {}", response.status())),
        Err(error) => failure(name, format!("Sin conexión: {error}")),
    }
}

fn failure(name: &'static str, detail: String) -> CheckResult {
    CheckResult {
        name,
        detail,
        ok: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_is_explicit() {
        assert!(!failure("Relay", "No configurado".into()).ok);
    }

    #[test]
    fn wss_diagnostic_uses_the_configured_port() {
        assert_eq!(
            wss_address("wss://relay.example:9443/socket").unwrap(),
            "relay.example:9443"
        );
        assert!(wss_address("https://relay.example").is_err());
    }
}
