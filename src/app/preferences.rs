use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::app::paths::{APP_FOLDER, config_dir};
use crate::stream_mode::subtitle_style::SubtitleStyle;
use crate::stream_mode::whisper_settings::WhisperSettings;
use crate::translate_mode::model::TranslationModel;

/// 四国めたん (ノーマル), a clear neutral voice.
const DEFAULT_VOICEVOX_SPEAKER: u32 = 2;

#[derive(Clone, Copy)]
pub struct Preferences {
    pub translation_model: TranslationModel,
    pub voicevox_speaker: u32,
    pub whisper: WhisperSettings,
    pub subtitle_style: SubtitleStyle,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            translation_model: TranslationModel::default(),
            voicevox_speaker: DEFAULT_VOICEVOX_SPEAKER,
            whisper: WhisperSettings::default(),
            subtitle_style: SubtitleStyle::default(),
        }
    }
}

fn file_path() -> PathBuf {
    config_dir().join(APP_FOLDER).join("preferences.json")
}

impl Preferences {
    pub fn load() -> Self {
        let saved: Value = fs::read_to_string(file_path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();

        let translation_model = saved["translation_model"]
            .as_str()
            .and_then(TranslationModel::from_id)
            .unwrap_or_default();

        let voicevox_speaker = saved["voicevox_speaker"]
            .as_u64()
            .map(|speaker| speaker as u32)
            .unwrap_or(DEFAULT_VOICEVOX_SPEAKER);

        Self {
            translation_model,
            voicevox_speaker,
            whisper: WhisperSettings::from_json(&saved["whisper"]),
            subtitle_style: SubtitleStyle::from_json(&saved["subtitle_style"]),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = file_path();

        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)
                .map_err(|error| t!("error.create", path = folder.display(), error = error).to_string())?;
        }

        let saved = json!({
            "translation_model": self.translation_model.id(),
            "voicevox_speaker": self.voicevox_speaker,
            "whisper": self.whisper.to_json(),
            "subtitle_style": self.subtitle_style.to_json(),
        });

        fs::write(&path, saved.to_string())
            .map_err(|error| t!("error.save", path = path.display(), error = error).to_string())
    }
}
