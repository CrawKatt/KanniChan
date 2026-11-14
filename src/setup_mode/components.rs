use std::fs;
use std::path::{Path, PathBuf};

use crate::app::paths::path_setting;
use crate::setup_mode::archive;
use crate::setup_mode::download::download;
use crate::setup_mode::ollama;
use crate::stream_mode::streamer::{vad_model_path, whisper_model_path};
use crate::translate_mode::model::TranslationModel;
use crate::voice::tts::{piper_bin_path, piper_voice_path};
use crate::voice::voicevox;

struct PinnedFile {
    url: &'static str,
    sha256: &'static str,
}

type Extractor = fn(&Path, &Path) -> Result<(), String>;

#[cfg(not(windows))]
mod platform {
    use super::{Extractor, PinnedFile};
    use crate::setup_mode::archive;

    pub const OLLAMA: PinnedFile = PinnedFile {
        url: "https://github.com/ollama/ollama/releases/download/v0.40.15/ollama-linux-amd64.tar.zst",
        sha256: "628c018404a4206493efd38e46251c154ede5141b8b8dc85c659b3b027c47379",
    };
    pub const OLLAMA_EXTRACTOR: Extractor = archive::extract_tar_zst;
    pub const OLLAMA_GPU_RUNTIME: Option<PinnedFile> = None;
    pub const WHISPER_GPU_RUNTIME: &[PinnedFile] = &[];

    pub const VOICEVOX: PinnedFile = PinnedFile {
        url: "https://github.com/VOICEVOX/voicevox_engine/releases/download/0.26.1/voicevox_engine-linux-cpu-x64-0.26.1.7z.001",
        sha256: "120f19bc4a6eb1147113279ecf581b2512934448e574027c36e086f709756b8b",
    };

    pub const PIPER: PinnedFile = PinnedFile {
        url: "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_x86_64.tar.gz",
        sha256: "a50cb45f355b7af1f6d758c1b360717877ba0a398cc8cbe6d2a7a3a26e225992",
    };
    pub const PIPER_EXTRACTOR: Extractor = archive::extract_tar_gz;
}

#[cfg(windows)]
mod platform {
    use super::{Extractor, PinnedFile};
    use crate::setup_mode::archive;

    pub const OLLAMA: PinnedFile = PinnedFile {
        url: "https://github.com/ollama/ollama/releases/download/v0.40.15/ollama-windows-amd64.zip",
        sha256: "9ca74a7ab66ec86870b1eecac7df160df08dc4e3a291ab5d2d59bcf9310221c6",
    };
    pub const OLLAMA_EXTRACTOR: Extractor = archive::extract_zip;

    // The base archive only accelerates NVIDIA; the AMD build adds Ollama's ROCm runtime on top.
    #[cfg(feature = "vulkan")]
    pub const OLLAMA_GPU_RUNTIME: Option<PinnedFile> = Some(PinnedFile {
        url: "https://github.com/ollama/ollama/releases/download/v0.40.15/ollama-windows-amd64-rocm.zip",
        sha256: "2f5e81675329a5de2477a4a25823d55d5c3ead8d59522e92eac1664ab492faeb",
    });
    #[cfg(not(feature = "vulkan"))]
    pub const OLLAMA_GPU_RUNTIME: Option<PinnedFile> = None;

    // Must match the CUDA version the release is built with (see the Windows workflow).
    #[cfg(feature = "cuda")]
    pub const WHISPER_GPU_RUNTIME: &[PinnedFile] = &[
        PinnedFile {
            url: "https://developer.download.nvidia.com/compute/cuda/redist/cuda_cudart/windows-x86_64/cuda_cudart-windows-x86_64-13.2.51-archive.zip",
            sha256: "906b430b53e6396f7cbaa6874fbe7f9307a184e830b119a4ad3f58dc6e0165c6",
        },
        PinnedFile {
            url: "https://developer.download.nvidia.com/compute/cuda/redist/libcublas/windows-x86_64/libcublas-windows-x86_64-13.3.0.5-archive.zip",
            sha256: "438a88f07af3721eda15ed1f4d2b957ea810e29061b2a6292837f35520dbd212",
        },
    ];
    #[cfg(not(feature = "cuda"))]
    pub const WHISPER_GPU_RUNTIME: &[PinnedFile] = &[];

    pub const VOICEVOX: PinnedFile = PinnedFile {
        url: "https://github.com/VOICEVOX/voicevox_engine/releases/download/0.26.1/voicevox_engine-windows-cpu-0.26.1.7z.001",
        sha256: "2143ab794bf7241b1f88704f88bf4fb10ea6cf453035f54e1b3a73aea2c668fc",
    };

