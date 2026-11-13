use gpui_kit::*;

use crate::language::furigana::Segment;
use crate::translate_mode::translate::Translation;

pub const TITLE_TEXT_SIZE: f32 = 30.;
pub const RESULT_TEXT_SIZE: f32 = 28.;
pub const SECONDARY_TEXT_SIZE: f32 = 17.;
const READING_TEXT_SIZE: f32 = 15.;

pub fn panel(border_color: Hsla, radius: Pixels) -> Div {
    div()
        .flex_1()
        .h_full()
        .min_w_0()
        .p_4()
        .border_1()
        .border_color(border_color)
        .rounded(radius)
}

pub fn render_text(text: &Translation, reading_color: Hsla) -> AnyElement {
    match text {
        Translation::Spanish(spanish) => div()
            .text_size(px(RESULT_TEXT_SIZE))
            .child(spanish.clone())
            .into_any_element(),

        Translation::Japanese(lines) => div()
            .flex()
            .flex_col()
            .gap_3()
            .text_size(px(RESULT_TEXT_SIZE))
            .children(
                lines
                    .iter()
                    .map(|line| render_furigana_line(line, reading_color)),
            )
            .into_any_element(),
    }
}

fn render_furigana_line(line: &[Segment], reading_color: Hsla) -> Div {
    let segments = line.iter().map(|segment| {
        let reading = segment.ruby.clone().unwrap_or_default();
        let reading_row = div()
            .h(px(READING_TEXT_SIZE + 4.))
            .text_size(px(READING_TEXT_SIZE))
            .text_color(reading_color)
            .child(reading);

        div()
            .flex()
            .flex_col()
            .items_center()
            .child(reading_row)
            .child(segment.text.clone())
    });

    div()
        .flex()
        .flex_wrap()
        .children(segments)
}
