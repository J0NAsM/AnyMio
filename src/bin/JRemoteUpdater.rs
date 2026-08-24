use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

const MAX_DOWNLOAD_BYTES: u64 = 200 * 1024 * 1024;
const REPLACE_RETRIES: u8 = 30;

#[derive(Debug, Parser)]
#[command(name = "JRemoteUpdater", about = "JRemote's verified update helper")]
struct Args {
    #[arg(long)]
    target: PathBuf,
    #[arg(long)]
    url: Option<String>,
    #[arg(long)]
    sha256: Option<String>,
    /// Restore JRemote.previous.exe instead of downloading a new update.
    #[arg(long)]
    rollback: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    validate_target(&args.target)?;
    if args.rollback {
        return rollback_and_restart(&args.target);
    }
    let url = args
        .url
        .context("--url is required unless --rollback is used")?;
    let sha256 = args
        .sha256
        .context("--sha256 is required unless --rollback is used")?;
    validate_https(&url)?;
    validate_sha256(&sha256)?;

    let download_path = temporary_download_path(&args.target)?;
    download_and_verify(&url, &sha256, &download_path).await?;
    replace_and_restart(&args.target, &download_path)
}

fn rollback_and_restart(target: &Path) -> Result<()> {
    let backup = target.with_file_name("JRemote.previous.exe");
    if !backup.is_file() {
        bail!("there is no previous JRemote executable to restore");
    }
    let staged = target.with_file_name("JRemote.rollback-staged.exe");
    for attempt in 1..=REPLACE_RETRIES {
        match rollback_once(target, &backup, &staged) {
            Ok(()) => {
                Command::new(target)
                    .spawn()
                    .context("the restored application could not be started")?;
                return Ok(());
            }
            Err(error) if attempt < REPLACE_RETRIES => {
                std::thread::sleep(Duration::from_secs(1));
                let _ = error;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the retry loop always returns")
}

fn rollback_once(target: &Path, backup: &Path, staged: &Path) -> Result<()> {
    if staged.exists() {
        std::fs::remove_file(staged).context("could not remove stale rollback staging")?;
    }
    std::fs::rename(target, staged).context("JRemote is still running or cannot be replaced")?;
    if let Err(error) = std::fs::rename(backup, target) {
        std::fs::rename(staged, target).context("could not restore the current executable")?;
        return Err(error).context("could not restore the previous executable");
    }
    std::fs::rename(staged, backup)
        .context("could not retain the newer executable as rollback backup")
}

fn validate_target(target: &Path) -> Result<()> {
    if target.file_name() != Some(OsStr::new("JRemote.exe")) {
        bail!("the updater may only replace JRemote.exe");
    }
    if !target.is_file() {
        bail!("the target executable does not exist");
    }
    Ok(())
}

fn validate_https(url: &str) -> Result<()> {
    let parsed = reqwest::Url::parse(url).context("the update URL is malformed")?;
    if parsed.scheme() != "https" {
        bail!("the update URL must use HTTPS");
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("the SHA-256 checksum must contain exactly 64 hexadecimal characters");
    }
    Ok(())
}

fn temporary_download_path(target: &Path) -> Result<PathBuf> {
    let parent = target
        .parent()
        .context("the target executable has no parent directory")?;
    Ok(parent.join("JRemote.exe.download"))
}

async fn download_and_verify(url: &str, expected_sha256: &str, destination: &Path) -> Result<()> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("JRemoteUpdater")
        .build()?
        .get(url)
        .send()
        .await
        .context("could not download the update")?
        .error_for_status()
        .context("the update server returned an error")?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_DOWNLOAD_BYTES)
    {
        bail!("the update exceeds the 200 MiB download limit");
    }

    let mut file = tokio::fs::File::create(destination)
        .await
        .context("could not create the temporary update file")?;
    let mut stream = response.bytes_stream();
    let mut downloaded = 0_u64;
    let mut hasher = Sha256::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("the update download was interrupted")?;
        downloaded = downloaded
            .checked_add(chunk.len() as u64)
            .context("the update size overflowed")?;
        if downloaded > MAX_DOWNLOAD_BYTES {
            bail!("the update exceeds the 200 MiB download limit");
        }
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .context("could not save the update")?;
    }
    file.flush()
        .await
        .context("could not finish the update file")?;

    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        bail!("the downloaded update does not match its SHA-256 checksum");
    }
    Ok(())
}

fn replace_and_restart(target: &Path, download: &Path) -> Result<()> {
    let backup = target.with_file_name("JRemote.previous.exe");
    for attempt in 1..=REPLACE_RETRIES {
        match replace_once(target, download, &backup) {
            Ok(()) => {
                Command::new(target)
                    .spawn()
                    .context("the updated application could not be started")?;
                return Ok(());
            }
            Err(error) if attempt < REPLACE_RETRIES => {
                if attempt == REPLACE_RETRIES - 1 {
                    eprintln!("Waiting for JRemote to close before replacing it...");
                }
                std::thread::sleep(Duration::from_secs(1));
                let _ = error;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the retry loop always returns")
}

fn replace_once(target: &Path, download: &Path, backup: &Path) -> Result<()> {
    if backup.exists() {
        std::fs::remove_file(backup).context("could not remove the previous update backup")?;
    }
    std::fs::rename(target, backup).context("JRemote is still running or cannot be replaced")?;
    if let Err(error) = std::fs::rename(download, target) {
        std::fs::rename(backup, target)
            .context("could not restore the previous JRemote executable")?;
        return Err(error).context("could not install the downloaded update");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_the_expected_target_name() {
        assert!(validate_target(Path::new("not-jremote.exe")).is_err());
    }

    #[test]
    fn rejects_non_https_urls_and_bad_hashes() {
        assert!(validate_https("http://example.com/JRemote.exe").is_err());
        assert!(validate_sha256("bad").is_err());
    }
}
