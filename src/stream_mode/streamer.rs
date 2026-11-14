use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
    WhisperVadContext, WhisperVadContextParams, WhisperVadParams,
};

use crate::language::furigana;
use crate::app::paths::path_setting;
use crate::setup_mode::components::whisper_gpu_runtime_installed;
use crate::stream_mode::whisper_settings::WhisperSettings;
use crate::translate_mode::translate::{self, Direction, Translation};

const DEFAULT_WHISPER_MODEL: &str = "whisper/ggml-large-v3-turbo-q5_0.bin";
const DEFAULT_VAD_MODEL: &str = "whisper/ggml-silero-v6.2.0.bin";
const WHISPER_SAMPLE_RATE: u32 = 16_000;
const FRAME_MS: usize = 30;
const KNOWN_HALLUCINATIONS: &[&str] = &[
    "ご視聴ありがとうございました",
    "ご視聴ありがとうございます",
    "チャンネル登録",
    "Subtítulos realizados por la comunidad de Amara.org",
    "¡Suscríbete",
    "Gracias por ver",
];

pub struct Subtitle {
    pub original: Translation,
    pub translation: Result<Translation, String>,
}

pub enum StreamerEvent {
    Listening,
    Processing,
    Subtitle(Subtitle),
    Failed(String),
}

pub struct Streamer {
    stop: Arc<AtomicBool>,
}

impl Drop for Streamer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn start(events: Sender<StreamerEvent>, settings: WhisperSettings) -> Streamer {
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();

    thread::spawn(move || {
        if let Err(error) = listen(&events, &thread_stop, settings) {
            let _ = events.send(StreamerEvent::Failed(error));
        }
    });

    Streamer { stop }
}

pub fn whisper_model_path() -> PathBuf {
    path_setting("WHISPER_MODEL", DEFAULT_WHISPER_MODEL)
}

pub fn vad_model_path() -> PathBuf {
    path_setting("VAD_MODEL", DEFAULT_VAD_MODEL)
}

fn listen(
    events: &Sender<StreamerEvent>,
    stop: &AtomicBool,
    settings: WhisperSettings,
) -> Result<(), String> {
    let model = whisper_model_path();

    if !whisper_gpu_runtime_installed() {
        return Err(t!("error.whisper_gpu_runtime_missing").to_string());
    }

    let context = WhisperContext::new_with_params(
        &model,
        WhisperContextParameters::default(),
    )
    .map_err(|error| t!("error.whisper_load", path = model.display(), error = error).to_string())?;

    let mut whisper = context
        .create_state()
        .map_err(|error| t!("error.whisper_prepare", error = error).to_string())?;

    let vad_model = vad_model_path();
    let mut vad_params = WhisperVadContextParams::new();
    vad_params.set_n_threads(settings.threads as i32);
    vad_params.set_use_gpu(false);

    let mut vad = WhisperVadContext::new(&vad_model.to_string_lossy(), vad_params)
        .map_err(|error| t!("error.whisper_load", path = vad_model.display(), error = error).to_string())?;

    let (audio_sender, audio) = mpsc::channel();

    let (_microphone, sample_rate) = open_microphone(audio_sender)?;

    let mut segmenter = SpeechSegmenter::new(sample_rate, settings);

    let _ = events.send(StreamerEvent::Listening);

    while !stop.load(Ordering::Relaxed) {
        let samples = match audio.recv_timeout(Duration::from_millis(200)) {
            Ok(samples) => samples,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                return Err(t!("error.mic_stopped").to_string());
            }
        };

        for sentence in segmenter.push(&samples) {
            let _ = events.send(StreamerEvent::Processing);

            let audio_16k = resample(&sentence, sample_rate, WHISPER_SAMPLE_RATE);

            if !contains_voice(&mut vad, &audio_16k, settings)? {
                let _ = events.send(StreamerEvent::Listening);
                continue;
            }

            if let Some(subtitle) = transcribe(&mut whisper, &audio_16k, settings)? {
                let _ = events.send(StreamerEvent::Subtitle(subtitle));
            }

            let _ = events.send(StreamerEvent::Listening);
        }
    }

    Ok(())
}

