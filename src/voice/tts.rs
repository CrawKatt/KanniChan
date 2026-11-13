use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::app::paths::path_setting;
use crate::app::process::background_command;
use crate::translate_mode::translate::Translation;
use crate::voice::voicevox;

#[cfg(not(windows))]
const DEFAULT_PIPER_BIN: &str = "piper/piper/piper";
#[cfg(windows)]
const DEFAULT_PIPER_BIN: &str = "piper/piper/piper.exe";

const DEFAULT_PIPER_VOICE: &str = "piper-voices/es_MX-claude-high.onnx";
const JAPANESE_SAMPLE: &str = "こんにちは。今日はいい天気ですね。一緒に日本語を勉強しましょう。";

fn audio_file() -> PathBuf {
    std::env::temp_dir().join("kannichan-speech.wav")
}

pub fn speak(translation: &Translation) -> Result<(), String> {
    let text = translation.plain_text();
    let audio_file = audio_file();

    match translation {
        Translation::Japanese(_) => {
            voicevox::synthesize(&text, voicevox::selected_speaker(), &audio_file)?
        }
        Translation::Spanish(_) => synthesize_spanish(&text, &audio_file)?,
    }

    play(&audio_file)
}

pub fn speak_japanese_sample(speaker: u32) -> Result<(), String> {
    let audio_file = audio_file();

    voicevox::synthesize(JAPANESE_SAMPLE, speaker, &audio_file)?;

    play(&audio_file)
}

pub fn piper_bin_path() -> PathBuf {
    path_setting("PIPER_BIN", DEFAULT_PIPER_BIN)
}

pub fn piper_voice_path() -> PathBuf {
    path_setting("PIPER_VOICE", DEFAULT_PIPER_VOICE)
}

fn synthesize_spanish(text: &str, audio_file: &Path) -> Result<(), String> {
    let piper = piper_bin_path();
    let voice = piper_voice_path();

    if !voice.exists() {
        return Err(t!("error.piper_voice_missing", path = voice.display()).to_string());
    }

    let mut process = background_command(&piper)
        .arg("--model")
        .arg(&voice)
        .arg("--output_file")
        .arg(audio_file)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| t!("error.piper_run", path = piper.display(), error = error).to_string())?;

    process
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(text.as_bytes())
        .map_err(|error| t!("error.piper_send", error = error).to_string())?;

    let status = process
        .wait()
        .map_err(|error| t!("error.piper_failed", error = error).to_string())?;

    if !status.success() {
        return Err(t!("error.piper_exit", status = status).to_string());
    }

    Ok(())
}

fn play(audio_file: &Path) -> Result<(), String> {
    let output = rodio::DeviceSinkBuilder::open_default_sink()
        .map_err(|error| t!("error.audio_output", error = error).to_string())?;

    let file = std::fs::File::open(audio_file)
        .map_err(|error| t!("error.audio_open", error = error).to_string())?;

    let sound = rodio::Decoder::try_from(file)
        .map_err(|error| t!("error.audio_invalid", error = error).to_string())?;

    let player = rodio::Player::connect_new(output.mixer());
    player.append(sound);
    player.sleep_until_end();

    Ok(())
}
