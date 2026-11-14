use serde_json::{Value, json};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WhisperSettings {
    pub speech_threshold: f64,
    pub voice_threshold: f64,
    pub end_of_speech_ms: f64,
    pub min_speech_ms: f64,
    pub max_segment_ms: f64,
    pub threads: f64,
    pub beam_size: f64,
}

impl Default for WhisperSettings {
    fn default() -> Self {
        Self {
            speech_threshold: 0.015,
            voice_threshold: 0.5,
            end_of_speech_ms: 700.,
            min_speech_ms: 300.,
            max_segment_ms: 12_000.,
            threads: 4.,
            beam_size: 1.,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Field {
    SpeechThreshold,
    VoiceThreshold,
    EndOfSpeech,
    MinSpeech,
    MaxSegment,
    Threads,
    BeamSize,
}

impl Field {
    pub const ALL: [Field; 7] = [
        Field::SpeechThreshold,
        Field::VoiceThreshold,
        Field::EndOfSpeech,
        Field::MinSpeech,
        Field::MaxSegment,
        Field::Threads,
        Field::BeamSize,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Field::SpeechThreshold => "speech_threshold",
            Field::VoiceThreshold => "voice_threshold",
            Field::EndOfSpeech => "end_of_speech_ms",
            Field::MinSpeech => "min_speech_ms",
            Field::MaxSegment => "max_segment_ms",
            Field::Threads => "threads",
            Field::BeamSize => "beam_size",
        }
    }

    /// Minimum, maximum and the step of the input's arrows.
    pub fn range(self) -> (f64, f64, f64) {
        match self {
            Field::SpeechThreshold => (0.005, 0.2, 0.005),
            Field::VoiceThreshold => (0.1, 0.9, 0.05),
            Field::EndOfSpeech => (200., 3_000., 100.),
            Field::MinSpeech => (100., 2_000., 100.),
            Field::MaxSegment => (2_000., 30_000., 1_000.),
            Field::Threads => (1., max_threads(), 1.),
            Field::BeamSize => (1., 8., 1.),
        }
    }

    pub fn get(self, settings: &WhisperSettings) -> f64 {
        match self {
            Field::SpeechThreshold => settings.speech_threshold,
            Field::VoiceThreshold => settings.voice_threshold,
            Field::EndOfSpeech => settings.end_of_speech_ms,
            Field::MinSpeech => settings.min_speech_ms,
            Field::MaxSegment => settings.max_segment_ms,
            Field::Threads => settings.threads,
            Field::BeamSize => settings.beam_size,
        }
    }

    pub fn is_fractional(self) -> bool {
        matches!(self, Field::SpeechThreshold | Field::VoiceThreshold)
    }

    /// Out-of-range values are clamped; everything but the threshold is whole.
    pub fn set(self, settings: &mut WhisperSettings, value: f64) {
        let (min, max, _) = self.range();
        let value = value.clamp(min, max);
        let value = if self.is_fractional() {
            value
        } else {
            value.round()
        };

        let slot = match self {
            Field::SpeechThreshold => &mut settings.speech_threshold,
            Field::VoiceThreshold => &mut settings.voice_threshold,
            Field::EndOfSpeech => &mut settings.end_of_speech_ms,
            Field::MinSpeech => &mut settings.min_speech_ms,
            Field::MaxSegment => &mut settings.max_segment_ms,
            Field::Threads => &mut settings.threads,
            Field::BeamSize => &mut settings.beam_size,
        };

        *slot = value;
    }
}

fn max_threads() -> f64 {
    std::thread::available_parallelism()
        .map(|threads| threads.get() as f64)
        .unwrap_or(4.)
}

impl WhisperSettings {
    /// Missing or invalid fields keep their default, so older preference files still load.
    pub fn from_json(saved: &Value) -> Self {
        let mut settings = Self::default();

        for field in Field::ALL {
            if let Some(value) = saved[field.key()].as_f64() {
                field.set(&mut settings, value);
            }
        }

        settings
    }

    pub fn to_json(&self) -> Value {
        let mut saved = json!({});

        for field in Field::ALL {
            saved[field.key()] = json!(field.get(self));
        }

        saved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survives_a_round_trip_and_clamps_bad_values() {
        let mut settings = WhisperSettings::default();
        Field::EndOfSpeech.set(&mut settings, 950.4);

        assert_eq!(WhisperSettings::from_json(&settings.to_json()), settings);
        assert_eq!(settings.end_of_speech_ms, 950.);

        let saved = json!({ "speech_threshold": 9.0, "beam_size": "five" });
        let loaded = WhisperSettings::from_json(&saved);

        assert_eq!(loaded.speech_threshold, 0.2);
        assert_eq!(loaded.beam_size, WhisperSettings::default().beam_size);
    }
}
