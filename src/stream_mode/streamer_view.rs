use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::*;

use crate::app::preferences::Preferences;
use crate::stream_mode::overlay::Overlay;
use crate::overlay_mode::screen_overlay::ScreenOverlay;
use crate::stream_mode::streamer::*;
use crate::app::widgets::{RESULT_TEXT_SIZE, SECONDARY_TEXT_SIZE, panel, render_text};

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Off,
    LoadingWhisper,
    Listening,
    Processing,
}

impl Status {
    fn label(self) -> String {
        let key = match self {
            Status::Off => "streamer.status_off",
            Status::LoadingWhisper => "streamer.status_loading",
            Status::Listening => "streamer.status_listening",
            Status::Processing => "streamer.status_processing",
        };

        t!(key).to_string()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Output {
    Obs,
    Screen,
}

pub struct StreamerView {
    output: Output,
    streamer: Option<Streamer>,
    session: u64,
    status: Status,
    last_subtitle: Option<Subtitle>,
    error: Option<String>,
    overlay: Option<Overlay>,
    screen_overlay: Option<WindowHandle<ScreenOverlay>>,
}

impl StreamerView {
    pub fn new(output: Output) -> Self {
        Self {
            output,
            streamer: None,
            session: 0,
            status: Status::Off,
            last_subtitle: None,
            error: None,
            overlay: None,
            screen_overlay: None,
        }
    }

    fn start_listening(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.open_output(cx) {
            self.error = Some(error);
            cx.notify();
            return;
        }

        let (sender, events) = mpsc::channel();

        self.streamer = Some(start(sender, Preferences::load().whisper));
        self.session += 1;
        self.status = Status::LoadingWhisper;
        self.error = None;
        cx.notify();

        self.receive_events(events, self.session, cx);
    }

    fn stop_listening(&mut self, cx: &mut Context<Self>) {
        self.streamer = None;
        self.status = Status::Off;
        self.close_screen_overlay(cx);
        cx.notify();
    }

    fn open_output(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        match self.output {
            Output::Obs if self.overlay.is_none() => {
                self.overlay = Some(Overlay::start()?);
            }

            Output::Screen if self.screen_overlay.is_none() => {
                self.screen_overlay = Some(ScreenOverlay::open(cx)?);
            }

            Output::Obs | Output::Screen => {}
        }

        Ok(())
    }

    fn close_screen_overlay(&mut self, cx: &mut Context<Self>) {
        if let Some(screen_overlay) = self.screen_overlay.take() {
            let _ = screen_overlay.update(cx, |_, window, _| window.remove_window());
        }
    }

    fn receive_events(
        &mut self,
        events: Receiver<StreamerEvent>,
        session: u64,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;

                let mut received = Vec::new();
                let mut finished = false;

                loop {
                    match events.try_recv() {
                        Ok(event) => received.push(event),
                        Err(TryRecvError::Empty) => break,

                        Err(TryRecvError::Disconnected) => {
                            finished = true;
                            break;
                        }
                    }
                }

                let still_open = this.update(cx, |view, cx| {
                    if view.session == session && view.streamer.is_some() {
                        for event in received {
                            view.handle_event(event, cx);
                        }

                        cx.notify();
                    }
                });

                if finished || still_open.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn handle_event(&mut self, event: StreamerEvent, cx: &mut Context<Self>) {
        match event {
            StreamerEvent::Listening => self.status = Status::Listening,

            StreamerEvent::Processing => self.status = Status::Processing,

            StreamerEvent::Subtitle(subtitle) => {
                if let Some(overlay) = &mut self.overlay {
                    overlay.show(&subtitle);
                }

                if let Some(screen_overlay) = self.screen_overlay {
                    let shown = subtitle.clone();
                    let _ = screen_overlay.update(cx, |screen_overlay, _, cx| screen_overlay.show(shown, cx));
                }

                self.last_subtitle = Some(subtitle);
            }

            StreamerEvent::Failed(error) => {
                self.error = Some(error);
                self.streamer = None;
                self.status = Status::Off;
                self.close_screen_overlay(cx);
            }
        }
    }

    fn render_subtitle(&self, cx: &App) -> AnyElement {
        let theme = cx.theme();

        let Some(subtitle) = &self.last_subtitle else {
            return div()
                .text_size(px(RESULT_TEXT_SIZE))
                .text_color(theme.muted_foreground)
                .child(t!("streamer.empty").to_string())
                .into_any_element();
        };

        let translation = match &subtitle.translation {
            Ok(translation) => render_text(translation, theme.muted_foreground),

            Err(error) => div()
                .text_size(px(SECONDARY_TEXT_SIZE))
                .text_color(theme.danger)
                .child(error.clone())
                .into_any_element(),
        };

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(render_text(&subtitle.original, theme.muted_foreground))
            .child(translation)
            .into_any_element()
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();

        let status = div()
            .text_size(px(SECONDARY_TEXT_SIZE))
            .text_color(theme.muted_foreground)
            .child(self.status.label());

        let error = self.error.clone().map(|error| {
            div()
                .text_size(px(SECONDARY_TEXT_SIZE))
                .text_color(theme.danger)
                .child(error)
        });

        let destination = match self.output {
            Output::Obs => div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .text_size(px(SECONDARY_TEXT_SIZE))
                        .text_color(theme.muted_foreground)
                        .child(format!("OBS: {}", Overlay::url())),
                )
                .child(
                    Button::new("copy-url")
                        .ghost()
                        .label(t!("streamer.copy_url").to_string())
                        .on_click(|_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(Overlay::url()))),
                ),

            Output::Screen => div()
                .text_size(px(SECONDARY_TEXT_SIZE))
                .text_color(theme.muted_foreground)
                .child(t!("overlay.hint").to_string()),
        };

        let listen_button = if self.streamer.is_some() {
            Button::new("listen")
                .primary()
                .label(t!("streamer.stop").to_string())
                .on_click(cx.listener(|this, _, _, cx| this.stop_listening(cx)))
        } else {
            Button::new("listen")
                .primary()
                .label(t!("streamer.start").to_string())
                .on_click(cx.listener(|this, _, _, cx| this.start_listening(cx)))
        };

        let state = div()
            .flex()
            .items_center()
            .gap_3()
            .min_w_0()
            .child(status)
            .children(error);

        let actions = div()
            .flex()
            .items_center()
            .gap_3()
            .child(destination)
            .child(listen_button);

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(state)
            .child(actions)
    }
}

impl Render for StreamerView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let subtitle_panel = panel(theme.border, theme.radius)
            .id("subtitle")
            .overflow_y_scroll()
            .child(self.render_subtitle(cx));

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap_4()
            .child(subtitle_panel)
            .child(self.render_footer(cx))
    }
}