fn open_microphone(sender: Sender<Vec<f32>>) -> Result<(cpal::Stream, u32), String> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or_else(|| t!("error.mic_missing").to_string())?;

    let config = device
        .default_input_config()
        .map_err(|error| t!("error.mic_config", error = error).to_string())?;

    let sample_rate = config.sample_rate();

    let stream = match config.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(&device, config.into(), sender),
        SampleFormat::I16 => build_stream::<i16>(&device, config.into(), sender),
        SampleFormat::I32 => build_stream::<i32>(&device, config.into(), sender),
        other => return Err(t!("error.mic_format", format = other).to_string()),
    }?;

    stream
        .play()
        .map_err(|error| t!("error.mic_start", error = error).to_string())?;

    Ok((stream, sample_rate))
}

fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    sender: Sender<Vec<f32>>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;

    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let mono = data
                    .chunks(channels)
                    .map(|frame| {
                        let sum: f32 = frame.iter().map(|sample| sample.to_sample::<f32>()).sum();
                        sum / channels as f32
                    })
                    .collect();

                let _ = sender.send(mono);
            },
            |error| eprintln!("Microphone error: {error}"),
            None,
        )
        .map_err(|error| t!("error.mic_open", error = error).to_string())
}

fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }

    let output_len = samples.len() as u64 * to_rate as u64 / from_rate as u64;
    let step = from_rate as f64 / to_rate as f64;

    (0..output_len as usize)
        .map(|index| {
            let position = index as f64 * step;
            let before = position.floor() as usize;
            let after = (before + 1).min(samples.len() - 1);
            let weight = (position - before as f64) as f32;

            samples[before] * (1.0 - weight) + samples[after] * weight
        })
        .collect()
}

struct SpeechSegmenter {
    frame_len: usize,
    pending: Vec<f32>,
    sentence: Vec<f32>,
    loud_frames: usize,
    silent_frames: usize,
    settings: WhisperSettings,
}

impl SpeechSegmenter {
    fn new(sample_rate: u32, settings: WhisperSettings) -> Self {
        Self {
            frame_len: sample_rate as usize * FRAME_MS / 1000,
            pending: Vec::new(),
            sentence: Vec::new(),
            loud_frames: 0,
            silent_frames: 0,
            settings,
        }
    }

    fn push(&mut self, samples: &[f32]) -> Vec<Vec<f32>> {
        self.pending.extend_from_slice(samples);

        let mut finished = Vec::new();

        while self.pending.len() >= self.frame_len {
            let frame: Vec<f32> = self.pending.drain(..self.frame_len).collect();

            if let Some(sentence) = self.push_frame(&frame) {
                finished.push(sentence);
            }
        }

        finished
    }

    fn push_frame(&mut self, frame: &[f32]) -> Option<Vec<f32>> {
        let is_loud = rms(frame) as f64 > self.settings.speech_threshold;

        if self.sentence.is_empty() && !is_loud {
            return None;
        }

        self.sentence.extend_from_slice(frame);

        if is_loud {
            self.loud_frames += 1;
            self.silent_frames = 0;
        } else {
            self.silent_frames += 1;
        }

        let elapsed_ms = |frames: usize| (frames * FRAME_MS) as f64;

        let paused = elapsed_ms(self.silent_frames) >= self.settings.end_of_speech_ms;
        let too_long =
            elapsed_ms(self.sentence.len() / self.frame_len) >= self.settings.max_segment_ms;

        if !paused && !too_long {
            return None;
        }

        let was_speech = elapsed_ms(self.loud_frames) >= self.settings.min_speech_ms;
        let sentence = std::mem::take(&mut self.sentence);

        self.loud_frames = 0;
        self.silent_frames = 0;

        was_speech.then_some(sentence)
    }
}

fn rms(frame: &[f32]) -> f32 {
    let energy: f32 = frame.iter().map(|sample| sample * sample).sum();

    (energy / frame.len() as f32).sqrt()
}

