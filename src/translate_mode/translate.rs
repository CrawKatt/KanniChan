use serde_json::{Value, json};

use crate::language::furigana::{self, Segment};
use crate::translate_mode::model::TranslationModel;

const DEFAULT_URL: &str = "http://127.0.0.1:11434/v1/chat/completions";

#[derive(Clone)]
pub enum Translation {
    Spanish(String),
    Japanese(Vec<Vec<Segment>>),
}

impl Translation {
    pub fn plain_text(&self) -> String {
        match self {
            Translation::Spanish(spanish) => spanish.clone(),

            Translation::Japanese(lines) => {
                let lines: Vec<String> = lines
                    .iter()
                    .map(|line| line.iter().map(|segment| segment.text.as_str()).collect())
                    .collect();

                lines.join("\n")
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    SpanishToJapanese,
    JapaneseToSpanish,
}

impl Direction {
    pub fn flipped(self) -> Self {
        match self {
            Direction::SpanishToJapanese => Direction::JapaneseToSpanish,
            Direction::JapaneseToSpanish => Direction::SpanishToJapanese,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Direction::SpanishToJapanese => "Español → 日本語",
            Direction::JapaneseToSpanish => "日本語 → Español",
        }
    }
}

pub fn translate(text: &str, direction: Direction) -> Result<Translation, String> {
    match direction {
        Direction::JapaneseToSpanish => {
            let spanish = ask_model(text, "Spanish")?;

            Ok(Translation::Spanish(spanish))
        }

        Direction::SpanishToJapanese => {
            let japanese = ask_model(text, "Japanese")?;
            let lines = japanese.lines().map(furigana::annotate).collect();

            Ok(Translation::Japanese(lines))
        }
    }
}

pub fn model_name() -> String {
    std::env::var("LLM_MODEL").unwrap_or_else(|_| TranslationModel::selected().ollama_name().into())
}

fn ask_model(text: &str, target_language: &str) -> Result<String, String> {
    let url = std::env::var("LLM_URL").unwrap_or_else(|_| DEFAULT_URL.into());
    let model = model_name();

    let prompt = format!(
        "Translate the following segment into {target_language}, \
         without additional explanation.\n\n{text}"
    );

    let request = json!({
        "model": model,
        "messages": [
            { "role": "user", "content": prompt }
        ],
        "temperature": 0.7,
        "top_p": 0.6,
        "top_k": 20,
        "repeat_penalty": 1.05,
    });

    let response: Value = ureq::post(&url)
        .send_json(&request)
        .and_then(|mut response| response.body_mut().read_json())
        .map_err(|error| {
            t!("error.model_unreachable", url = url, error = error).to_string()
        })?;

    let answer = response["choices"][0]["message"]["content"].as_str();

    match answer {
        Some(answer) => Ok(answer.trim().to_string()),
        None => Err(t!("error.model_unexpected", response = response).to_string()),
    }
}
