use gpui_kit::{Hsla, Rgba};
use serde_json::{Value, json};

pub const FONT_SIZE_RANGE: (f64, f64, f64) = (24., 120., 2.);
pub const OUTLINE_WIDTH_RANGE: (f64, f64, f64) = (0., 10., 1.);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SubtitleStyle {
    pub font_size: f64,
    pub outline_width: f64,
    pub text_color: u32,
    pub outline_color: u32,
    pub furigana: bool,
}

impl Default for SubtitleStyle {
    fn default() -> Self {
        Self {
            font_size: 52.,
            outline_width: 2.,
            text_color: 0xFFFFFF,
            outline_color: 0x1F1B18,
            furigana: true,
        }
    }
}

pub fn rgb_from(color: Hsla) -> u32 {
    let color = Rgba::from(color);
    let channel = |value: f32| (value.clamp(0., 1.) * 255.).round() as u32;

    channel(color.r) << 16 | channel(color.g) << 8 | channel(color.b)
}

fn parse_hex(hex: &str) -> Option<u32> {
    let digits = hex.strip_prefix('#')?;

    (digits.len() == 6).then(|| u32::from_str_radix(digits, 16).ok())?
}

fn clamp((min, max, _): (f64, f64, f64), value: f64) -> f64 {
    value.clamp(min, max).round()
}

impl SubtitleStyle {
    pub fn set_font_size(&mut self, value: f64) {
        self.font_size = clamp(FONT_SIZE_RANGE, value);
    }

    pub fn set_outline_width(&mut self, value: f64) {
        self.outline_width = clamp(OUTLINE_WIDTH_RANGE, value);
    }

    pub fn from_json(saved: &Value) -> Self {
        let mut style = Self::default();

        if let Some(value) = saved["font_size"].as_f64() {
            style.set_font_size(value);
        }

        if let Some(value) = saved["outline_width"].as_f64() {
            style.set_outline_width(value);
        }

        let color = |key: &str| saved[key].as_str().and_then(parse_hex);

        style.text_color = color("text_color").unwrap_or(style.text_color);
        style.outline_color = color("outline_color").unwrap_or(style.outline_color);
        style.furigana = saved["furigana"].as_bool().unwrap_or(style.furigana);

        style
    }

    pub fn to_json(&self) -> Value {
        json!({
            "font_size": self.font_size,
            "outline_width": self.outline_width,
            "text_color": format!("#{:06X}", self.text_color),
            "outline_color": format!("#{:06X}", self.outline_color),
            "furigana": self.furigana,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survives_a_round_trip_and_rejects_bad_values() {
        let mut style = SubtitleStyle::default();
        style.set_font_size(500.);
        style.text_color = rgb_from(gpui_kit::rgb(0xE84A5F).into());
        style.furigana = false;

        let loaded = SubtitleStyle::from_json(&style.to_json());

        assert_eq!(loaded.font_size, 120.);
        assert_eq!(loaded.text_color, 0xE84A5F);
        assert!(!loaded.furigana);

        let broken = SubtitleStyle::from_json(&json!({ "outline_color": "red", "outline_width": "thick" }));

        assert_eq!(broken, SubtitleStyle::default());
    }
}
