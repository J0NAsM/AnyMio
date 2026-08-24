use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const EVENTS_FILE: &str = "events.jsonl";
const MAX_EVENT_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
    pub timestamp_unix: u64,
    pub kind: String,
    pub detail: String,
}

pub fn append(dir: &Path, kind: &str, detail: &str) -> Result<()> {
    let path = dir.join(EVENTS_FILE);
    if path
        .metadata()
        .is_ok_and(|metadata| metadata.len() > MAX_EVENT_BYTES)
    {
        std::fs::rename(&path, dir.join("events.previous.jsonl"))
            .context("could not rotate the event log")?;
    }
    let event = Event {
        timestamp_unix: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        kind: kind.to_owned(),
        detail: detail.to_owned(),
    };
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(&event)?)?;
    Ok(())
}

pub fn recent(dir: &Path, limit: usize) -> Result<Vec<Event>> {
    let path = dir.join(EVENTS_FILE);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let reader = BufReader::new(std::fs::File::open(path)?);
    let mut events = Vec::new();
    for line in reader.lines() {
        if let Ok(event) = serde_json::from_str::<Event>(&line?) {
            events.push(event);
        }
    }
    Ok(events.into_iter().rev().take(limit).collect())
}
