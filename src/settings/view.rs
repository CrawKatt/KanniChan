use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::*;

use crate::app::preferences::Preferences;
use crate::app::widgets::SECONDARY_TEXT_SIZE;
use crate::settings::subtitle_section::SubtitleSection;
use crate::settings::voice_picker::VoicePicker;
use crate::settings::whisper_section::WhisperSection;
use crate::setup_mode::view::SetupView;
use crate::translate_mode::model::TranslationModel;

const SECTION_TEXT_SIZE: f32 = 22.;

pub struct SettingsView {
    setup_view: Entity<SetupView>,
    voice_picker: Entity<VoicePicker>,
    whisper_section: Entity<WhisperSection>,
    subtitle_section: Entity<SubtitleSection>,
    model: TranslationModel,
    error: Option<String>,
}

impl SettingsView {
    pub fn new(setup_view: Entity<SetupView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            setup_view,
            voice_picker: cx.new(|cx| VoicePicker::new(window, cx)),
            whisper_section: cx.new(|cx| WhisperSection::new(window, cx)),
            subtitle_section: cx.new(|cx| SubtitleSection::new(window, cx)),
            model: TranslationModel::selected(),
            error: None,
        }
    }

    pub fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.voice_picker
            .update(cx, |picker, cx| picker.load(window, cx));
    }

    fn select_model(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(model) = TranslationModel::ALL.get(index).copied() else {
            return;
        };

        let mut preferences = Preferences::load();
        preferences.translation_model = model;

        self.error = preferences.save().err();
        self.model = model;
        self.setup_view.update(cx, |setup, cx| setup.check_all(cx));

        cx.notify();
    }

    fn model_description(model: TranslationModel) -> String {
        let summary = match model {
            TranslationModel::Large => t!("settings.model_large"),
            TranslationModel::Small => t!("settings.model_small"),
        };

        format!(
            "{} · {} · {summary}",
            model.display_name(),
            model.download_size()
        )
    }

    fn section_title(title: String) -> Div {
        div().text_size(px(SECTION_TEXT_SIZE)).child(title)
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let selected_index = TranslationModel::ALL
            .iter()
            .position(|model| *model == self.model);

        let model_options = TranslationModel::ALL.iter().map(|model| {
            Radio::new(SharedString::from(model.id())).label(Self::model_description(*model))
        });

        let model_picker = RadioGroup::vertical("translation-model")
            .selected_index(selected_index)
            .children(model_options)
            .on_change(cx.listener(|this, index: &usize, _, cx| this.select_model(*index, cx)));

        let error = self.error.clone().map(|error| {
            div()
                .text_size(px(SECONDARY_TEXT_SIZE))
                .text_color(theme.danger)
                .child(error)
        });

        let model_section = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(Self::section_title(t!("settings.model_title").to_string()))
            .child(div().text_size(px(SECONDARY_TEXT_SIZE)).child(model_picker))
            .children(error);

        let voice_section = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(Self::section_title(t!("settings.voice_title").to_string()))
            .child(self.voice_picker.clone());

        let whisper_section = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(Self::section_title(t!("settings.whisper_title").to_string()))
            .child(self.whisper_section.clone());

        let subtitle_section = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(Self::section_title(t!("settings.subtitle_title").to_string()))
            .child(self.subtitle_section.clone());

        let components_section = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(Self::section_title(t!("settings.components_title").to_string()))
            .child(self.setup_view.clone());

        let sections = [model_section, voice_section, whisper_section, subtitle_section, components_section]
            .map(|section| section.flex_shrink_0());

        div()
            .id("settings")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .pr_4()
            .flex()
            .flex_col()
            .gap_8()
            .children(sections)
    }
}
