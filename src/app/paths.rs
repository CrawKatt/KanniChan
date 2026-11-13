use std::fs;
use std::path::PathBuf;

/// The app's folder inside the data and config folders.
pub const APP_FOLDER: &str = "kannichan";

/// The app was called ui-translate before; its folders hold the user's preferences.
const LEGACY_APP_FOLDER: &str = "ui-translate";

pub fn data_dir() -> PathBuf {
    if cfg!(windows) {
        return env_path("LOCALAPPDATA").unwrap_or_default();
    }

    env_path("XDG_DATA_HOME").unwrap_or_else(|| home().join(".local/share"))
}

pub fn config_dir() -> PathBuf {
    if cfg!(windows) {
        return env_path("APPDATA").unwrap_or_default();
    }

    env_path("XDG_CONFIG_HOME").unwrap_or_else(|| home().join(".config"))
}

pub fn path_setting(variable: &str, default_in_data_dir: &str) -> PathBuf {
    env_path(variable).unwrap_or_else(|| data_dir().join(default_in_data_dir))
}

fn env_path(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn home() -> PathBuf {
    env_path("HOME").unwrap_or_default()
}

pub fn move_legacy_folders() {
    for base in [data_dir(), config_dir()] {
        let legacy = base.join(LEGACY_APP_FOLDER);
        let current = base.join(APP_FOLDER);

        if legacy.is_dir() && !current.exists() {
            let _ = fs::rename(legacy, current);
        }
    }
}