fn contains_voice(
    vad: &mut WhisperVadContext,
    audio: &[f32],
    settings: WhisperSettings,
) -> Result<bool, String> {
    let mut params = WhisperVadParams::new();
    params.set_threshold(settings.voice_threshold as f32);
    params.set_min_speech_duration(settings.min_speech_ms as i32);

    vad.segments_from_samples(params, audio)
        .map(|segments| segments.num_segments() > 0)
        .map_err(|error| t!("error.whisper_failed", error = error).to_string())
}

fn transcribe(
    whisper: &mut WhisperState,
    audio: &[f32],
    settings: WhisperSettings,
) -> Result<Option<Subtitle>, String> {
    let threads = settings.threads as usize;
    let whisper_error = |error: whisper_rs::WhisperError| t!("error.whisper_failed", error = error).to_string();

    whisper
        .pcm_to_mel(audio, threads)
        .map_err(whisper_error)?;

    let (_, probabilities) = whisper
        .lang_detect(0, threads)
        .map_err(whisper_error)?;

    let probability_of = |language: &str| {
        whisper_rs::get_lang_id(language)
            .and_then(|id| probabilities.get(id as usize).copied())
            .unwrap_or(0.0)
    };

    let is_japanese = probability_of("ja") > probability_of("es");

    let strategy = match settings.beam_size as i32 {
        1 => SamplingStrategy::Greedy { best_of: 1 },
        beam_size => SamplingStrategy::BeamSearch {
            beam_size,
            patience: -1.0,
        },
    };

    let mut params = FullParams::new(strategy);
    params.set_n_threads(threads as i32);
    params.set_language(Some(if is_japanese { "ja" } else { "es" }));
    params.set_no_context(true);
    params.set_suppress_blank(true);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    whisper.full(params, audio).map_err(whisper_error)?;

    let text: String = whisper
        .as_iter()
        .filter_map(|segment| segment.to_str_lossy().ok().map(|text| text.into_owned()))
        .collect();

    let text = text.trim();

    if text.is_empty() || is_known_hallucination(text) {
        return Ok(None);
    }

    let (original, direction) = if is_japanese {
        let lines = text.lines().map(furigana::annotate).collect();

        (Translation::Japanese(lines), Direction::JapaneseToSpanish)
    } else {
        (Translation::Spanish(text.to_string()), Direction::SpanishToJapanese)
    };

    let translation = translate::translate(text, direction);

    Ok(Some(Subtitle {
        original,
        translation,
    }))
}

fn is_known_hallucination(text: &str) -> bool {
    KNOWN_HALLUCINATIONS
        .iter()
        .any(|phrase| text.contains(phrase))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 16_000;

    fn tone(milliseconds: usize) -> Vec<f32> {
        let len = RATE as usize * milliseconds / 1000;

        (0..len).map(|index| (index as f32 * 0.05).sin() * 0.3).collect()
    }

    fn silence(milliseconds: usize) -> Vec<f32> {
        vec![0.0; RATE as usize * milliseconds / 1000]
    }

    #[test]
    fn a_pause_ends_the_sentence() {
        let mut segmenter = SpeechSegmenter::new(RATE, WhisperSettings::default());

        assert!(segmenter.push(&silence(500)).is_empty());
        assert!(segmenter.push(&tone(1000)).is_empty());

        let sentences = segmenter.push(&silence(1000));

        assert_eq!(sentences.len(), 1);
    }

    #[test]
    fn short_noises_are_ignored() {
        let mut segmenter = SpeechSegmenter::new(RATE, WhisperSettings::default());

        segmenter.push(&tone(100));

        assert!(segmenter.push(&silence(1000)).is_empty());
    }

    #[test]
    fn resampling_keeps_the_duration() {
        let one_second_at_48k = vec![0.5; 48_000];

        let resampled = resample(&one_second_at_48k, 48_000, RATE);

        assert_eq!(resampled.len(), RATE as usize);
        assert!(resampled.iter().all(|sample| (sample - 0.5).abs() < 1e-6));
    }
}