    pub const PIPER: PinnedFile = PinnedFile {
        url: "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip",
        sha256: "f3c58906402b24f3a96d92145f58acba6d86c9b5db896d207f78dc80811efcea",
    };
    pub const PIPER_EXTRACTOR: Extractor = archive::extract_zip;
}

const WHISPER_MODEL: PinnedFile = PinnedFile {
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
    sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
};

const VAD_MODEL: PinnedFile = PinnedFile {
    url: "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v6.2.0.bin",
    sha256: "2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987",
};

const PIPER_VOICE: PinnedFile = PinnedFile {
    url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_MX/claude/high/es_MX-claude-high.onnx",
    sha256: "3ef40a71ea63852cd8ab7e6fa7d2ecdcfa67a0b47c9c48e3f10e02ee02083ea0",
};

const PIPER_VOICE_CONFIG: PinnedFile = PinnedFile {
    url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_MX/claude/high/es_MX-claude-high.onnx.json",
    sha256: "1afc81f703c0e4cb3b4d7c0dca096b8b54a98806807f0170cf5eb5557723c12d",
};

pub enum Step {
    Downloading(f32),
    Extracting,
    Starting,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component {
    Ollama,
    TranslationModel,
    Whisper,
    Voicevox,
    Piper,
}

impl Component {
    pub const ALL: [Component; 5] = [
        Component::Ollama,
        Component::TranslationModel,
        Component::Whisper,
        Component::Voicevox,
        Component::Piper,
    ];

    pub fn name(self) -> String {
        match self {
            Component::Ollama => "Ollama".into(),
            Component::TranslationModel => {
                t!("component.model_name", model = TranslationModel::selected().display_name())
                    .to_string()
            }
            Component::Whisper => "Whisper".into(),
            Component::Voicevox => "VOICEVOX".into(),
            Component::Piper => t!("component.piper_name").to_string(),
        }
    }

    pub fn purpose(self) -> String {
        let key = match self {
            Component::Ollama => "component.ollama_purpose",
            Component::TranslationModel => "component.model_purpose",
            Component::Whisper => "component.whisper_purpose",
            Component::Voicevox => "component.voicevox_purpose",
            Component::Piper => "component.piper_purpose",
        };

        t!(key).to_string()
    }

    pub fn download_size(self) -> String {
        let key = match self {
            Component::Ollama => "size.ollama",
            Component::TranslationModel => return TranslationModel::selected().download_size(),
            Component::Whisper if platform::WHISPER_GPU_RUNTIME.is_empty() => "size.whisper",
            Component::Whisper => "size.whisper_cuda",
            Component::Voicevox => "size.voicevox",
            Component::Piper => "size.piper",
        };

        t!(key).to_string()
    }

    pub fn is_essential(self) -> bool {
        matches!(self, Component::Ollama | Component::TranslationModel)
    }

    pub fn is_installed(self) -> bool {
        match self {
            Component::Ollama => ollama::is_installed(),
            Component::TranslationModel => {
                ollama::is_running()
                    && ollama::has_model(TranslationModel::selected().ollama_name())
            }
            Component::Whisper => {
                whisper_model_path().exists() && vad_model_path().exists() && whisper_gpu_runtime_installed()
            }
            Component::Voicevox => voicevox::is_installed(),
            Component::Piper => piper_bin_path().exists() && piper_voice_path().exists(),
        }
    }

    pub fn install(self, report: &mut dyn FnMut(Step)) -> Result<(), String> {
        match self {
            Component::Ollama => install_ollama(report),
            Component::TranslationModel => install_translation_model(report),
            Component::Whisper => install_whisper(report),
            Component::Voicevox => install_voicevox(report),
            Component::Piper => install_piper(report),
        }
    }
}

fn install_archive(
    file: &PinnedFile,
    destination: &Path,
    extract: Extractor,
    report: &mut dyn FnMut(Step),
) -> Result<(), String> {
    let archive = destination.join("download.tmp");

    download(file.url, file.sha256, &archive, &mut |fraction| {
        report(Step::Downloading(fraction))
    })?;

    report(Step::Extracting);

    let extracted = extract(&archive, destination);
    let _ = fs::remove_file(&archive);

    extracted
}

fn grandparent(path: &Path) -> Result<PathBuf, String> {
    path.parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| t!("error.invalid_install_path", path = path.display()).to_string())
}

