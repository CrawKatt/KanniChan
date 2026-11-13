use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::select::{Select, SelectEvent, SelectItem, SelectState};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::app::preferences::Preferences;
use crate::app::widgets::SECONDARY_TEXT_SIZE;
use crate::voice::voicevox::{self, Voice};
use crate::voice::tts;

#[derive(Clone)]
struct VoiceOption(Voice);

impl SelectItem for VoiceOption {
    type Value = u32;

    fn title(&self) -> SharedString {
        format!("{}（{}）", self.0.character, self.0.style).into()
    }

    fn value(&self) -> &u32 {
        &self.0.speaker_id
    }
}

type VoiceSelect = SelectState<Vec<VoiceOption>>;

enum Status {
    Loading,
    NotInstalled,
    NotRunning,
    Starting,
    Ready,
    Failed(String),
}

enum Lookup {
    NotInstalled,
    NotRunning,
    Voices(Result<Vec<Voice>, String>),
}

pub struct VoicePicker {
    select: Entity<VoiceSelect>,
    status: Status,
    is_playing: bool,
    error: Option<String>,
}

impl VoicePicker {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let select = cx.new(|cx| VoiceSelect::new(Vec::new(), None, window, cx).searchable(true));

        cx.subscribe(&select, |picker, _, event: &SelectEvent<Vec<VoiceOption>>, cx| {
            let SelectEvent::Confirm(Some(speaker)) = event else {
                return;
            };

            let mut preferences = Preferences::load();
            preferences.voicevox_speaker = *speaker;

            picker.error = preferences.save().err();
            cx.notify();
        })
        .detach();

        Self {
            select,
            status: Status::Loading,
            is_playing: false,
            error: None,
        }
    }

    pub fn load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.status = Status::Loading;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let lookup = cx
                .background_spawn(async {
                    if !voicevox::is_installed() {
                        Lookup::NotInstalled
                    } else if !voicevox::is_running() {
                        Lookup::NotRunning
                    } else {
                        Lookup::Voices(voicevox::voices())
                    }
                })
                .await;

            this.update_in(cx, |picker, window, cx| {
                picker.status = match lookup {
                    Lookup::NotInstalled => Status::NotInstalled,
                    Lookup::NotRunning => Status::NotRunning,
                    Lookup::Voices(Err(error)) => Status::Failed(error),

                    Lookup::Voices(Ok(voices)) => {
                        let options = voices.into_iter().map(VoiceOption).collect();

                        picker.select.update(cx, |select, cx| {
                            select.set_items(options, window, cx);
                            select.set_selected_value(&voicevox::selected_speaker(), window, cx);
                        });

                        Status::Ready
                    }
                };

                cx.notify();
            })
        })
        .detach();
    }

    fn start_engine(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.status = Status::Starting;
        self.error = None;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let started = cx.background_spawn(async { voicevox::ensure_running() }).await;

            this.update_in(cx, |picker, window, cx| match started {
                Ok(()) => picker.load(window, cx),

                Err(error) => {
                    picker.status = Status::Failed(error);
                    cx.notify();
                }
            })
        })
        .detach();
    }

    fn play_sample(&mut self, cx: &mut Context<Self>) {
        if self.is_playing {
            return;
        }

        let speaker = self
            .select
            .read(cx)
            .selected_value()
            .copied()
            .unwrap_or_else(voicevox::selected_speaker);

        self.is_playing = true;
        self.error = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let played = cx
                .background_spawn(async move { tts::speak_japanese_sample(speaker) })
                .await;

            this.update(cx, |picker, cx| {
                picker.error = played.err();
                picker.is_playing = false;
                cx.notify();
            })
        })
        .detach();
    }

    fn start_button(&self, cx: &mut Context<Self>) -> Button {
        let is_starting = matches!(self.status, Status::Starting);

        let label = if is_starting {
            t!("translate.starting_voicevox")
        } else {
            t!("translate.start_voicevox")
        };

        Button::new("start-voicevox")
            .ghost()
            .label(label.to_string())
            .loading(is_starting)
            .on_click(cx.listener(|picker, _, window, cx| picker.start_engine(window, cx)))
    }
}

impl Render for VoicePicker {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_color = cx.theme().muted_foreground;
        let danger_color = cx.theme().danger;

        let muted = |text: String| div().text_color(muted_color).child(text);
        let danger = |text: String| div().text_color(danger_color).child(text);

        let body = match &self.status {
            Status::Loading => muted(t!("settings.voice_loading").to_string()),
            Status::NotInstalled => muted(t!("settings.voice_not_installed").to_string()),

            Status::NotRunning | Status::Starting => div()
                .flex()
                .items_center()
                .gap_3()
                .child(muted(t!("settings.voice_not_running").to_string()))
                .child(self.start_button(cx)),

            Status::Failed(error) => div()
                .flex()
                .items_center()
                .gap_3()
                .child(danger(error.clone()))
                .child(self.start_button(cx)),

            Status::Ready => {
                let select = Select::new(&self.select)
                    .w(px(360.))
                    .placeholder(t!("settings.voice_placeholder").to_string())
                    .search_placeholder(t!("settings.voice_search").to_string());

                let sample_button = Button::new("voice-sample")
                    .ghost()
                    .label(t!("settings.voice_sample").to_string())
                    .loading(self.is_playing)
                    .on_click(cx.listener(|picker, _, _, cx| picker.play_sample(cx)));

                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(select)
                    .child(sample_button)
            }
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(px(SECONDARY_TEXT_SIZE))
            .child(body)
            .children(self.error.clone().map(danger))
    }
}
