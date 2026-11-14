use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::*;

use crate::app::preferences::Preferences;
use crate::app::widgets::SECONDARY_TEXT_SIZE;
use crate::stream_mode::whisper_settings::{Field, WhisperSettings};

const DESCRIPTION_TEXT_SIZE: f32 = 15.;
const SLIDER_WIDTH: f32 = 220.;
const VALUE_WIDTH: f32 = 70.;

pub struct WhisperSection {
    sliders: Vec<(Field, Entity<SliderState>)>,
    settings: WhisperSettings,
    error: Option<String>,
}

fn format_value(field: Field, settings: &WhisperSettings) -> String {
    let value = field.get(settings);

    if field == Field::SpeechThreshold {
        format!("{value:.3}")
    } else if field.is_fractional() {
        format!("{value:.2}")
    } else {
        format!("{value:.0}")
    }
}

impl WhisperSection {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = Preferences::load().whisper;

        let sliders = Field::ALL
            .into_iter()
            .map(|field| {
                let (min, max, step) = field.range();

                // max before min: `min()` clamps against the current max, which starts at 100.
                let slider = cx.new(|_| {
                    SliderState::new()
                        .max(max as f32)
                        .min(min as f32)
                        .step(step as f32)
                        .default_value(field.get(&settings) as f32)
                });

                cx.subscribe_in(&slider, window, move |section, _, event: &SliderEvent, _, cx| {
                    match event {
                        SliderEvent::Change(value) => {
                            field.set(&mut section.settings, value.start() as f64);
                            cx.notify();
                        }

                        SliderEvent::Release(_) => section.save(cx),
                    }
                })
                .detach();

                (field, slider)
            })
            .collect();

        Self {
            sliders,
            settings,
            error: None,
        }
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let mut preferences = Preferences::load();
        preferences.whisper = self.settings;

        self.error = preferences.save().err();
        cx.notify();
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings = WhisperSettings::default();

        for (field, slider) in &self.sliders {
            let value = field.get(&self.settings) as f32;

            slider.update(cx, |slider, cx| slider.set_value(value, window, cx));
        }

        self.save(cx);
    }

    fn render_row(&self, field: Field, slider: &Entity<SliderState>, muted: Hsla) -> Div {
        let key = field.key();

        let text = div()
            .flex()
            .flex_col()
            .gap_1()
            .min_w_0()
            .child(t!(format!("whisper.{key}")).to_string())
            .child(
                div()
                    .text_size(px(DESCRIPTION_TEXT_SIZE))
                    .text_color(muted)
                    .child(t!(format!("whisper.{key}_help")).to_string()),
            );

        let control = div()
            .flex()
            .items_center()
            .gap_3()
            .flex_shrink_0()
            .child(
                div()
                    .w(px(SLIDER_WIDTH))
                    .child(Slider::new(slider).accessibility_label(t!(format!("whisper.{key}")).to_string())),
            )
            .child(
                div()
                    .w(px(VALUE_WIDTH))
                    .text_right()
                    .child(format_value(field, &self.settings)),
            );

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_6()
            .child(text)
            .child(control)
    }
}

impl Render for WhisperSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let danger = cx.theme().danger;

        let rows: Vec<Div> = self
            .sliders
            .iter()
            .map(|(field, slider)| self.render_row(*field, slider, muted))
            .collect();

        let reset_button = Button::new("whisper-reset")
            .ghost()
            .label(t!("whisper.reset").to_string())
            .on_click(cx.listener(|section, _, window, cx| section.reset(window, cx)));

        let footer = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(div().text_color(muted).child(t!("whisper.applies_on_start").to_string()))
            .child(reset_button);

        div()
            .flex()
            .flex_col()
            .gap_4()
            .text_size(px(SECONDARY_TEXT_SIZE))
            .children(rows)
            .child(footer)
            .children(
                self.error
                    .clone()
                    .map(|error| div().text_color(danger).child(error)),
            )
    }
}