fn install_ollama(report: &mut dyn FnMut(Step)) -> Result<(), String> {
    install_archive(
        &platform::OLLAMA,
        &ollama::install_dir(),
        platform::OLLAMA_EXTRACTOR,
        report,
    )?;

    if let Some(runtime) = &platform::OLLAMA_GPU_RUNTIME {
        install_archive(runtime, &ollama::install_dir(), platform::OLLAMA_EXTRACTOR, report)?;
    }

    report(Step::Starting);

    ollama::ensure_running()
}

fn install_translation_model(report: &mut dyn FnMut(Step)) -> Result<(), String> {
    let model = TranslationModel::selected();

    report(Step::Starting);

    ollama::ensure_running()?;

    ollama::pull_model(model.source(), &mut |fraction| {
        report(Step::Downloading(fraction))
    })?;

    if let Some((template, stop)) = model.fixed_template() {
        ollama::create_model(model.ollama_name(), model.source(), template, stop)?;
    }

    Ok(())
}

fn whisper_gpu_dll_folder(file: &PinnedFile) -> PathBuf {
    let archive_name = file.url.rsplit('/').next().unwrap_or_default().trim_end_matches(".zip");

    whisper_gpu_runtime_dir().join(archive_name).join("bin").join("x64")
}

fn whisper_gpu_runtime_dir() -> PathBuf {
    path_setting("WHISPER_GPU_RUNTIME", "whisper/gpu-runtime")
}

pub fn whisper_gpu_runtime_installed() -> bool {
    platform::WHISPER_GPU_RUNTIME
        .iter()
        .all(|file| whisper_gpu_dll_folder(file).exists())
}

pub fn whisper_gpu_runtime_loadable() -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let searched: Vec<PathBuf> = std::env::split_paths(&path).collect();

    whisper_gpu_runtime_installed()
        && platform::WHISPER_GPU_RUNTIME
            .iter()
            .all(|file| searched.contains(&whisper_gpu_dll_folder(file)))
}

/// Lets Windows find the delay-loaded CUDA DLLs (see build.rs) once they are installed.
/// Must run before any other thread starts, as it changes the environment.
pub fn add_whisper_gpu_runtime_to_dll_path() {
    if platform::WHISPER_GPU_RUNTIME.is_empty() {
        return;
    }

    let folders = platform::WHISPER_GPU_RUNTIME.iter().map(whisper_gpu_dll_folder);
    let current = std::env::var_os("PATH").unwrap_or_default();

    if let Ok(path) = std::env::join_paths(folders.chain(std::env::split_paths(&current))) {
        // SAFETY: called from main before the app spawns threads.
        unsafe { std::env::set_var("PATH", path) };
    }
}

fn install_whisper(report: &mut dyn FnMut(Step)) -> Result<(), String> {
    let model = whisper_model_path();

    if !model.exists() {
        download(WHISPER_MODEL.url, WHISPER_MODEL.sha256, &model, &mut |fraction| {
            report(Step::Downloading(fraction))
        })?;
    }

    let vad_model = vad_model_path();

    if !vad_model.exists() {
        download(VAD_MODEL.url, VAD_MODEL.sha256, &vad_model, &mut |_| {})?;
    }

    for file in platform::WHISPER_GPU_RUNTIME {
        if !whisper_gpu_dll_folder(file).exists() {
            install_archive(file, &whisper_gpu_runtime_dir(), archive::extract_zip, report)?;
        }
    }

    Ok(())
}

fn install_voicevox(report: &mut dyn FnMut(Step)) -> Result<(), String> {
    let run = voicevox::run_path();

    install_archive(&platform::VOICEVOX, &grandparent(&run)?, archive::extract_7z, report)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        fs::set_permissions(&run, fs::Permissions::from_mode(0o755))
            .map_err(|error| t!("error.voicevox_prepare", error = error).to_string())?;
    }

    Ok(())
}

fn install_piper(report: &mut dyn FnMut(Step)) -> Result<(), String> {
    install_archive(
        &platform::PIPER,
        &grandparent(&piper_bin_path())?,
        platform::PIPER_EXTRACTOR,
        report,
    )?;

    let voice = piper_voice_path();
    let voice_config = PathBuf::from(format!("{}.json", voice.display()));

    download(PIPER_VOICE.url, PIPER_VOICE.sha256, &voice, &mut |fraction| {
        report(Step::Downloading(fraction))
    })?;

    download(
        PIPER_VOICE_CONFIG.url,
        PIPER_VOICE_CONFIG.sha256,
        &voice_config,
        &mut |_| {},
    )
}
