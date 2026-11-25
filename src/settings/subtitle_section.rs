use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::switch::Switch;
use gpui_kit::*;

use crate::app::preferences::Preferences;
use crate::app::widgets::SECONDARY_TEXT_SIZE;
use crate::overlay_mode::screen_overlay::{screen_id, screen_label, screens};
use crate::stream_mode::subtitle_style::{FONT_SIZE_RANGE, OUTLINE_WIDTH_RANGE, SubtitleStyle, rgb_from};

const SLIDER_WIDTH: f32 = 220.;
const VALUE_WIDTH: f32 = 70.;

pub struct SubtitleSection {
    font_size: Entity<SliderState>,
    outline_width: Entity<SliderState>,
    text_color: Entity<ColorPickerState>,
    outline_color: Entity<ColorPickerState>,
    style: SubtitleStyle,
    overlay_screen: Option<String>,
    error: Option<String>,
}

impl SubtitleSection {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let preferences = Preferences::load();
        let style = preferences.subtitle_style;

        let font_size = Self::slider(FONT_SIZE_RANGE, style.font_size, SubtitleStyle::set_font_size, window, cx);
        let outline_width = Self::slider(
            OUTLINE_WIDTH_RANGE,
            style.outline_width,
            SubtitleStyle::set_outline_width,
            window,
            cx,
        );

        let text_color = Self::color_picker(style.text_color, |style, color| style.text_color = color, window, cx);
        let outline_color =
            Self::color_picker(style.outline_color, |style, color| style.outline_color = color, window, cx);

        Self {
            font_size,
            outline_width,
            text_color,
            outline_color,
            style,
            overlay_screen: preferences.overlay_screen,
            error: None,
        }
    }

    fn slider(
        (min, max, step): (f64, f64, f64),
        value: f64,
        set: fn(&mut SubtitleStyle, f64),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<SliderState> {
        // max before min: `min()` clamps against the current max, which starts at 100.
        let slider = cx.new(|_| {
            SliderState::new()
                .max(max as f32)
                .min(min as f32)
                .step(step as f32)
                .default_value(value as f32)
        });

        cx.subscribe_in(&slider, window, move |section, _, event: &SliderEvent, _, cx| match event {
            SliderEvent::Change(value) => {
                set(&mut section.style, value.start() as f64);
                cx.notify();
            }

            SliderEvent::Release(_) => section.save(cx),
        })
        .detach();

        slider
    }

    fn color_picker(
        value: u32,
        set: fn(&mut SubtitleStyle, u32),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<ColorPickerState> {
        let picker = cx.new(|cx| ColorPickerState::new(window, cx).default_value(rgb(value)));

        cx.subscribe_in(&picker, window, move |section, _, event: &ColorPickerEvent, _, cx| {
            if let ColorPickerEvent::Change(Some(color)) = event {
                set(&mut section.style, rgb_from(*color));
                section.save(cx);
            }
        })
        .detach();

        picker
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let mut preferences = Preferences::load();
        preferences.subtitle_style = self.style;

        self.error = preferences.save().err();
        cx.notify();
    }

    fn select_screen(&mut self, index: usize, cx: &mut Context<Self>) {
        let screen = index
            .checked_sub(1)
            .and_then(|index| screens(cx).get(index).cloned())
            .and_then(|screen| screen_id(screen.as_ref()));

        let mut preferences = Preferences::load();
        preferences.overlay_screen = screen.clone();

        self.error = preferences.save().err();
        self.overlay_screen = screen;
        cx.notify();
    }

    fn render_screen_picker(&self, cx: &mut Context<Self>) -> Div {
        let screens = screens(cx);

        let selected = self
            .overlay_screen
            .as_ref()
            .and_then(|id| {
                screens
                    .iter()
                    .position(|screen| screen_id(screen.as_ref()).as_ref() == Some(id))
            })
            .map_or(0, |index| index + 1);

        let options = std::iter::once(Radio::new("screen-auto").label(t!("subtitle.screen_auto").to_string())).chain(
            screens.iter().enumerate().map(|(index, screen)| {
                Radio::new(SharedString::from(format!("screen-{index}")))
                    .label(screen_label(index + 1, screen.as_ref()))
            }),
        );

        let picker = RadioGroup::vertical("overlay-screen")
            .selected_index(Some(selected))
            .children(options)
            .on_change(cx.listener(|section, index: &usize, _, cx| section.select_screen(*index, cx)));

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(t!("subtitle.screen").to_string())
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("subtitle.screen_help").to_string()),
            )
            .child(picker)
    }

    fn set_furigana(&mut self, furigana: bool, cx: &mut Context<Self>) {
        self.style.furigana = furigana;
        self.save(cx);
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.style = SubtitleStyle::default();

        let style = self.style;

        self.font_size
            .update(cx, |slider, cx| slider.set_value(style.font_size as f32, window, cx));
        self.outline_width
            .update(cx, |slider, cx| slider.set_value(style.outline_width as f32, window, cx));
        self.text_color
            .update(cx, |picker, cx| picker.set_value(rgb(style.text_color), window, cx));
        self.outline_color
            .update(cx, |picker, cx| picker.set_value(rgb(style.outline_color), window, cx));

        self.save(cx);
    }

    fn render_row(label: String, control: impl IntoElement) -> Div {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_6()
            .child(label)
            .child(div().flex().items_center().gap_3().flex_shrink_0().child(control))
    }

    fn render_slider_row(label: String, slider: &Entity<SliderState>, value: f64) -> Div {
        let control = div()
            .flex()
            .items_center()
            .gap_3()
            .child(div().w(px(SLIDER_WIDTH)).child(Slider::new(slider).accessibility_label(label.clone())))
            .child(div().w(px(VALUE_WIDTH)).text_right().child(format!("{value:.0} px")));

        Self::render_row(label, control)
    }
}

impl Render for SubtitleSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let danger = cx.theme().danger;

        let screen_picker = self.render_screen_picker(cx);

        let font_size = Self::render_slider_row(t!("subtitle.font_size").to_string(), &self.font_size, self.style.font_size);
        let outline_width = Self::render_slider_row(
            t!("subtitle.outline_width").to_string(),
            &self.outline_width,
            self.style.outline_width,
        );

        let text_color = Self::render_row(
            t!("subtitle.text_color").to_string(),
            ColorPicker::new(&self.text_color).accessibility_label(t!("subtitle.text_color").to_string()),
        );
        let outline_color = Self::render_row(
            t!("subtitle.outline_color").to_string(),
            ColorPicker::new(&self.outline_color).accessibility_label(t!("subtitle.outline_color").to_string()),
        );

        let furigana = Self::render_row(
            t!("subtitle.furigana").to_string(),
            Switch::new("subtitle-furigana")
                .checked(self.style.furigana)
                .accessibility_label(t!("subtitle.furigana").to_string())
                .on_click(cx.listener(|section, checked: &bool, _, cx| section.set_furigana(*checked, cx))),
        );

        let reset_button = Button::new("subtitle-reset")
            .ghost()
            .label(t!("subtitle.reset").to_string())
            .on_click(cx.listener(|section, _, window, cx| section.reset(window, cx)));

        let footer = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(div().text_color(muted).child(t!("subtitle.applies_live").to_string()))
            .child(reset_button);

        div()
            .flex()
            .flex_col()
            .gap_4()
            .text_size(px(SECONDARY_TEXT_SIZE))
            .child(font_size)
            .child(outline_width)
            .child(text_color)
            .child(outline_color)
            .child(furigana)
            .child(screen_picker)
            .child(footer)
            .children(self.error.clone().map(|error| div().text_color(danger).child(error)))
    }
}
