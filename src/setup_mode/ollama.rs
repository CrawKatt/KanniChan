use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;
use std::process::Child;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::app::paths::path_setting;
use crate::app::preferences::Preferences;
use crate::app::process::background_command;

const OLLAMA_URL: &str = "http://127.0.0.1:11434";
const DEFAULT_OLLAMA_DIR: &str = "ollama";

#[cfg(not(windows))]
const OWN_BINARY: &str = "bin/ollama";
#[cfg(windows)]
const OWN_BINARY: &str = "ollama.exe";

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
static STARTED_SERVER: Mutex<Option<Child>> = Mutex::new(None);

pub fn install_dir() -> PathBuf {
    path_setting("OLLAMA_DIR", DEFAULT_OLLAMA_DIR)
}

fn binary() -> Option<PathBuf> {
    let own_copy = install_dir().join(OWN_BINARY);

    if own_copy.exists() {
        return Some(own_copy);
    }

    let path = std::env::var_os("PATH")?;

    std::env::split_paths(&path)
        .map(|folder| folder.join(format!("ollama{}", std::env::consts::EXE_SUFFIX)))
        .find(|candidate| candidate.exists())
}

pub fn is_running() -> bool {
    ureq::get(format!("{OLLAMA_URL}/api/version"))
        .config()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .call()
        .is_ok()
}

pub fn is_installed() -> bool {
    is_running() || binary().is_some()
}

pub fn ensure_running() -> Result<(), String> {
    let mut started_server = STARTED_SERVER
        .lock()
        .map_err(|_| t!("error.ollama_unresponsive").to_string())?;

    if is_running() {
        return Ok(());
    }

    let binary = binary().ok_or_else(|| t!("error.ollama_not_installed").to_string())?;

    let mut command = background_command(&binary);
    command.arg("serve");

    if let Some(install_dir) = Preferences::load().install_dir {
        command.env("OLLAMA_MODELS", install_dir.join("ollama-models"));
    }

    let mut server = command
        .spawn()
        .map_err(|error| t!("error.ollama_start", error = error).to_string())?;

    let deadline = Instant::now() + STARTUP_TIMEOUT;

    while Instant::now() < deadline {
        if is_running() {
            *started_server = Some(server);
            return Ok(());
        }

        if let Ok(Some(status)) = server.try_wait() {
            return Err(t!("error.ollama_exited", status = status).to_string());
        }

        thread::sleep(Duration::from_millis(300));
    }

    let _ = server.kill();

    Err(t!("error.ollama_timeout").to_string())
}

pub fn stop_started_server() {
    if let Ok(mut started_server) = STARTED_SERVER.lock()
        && let Some(mut server) = started_server.take()
    {
        let _ = server.kill();
    }
}

pub fn has_model(name: &str) -> bool {
    let Ok(mut response) = ureq::get(format!("{OLLAMA_URL}/api/tags")).call() else {
        return false;
    };

    let Ok(tags) = response.body_mut().read_json::<Value>() else {
        return false;
    };

    tags["models"]
        .as_array()
        .is_some_and(|models| models.iter().any(|model| model["name"] == name))
}

pub fn pull_model(name: &str, on_progress: &mut dyn FnMut(f32)) -> Result<(), String> {
    let response = ureq::post(format!("{OLLAMA_URL}/api/pull"))
        .send_json(json!({ "model": name, "stream": true }))
        .map_err(|error| t!("error.ollama_pull", error = error).to_string())?;

    let reader = BufReader::new(
        response
            .into_body()
            .into_with_config()
            .limit(u64::MAX)
            .reader(),
    );

    for line in reader.lines() {
        let line = line.map_err(|error| t!("error.download_interrupted", error = error).to_string())?;

        let Ok(update) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        if let Some(error) = update["error"].as_str() {
            return Err(t!("error.ollama_pull", error = error).to_string());
        }

        if update["status"] == "success" {
            return Ok(());
        }

        if let (Some(completed), Some(total)) =
            (update["completed"].as_f64(), update["total"].as_f64())
            && total > 0.0
        {
            on_progress((completed / total) as f32);
        }
    }

    Err(t!("error.ollama_pull_unconfirmed").to_string())
}

pub fn create_model(name: &str, from: &str, template: &str, stop: &str) -> Result<(), String> {
    ureq::post(format!("{OLLAMA_URL}/api/create"))
        .send_json(json!({
            "model": name,
            "from": from,
            "template": template,
            "parameters": { "stop": [stop] },
            "stream": false,
        }))
        .map(|_| ())
        .map_err(|error| t!("error.ollama_create", error = error).to_string())
}
