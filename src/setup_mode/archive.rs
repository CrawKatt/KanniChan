use std::fs::File;
use std::path::Path;

use flate2::read::GzDecoder;

fn open(archive: &Path) -> Result<File, String> {
    File::open(archive).map_err(|error| t!("error.open", path = archive.display(), error = error).to_string())
}

fn extraction_error(error: impl std::fmt::Display) -> String {
    t!("error.extract", error = error).to_string()
}

#[cfg_attr(windows, allow(dead_code))]
pub fn extract_tar_gz(archive: &Path, destination: &Path) -> Result<(), String> {
    tar::Archive::new(GzDecoder::new(open(archive)?))
        .unpack(destination)
        .map_err(extraction_error)
}

#[cfg_attr(windows, allow(dead_code))]
pub fn extract_tar_zst(archive: &Path, destination: &Path) -> Result<(), String> {
    let decoder = zstd::Decoder::new(open(archive)?).map_err(extraction_error)?;

    tar::Archive::new(decoder)
        .unpack(destination)
        .map_err(extraction_error)
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn extract_zip(archive: &Path, destination: &Path) -> Result<(), String> {
    zip::ZipArchive::new(open(archive)?)
        .and_then(|mut zip| zip.extract(destination))
        .map_err(extraction_error)
}

pub fn extract_7z(archive: &Path, destination: &Path) -> Result<(), String> {
    sevenz_rust2::decompress_file(archive, destination).map_err(extraction_error)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use flate2::Compression;
    use flate2::write::GzEncoder;

    use super::*;

    #[test]
    fn extracts_a_tar_gz_into_its_folders() {
        let workspace = std::env::temp_dir().join(format!("kannichan-test-{}", std::process::id()));
        let archive = workspace.join("test.tar.gz");
        let destination = workspace.join("out");

        fs::create_dir_all(&workspace).unwrap();

        let mut builder = tar::Builder::new(GzEncoder::new(
            File::create(&archive).unwrap(),
            Compression::fast(),
        ));

        let content = b"hola";
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();

        builder
            .append_data(&mut header, "piper/piper", &content[..])
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap();

        extract_tar_gz(&archive, &destination).unwrap();

        let extracted = fs::read(destination.join("piper/piper")).unwrap();
        let _ = fs::remove_dir_all(&workspace);

        assert_eq!(extracted, content);
    }
}
