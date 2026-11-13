use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _};
use gpui_kit::*;

use crate::app::widgets::{RESULT_TEXT_SIZE, SECONDARY_TEXT_SIZE, panel, render_text};
use crate::language::furigana;
use crate::translate_mode::translate::{self, Direction, Translation};
use crate::voice::{tts, voicevox};

#[derive(Clone, Copy, PartialEq)]
enum VoicevoxStatus {
    Checking,
    Stopped,
    Starting,
    Running,
}

pub struct TranslateView {
    input: Entity<TextareaState>,
    direction: Direction,
    result: Option<Result<Translation, String>>,
    is_translating: bool,
    is_speaking: bool,
    is_copied: bool,
    speech_error: Option<String>,
    voicevox_status: VoicevoxStatus,
}

impl TranslateView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            TextareaState::new(window, cx).placeholder(t!("translate.placeholder").to_string())
        });

        cx.subscribe(&input, |this, _, event: &InputEvent, cx| match event {
            InputEvent::PressEnter { secondary: true, .. } => this.start_translation(cx),

            InputEvent::Change => this.follow_pasted_japanese(cx),

            _ => {}
        })
        .detach();

        input.update(cx, |input, cx| input.focus(window, cx));

        let mut view = Self {
            input,
            direction: Direction::SpanishToJapanese,
            result: None,
            is_translating: false,
            is_speaking: false,
            is_copied: false,
            speech_error: None,
            voicevox_status: VoicevoxStatus::Checking,
        };

        view.check_voicevox(cx);

        view
    }

    pub fn check_voicevox(&mut self, cx: &mut Context<Self>) {
        self.voicevox_status = VoicevoxStatus::Checking;

        cx.spawn(async move |this, cx| {
            let is_running = cx
                .background_spawn(async { voicevox::is_running() })
                .await;

            this.update(cx, |this, cx| {
                this.voicevox_status = if is_running {
                    VoicevoxStatus::Running
                } else {
                    VoicevoxStatus::Stopped
                };
                cx.notify();
            })
        })
        .detach();
    }

    fn start_voicevox(&mut self, cx: &mut Context<Self>) {
        if self.voicevox_status != VoicevoxStatus::Stopped {
            return;
        }

        self.voicevox_status = VoicevoxStatus::Starting;
        self.speech_error = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let started = cx.background_spawn(async { voicevox::ensure_running() }).await;

            this.update(cx, |this, cx| {
                match started {
                    Ok(()) => this.voicevox_status = VoicevoxStatus::Running,

                    Err(error) => {
                        this.speech_error = Some(error);
                        this.voicevox_status = VoicevoxStatus::Stopped;
                    }
                }

                cx.notify();
            })
        })
        .detach();
    }

    fn follow_pasted_japanese(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value();

        if furigana::has_japanese(&text) && self.direction != Direction::JapaneseToSpanish {
            self.direction = Direction::JapaneseToSpanish;
            cx.notify();
        }
    }

    fn start_translation(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value().to_string();

        if text.trim().is_empty() || self.is_translating {
            return;
        }

        self.is_translating = true;
        self.speech_error = None;
        cx.notify();

        let direction = self.direction;

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { translate::translate(&text, direction) })
                .await;

            this.update(cx, |this, cx| {
                this.result = Some(result);
                this.is_copied = false;
                this.is_translating = false;
                cx.notify();
            })
        })
        .detach();
    }

    fn start_speaking(&mut self, cx: &mut Context<Self>) {
        let Some(Ok(translation)) = &self.result else {
            return;
        };

        if self.is_speaking {
            return;
        }

        let translation = translation.clone();

        self.is_speaking = true;
        self.speech_error = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { tts::speak(&translation) })
                .await;

            this.update(cx, |this, cx| {
                if outcome.is_err() {
                    this.check_voicevox(cx);
                }

                this.speech_error = outcome.err();
                this.is_speaking = false;
                cx.notify();
            })
        })
        .detach();
    }

    fn copy_translation(&mut self, cx: &mut Context<Self>) {
        let Some(Ok(translation)) = &self.result else {
            return;
        };

        cx.write_to_clipboard(ClipboardItem::new_string(translation.plain_text()));

        self.is_copied = true;
        cx.notify();
    }

    fn swap_direction(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(Ok(translation)) = self.result.take() {
            let text = translation.plain_text();

            self.input
                .update(cx, |input, cx| input.set_value(text, window, cx));
        }

        self.direction = self.direction.flipped();
        cx.notify();
    }

    pub fn render_direction_controls(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();

        let direction = div()
            .text_size(px(SECONDARY_TEXT_SIZE))
            .text_color(theme.muted_foreground)
            .child(self.direction.label());

        let swap_button = Button::new("swap")
            .ghost()
            .label(t!("translate.swap").to_string())
            .on_click(cx.listener(|this, _, window, cx| this.swap_direction(window, cx)));

        div()
            .flex()
            .items_center()
            .gap_3()
            .child(direction)
            .child(swap_button)
    }

    fn render_result(&self, cx: &App) -> AnyElement {
        let theme = cx.theme();

        match &self.result {
            None => div()
                .text_size(px(RESULT_TEXT_SIZE))
                .text_color(theme.muted_foreground)
                .child(t!("translate.empty").to_string())
                .into_any_element(),

            Some(Ok(translation)) => render_text(translation, theme.muted_foreground),

            Some(Err(error)) => div()
                .text_size(px(SECONDARY_TEXT_SIZE))
                .text_color(theme.danger)
                .child(error.clone())
                .into_any_element(),
        }
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();

        let shortcut_hint = div()
            .text_size(px(SECONDARY_TEXT_SIZE))
            .text_color(theme.muted_foreground)
            .child("Ctrl + Enter");

        let translate_button = Button::new("translate")
            .primary()
            .label("翻訳する")
            .loading(self.is_translating)
            .on_click(cx.listener(|this, _, _, cx| this.start_translation(cx)));

        let has_translation = matches!(self.result, Some(Ok(_)));

        let listen_button = Button::new("listen")
            .ghost()
            .label(t!("translate.listen").to_string())
            .loading(self.is_speaking)
            .disabled(!has_translation)
            .on_click(cx.listener(|this, _, _, cx| this.start_speaking(cx)));

        let copy_label = if self.is_copied {
            t!("translate.copied")
        } else {
            t!("translate.copy")
        };

        let copy_button = Button::new("copy")
            .ghost()
            .label(copy_label.to_string())
            .disabled(!has_translation)
            .on_click(cx.listener(|this, _, _, cx| this.copy_translation(cx)));

        let speech_error = self.speech_error.clone().map(|error| {
            div()
                .text_size(px(SECONDARY_TEXT_SIZE))
                .text_color(theme.danger)
                .child(error)
        });

        let voicevox_button = match self.voicevox_status {
            VoicevoxStatus::Stopped => Some(
                Button::new("start-voicevox")
                    .ghost()
                    .label(t!("translate.start_voicevox").to_string())
                    .on_click(cx.listener(|this, _, _, cx| this.start_voicevox(cx))),
            ),

            VoicevoxStatus::Starting => Some(
                Button::new("start-voicevox")
                    .ghost()
                    .label(t!("translate.starting_voicevox").to_string())
                    .loading(true),
            ),

            VoicevoxStatus::Checking | VoicevoxStatus::Running => None,
        };

        let speech_controls = div()
            .flex()
            .items_center()
            .gap_3()
            .min_w_0()
            .child(copy_button)
            .child(listen_button)
            .children(voicevox_button)
            .children(speech_error);

        let translate_controls = div()
            .flex()
            .items_center()
            .gap_3()
            .child(shortcut_hint)
            .child(translate_button);

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(speech_controls)
            .child(translate_controls)
    }
}

impl Render for TranslateView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let input_is_focused = self.input.read(cx).focus_handle(cx).is_focused(window);

        let source_border = if input_is_focused {
            theme.ring
        } else {
            theme.border
        };

        let source_text = Textarea::new(&self.input)
            .appearance(false)
            .h_full()
            .text_size(px(RESULT_TEXT_SIZE));

        let source_panel = panel(source_border, theme.radius).child(source_text);

        let result_panel = panel(theme.border, theme.radius)
            .id("result")
            .overflow_y_scroll()
            .child(self.render_result(cx));

        let panels = div()
            .flex_1()
            .min_h_0()
            .flex()
            .gap_4()
            .child(source_panel)
            .child(result_panel);

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap_4()
            .child(panels)
            .child(self.render_footer(cx))
    }
}
