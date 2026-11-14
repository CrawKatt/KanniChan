use std::fs;
use std::path::PathBuf;

use crate::app::paths::data_dir;

const SETUP_MARKER: &str = "kannichan/setup-done";

fn marker_path() -> PathBuf {
    data_dir().join(SETUP_MARKER)
}

pub fn is_done() -> bool {
    marker_path().exists()
}

pub fn mark_done() -> Result<(), String> {
    let marker = marker_path();

    if let Some(folder) = marker.parent() {
        fs::create_dir_all(folder)
            .map_err(|error| t!("error.create", path = folder.display(), error = error).to_string())?;
    }

    fs::write(&marker, "")
        .map_err(|error| t!("error.save", path = marker.display(), error = error).to_string())
}
