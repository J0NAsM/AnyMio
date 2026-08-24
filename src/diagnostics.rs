use std::time::Duration;

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
        Some(url) if url.starts_with("wss://") => results.push(CheckResult {
            name: "Relay",
            detail: format!("WSS configurado: {url}"),
            ok: true,
        }),
        Some(url) => results.push(failure("Relay", format!("URL no segura: {url}"))),
        None => results.push(failure("Relay", "No configurado".into())),
    }
    results
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
}
