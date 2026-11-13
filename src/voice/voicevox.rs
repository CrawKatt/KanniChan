use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::app::paths::path_setting;
use crate::app::preferences::Preferences;
use crate::app::process::background_command;

const DEFAULT_URL: &str = "http://127.0.0.1:50021";
#[cfg(not(windows))]
const DEFAULT_RUN: &str = "voicevox/linux-cpu-x64/run";
#[cfg(windows)]
const DEFAULT_RUN: &str = "voicevox/windows-cpu/run.exe";

const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_AUDIO_BYTES: u64 = 200 * 1024 * 1024;

/// Only the engine this app started is stopped on exit; one started by hand is left alone.
static STARTED_ENGINE: Mutex<Option<Child>> = Mutex::new(None);

#[derive(Clone, PartialEq)]
pub struct Voice {
    pub speaker_id: u32,
    pub character: String,
    pub style: String,
}

pub fn run_path() -> PathBuf {
    path_setting("VOICEVOX_RUN", DEFAULT_RUN)
}

fn url() -> String {
    std::env::var("VOICEVOX_URL").unwrap_or_else(|_| DEFAULT_URL.into())
}

pub fn selected_speaker() -> u32 {
    std::env::var("VOICEVOX_SPEAKER")
        .ok()
        .and_then(|speaker| speaker.parse().ok())
        .unwrap_or_else(|| Preferences::load().voicevox_speaker)
}

pub fn is_installed() -> bool {
    run_path().exists()
}

pub fn is_running() -> bool {
    ureq::get(format!("{}/version", url()))
        .config()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .call()
        .is_ok()
}

pub fn ensure_running() -> Result<(), String> {
    let mut started_engine = STARTED_ENGINE.lock().unwrap_or_else(PoisonError::into_inner);

    if is_running() {
        return Ok(());
    }

    let run = run_path();

    let mut engine = background_command(&run)
        .spawn()
        .map_err(|error| t!("error.voicevox_start", path = run.display(), error = error).to_string())?;

    let deadline = Instant::now() + STARTUP_TIMEOUT;

    while Instant::now() < deadline {
        if is_running() {
            *started_engine = Some(engine);
            return Ok(());
        }

        if let Ok(Some(status)) = engine.try_wait() {
            return Err(t!("error.voicevox_exited", status = status).to_string());
        }

        thread::sleep(Duration::from_millis(500));
    }

    let _ = engine.kill();

    Err(t!("error.voicevox_timeout", seconds = STARTUP_TIMEOUT.as_secs()).to_string())
}

pub fn stop_started_engine() {
    if let Ok(mut started_engine) = STARTED_ENGINE.lock()
        && let Some(mut engine) = started_engine.take()
    {
        let _ = engine.kill();
    }
}

/// Every style of every character, e.g. 四国めたん (ノーマル), 四国めたん (あまあま)…
pub fn voices() -> Result<Vec<Voice>, String> {
    let speakers: Value = ureq::get(format!("{}/speakers", url()))
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .map_err(|error| t!("error.voicevox_voices", error = error).to_string())?;

    let voices = speakers
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|speaker| {
            let character = speaker["name"].as_str().unwrap_or_default().to_string();

            speaker["styles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(move |style| {
                    Some(Voice {
                        speaker_id: style["id"].as_u64()? as u32,
                        character: character.clone(),
                        style: style["name"].as_str()?.to_string(),
                    })
                })
        })
        .collect();

    Ok(voices)
}

pub fn synthesize(text: &str, speaker: u32, audio_file: &Path) -> Result<(), String> {
    let url = url();
    let speaker = speaker.to_string();

    let not_reachable = |error: ureq::Error| {
        t!("error.voicevox_unreachable", url = url, error = error).to_string()
    };

    // VOICEVOX first plans the pronunciation, then renders that plan as audio.
    let pronunciation: Value = ureq::post(format!("{url}/audio_query"))
        .query("text", text)
        .query("speaker", &speaker)
        .send_empty()
        .and_then(|mut response| response.body_mut().read_json())
        .map_err(not_reachable)?;

    let audio = ureq::post(format!("{url}/synthesis"))
        .query("speaker", &speaker)
        .send_json(&pronunciation)
        .and_then(|mut response| {
            response
                .body_mut()
                .with_config()
                .limit(MAX_AUDIO_BYTES)
                .read_to_vec()
        })
        .map_err(not_reachable)?;

    std::fs::write(audio_file, audio).map_err(|error| {
        t!("error.audio_save", path = audio_file.display(), error = error).to_string()
    })
}
