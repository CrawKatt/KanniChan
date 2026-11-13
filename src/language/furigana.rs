use std::borrow::Cow;
use std::sync::LazyLock;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub text: String,
    pub ruby: Option<String>,
}

impl Segment {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ruby: None,
        }
    }

    fn with_reading(text: impl Into<String>, reading: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ruby: Some(reading.into()),
        }
    }
}

static SEGMENTER: LazyLock<Segmenter> = LazyLock::new(|| {
    let dictionary = load_dictionary("embedded://ipadic").expect("embedded ipadic dictionary");

    Segmenter::new(Mode::Normal, dictionary, None)
});

const READING_FIELD: usize = 7;

fn is_kanji(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '々')
}

fn is_kana(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{30FF}')
}

pub fn has_japanese(text: &str) -> bool {
    text.chars().any(|c| is_kanji(c) || is_kana(c))
}

fn katakana_to_hiragana(c: char) -> char {
    // Katakana and hiragana blocks are parallel, 0x60 code points apart.
    match c {
        '\u{30A1}'..='\u{30F6}' => char::from_u32(c as u32 - 0x60).unwrap(),
        _ => c,
    }
}

fn to_hiragana(text: &str) -> String {
    text.chars().map(katakana_to_hiragana).collect()
}

pub fn annotate(line: &str) -> Vec<Segment> {
    let Ok(tokens) = SEGMENTER.segment(Cow::Borrowed(line)) else {
        return vec![Segment::plain(line)];
    };

    let mut segments = Vec::new();

    for mut token in tokens {
        let surface = token.surface.to_string();

        let reading = token
            .details()
            .get(READING_FIELD)
            .filter(|reading| **reading != "*")
            .map(|reading| to_hiragana(reading));

        match reading {
            Some(reading) if surface.chars().any(is_kanji) => {
                segments.extend(split_okurigana(&surface, &reading));
            }
            _ => segments.push(Segment::plain(surface)),
        }
    }

    segments
}

fn split_okurigana(surface: &str, reading: &str) -> Vec<Segment> {
    let surface: Vec<char> = surface.chars().collect();
    let reading: Vec<char> = reading.chars().collect();

    let same_kana = |written: char, read: char| {
        !is_kanji(written) && katakana_to_hiragana(written) == read
    };

    let mut prefix_len = 0;
    while prefix_len < surface.len()
        && prefix_len < reading.len()
        && same_kana(surface[prefix_len], reading[prefix_len])
    {
        prefix_len += 1;
    }

    let mut suffix_len = 0;
    while suffix_len < surface.len() - prefix_len
        && suffix_len < reading.len() - prefix_len
        && same_kana(
            surface[surface.len() - 1 - suffix_len],
            reading[reading.len() - 1 - suffix_len],
        )
    {
        suffix_len += 1;
    }

    let prefix: String = surface[..prefix_len].iter().collect();
    let kanji_part: String = surface[prefix_len..surface.len() - suffix_len].iter().collect();
    let kanji_reading: String = reading[prefix_len..reading.len() - suffix_len].iter().collect();
    let suffix: String = surface[surface.len() - suffix_len..].iter().collect();

    let mut segments = Vec::new();

    if !prefix.is_empty() {
        segments.push(Segment::plain(prefix));
    }

    segments.push(Segment::with_reading(kanji_part, kanji_reading));

    if !suffix.is_empty() {
        segments.push(Segment::plain(suffix));
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn okurigana_stays_outside_the_reading() {
        assert_eq!(
            split_okurigana("食べる", "たべる"),
            [Segment::with_reading("食", "た"), Segment::plain("べる")]
        );

        assert_eq!(
            split_okurigana("お茶", "おちゃ"),
            [Segment::plain("お"), Segment::with_reading("茶", "ちゃ")]
        );
    }

    #[test]
    fn annotates_a_sentence() {
        let segments = annotate("私は日本語を勉強します");

        assert!(segments.contains(&Segment::with_reading("私", "わたし")));
        assert!(segments.contains(&Segment::with_reading("日本語", "にほんご")));

        for segment in &segments {
            let has_kanji = segment.text.chars().any(is_kanji);
            assert_eq!(segment.ruby.is_some(), has_kanji, "{segment:?}");
        }
    }
}
