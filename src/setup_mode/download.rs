use std::fs::{self, File};
use std::io::{Read as _, Write as _};
use std::path::Path;

use sha2::{Digest as _, Sha256};

const CHUNK_SIZE: usize = 256 * 1024;

pub fn download(
    url: &str,
    expected_sha256: &str,
    destination: &Path,
    on_progress: &mut dyn FnMut(f32),
) -> Result<(), String> {
    if let Some(folder) = destination.parent() {
        fs::create_dir_all(folder)
            .map_err(|error| t!("error.create", path = folder.display(), error = error).to_string())?;
    }

    let partial = destination.with_extension("part");

    let response = ureq::get(url)
        .call()
        .map_err(|error| t!("error.download", url = url, error = error).to_string())?;

    let total_bytes = response.body().content_length().unwrap_or(0);

    let mut reader = response
        .into_body()
        .into_with_config()
        .limit(u64::MAX)
        .reader();

    let mut file = File::create(&partial)
        .map_err(|error| t!("error.create", path = partial.display(), error = error).to_string())?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0; CHUNK_SIZE];
    let mut downloaded_bytes = 0u64;

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| t!("error.download_interrupted", error = error).to_string())?;

        if read == 0 {
            break;
        }

        file.write_all(&buffer[..read])
            .map_err(|error| t!("error.write_file", error = error).to_string())?;

        hasher.update(&buffer[..read]);
        downloaded_bytes += read as u64;

        if total_bytes > 0 {
            on_progress(downloaded_bytes as f32 / total_bytes as f32);
        }
    }

    drop(file);

    let actual_sha256 = format!("{:x}", hasher.finalize());

    if actual_sha256 != expected_sha256 {
        let _ = fs::remove_file(&partial);

        return Err(t!("error.download_corrupt").to_string());
    }

    fs::rename(&partial, destination)
        .map_err(|error| t!("error.save", path = destination.display(), error = error).to_string())
}
