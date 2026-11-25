use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use serde_json::Value;

use crate::setup_mode::download::download;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/CrawKatt/KanniChan/releases/latest";

#[cfg(feature = "vulkan")]
const ASSET_NAME: &str = "kannichan-windows-amd.exe";
#[cfg(not(feature = "vulkan"))]
const ASSET_NAME: &str = "kannichan-windows-nvidia.exe";

static PENDING_RESTART: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(Clone)]
pub struct Release {
    pub version: String,
    pub page_url: String,
    download: Option<Download>,
}

#[derive(Clone)]
struct Download {
    url: String,
    sha256: String,
}

impl Release {
    pub fn can_install(&self) -> bool {
        self.download.is_some()
    }
}

pub fn check() -> Option<Release> {
    let mut response = ureq::get(LATEST_RELEASE_URL).call().ok()?;
    let latest = response.body_mut().read_json::<Value>().ok()?;

    let version = latest["tag_name"].as_str()?.trim_start_matches('v');

    if !is_newer(version, env!("CARGO_PKG_VERSION")) {
        return None;
    }

    let download = cfg!(windows)
        .then(|| find_download(&latest))
        .flatten();

    Some(Release {
        version: version.to_string(),
        page_url: latest["html_url"].as_str()?.to_string(),
        download,
    })
}

fn find_download(release: &Value) -> Option<Download> {
    let asset = release["assets"]
        .as_array()?
        .iter()
        .find(|asset| asset["name"] == ASSET_NAME)?;

    Some(Download {
        url: asset["browser_download_url"].as_str()?.to_string(),
        sha256: asset["digest"].as_str()?.strip_prefix("sha256:")?.to_string(),
    })
}

fn is_newer(candidate: &str, current: &str) -> bool {
    let numbers = |version: &str| -> Option<Vec<u64>> {
        version.split('.').map(|part| part.parse().ok()).collect()
    };

    match (numbers(candidate), numbers(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

pub fn install(release: &Release, on_progress: &mut dyn FnMut(f32)) -> Result<(), String> {
    let Some(file) = &release.download else {
        return Err(t!("error.update_unavailable").to_string());
    };

    let current = std::env::current_exe()
        .map_err(|error| t!("error.update_replace", error = error).to_string())?;

    let new = current.with_extension("new");
    let old = current.with_extension("old");

    download(&file.url, &file.sha256, &new, on_progress)?;

    let _ = fs::remove_file(&old);

    fs::rename(&current, &old)
        .map_err(|error| t!("error.update_replace", error = error).to_string())?;

    if let Err(error) = fs::rename(&new, &current) {
        let _ = fs::rename(&old, &current);

        return Err(t!("error.update_replace", error = error).to_string());
    }

    if let Ok(mut pending) = PENDING_RESTART.lock() {
        *pending = Some(current);
    }

    Ok(())
}

pub fn restart_if_updated() {
    let pending = PENDING_RESTART.lock().ok().and_then(|mut pending| pending.take());

    if let Some(program) = pending {
        let _ = Command::new(program).spawn();
    }
}

pub fn remove_previous_version() {
    if let Ok(current) = std::env::current_exe() {
        let _ = fs::remove_file(current.with_extension("old"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_by_number_not_text() {
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(is_newer("1.0.0", "0.99.9"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.1.9", "0.2.0"));
        assert!(!is_newer("0.3.0-beta", "0.2.0"));
    }
}
