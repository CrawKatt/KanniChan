use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Theme};
use gpui_kit::*;

use crate::app::widgets::TITLE_TEXT_SIZE;
use crate::settings::view::SettingsView;
use crate::setup_mode::components::Component;
use crate::setup_mode::view::{SetupFinished, SetupView};
use crate::setup_mode::{first_run, ollama};
use crate::stream_mode::streamer_view::StreamerView;
use crate::translate_mode::view::TranslateView;
use crate::voice::voicevox;

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Welcome,
    Translate,
    Streamer,
    Settings,
}

pub struct Root {
    translate_view: Entity<TranslateView>,
    streamer_view: Entity<StreamerView>,
    settings_view: Entity<SettingsView>,
    setup_view: Entity<SetupView>,
    screen: Screen,
}

impl Root {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Theme::sync_system_appearance(Some(window), cx);

        cx.observe_window_appearance(window, |_, window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
        })
        .detach();

        cx.on_app_quit(|_, _| {
            ollama::stop_started_server();
            voicevox::stop_started_engine();

            async {}
        })
        .detach();

        let setup_view = cx.new(|_| SetupView::new());

        cx.subscribe_in(&setup_view, window, |root, _, _: &SetupFinished, window, cx| {
            root.finish_welcome(window, cx);
        })
        .detach();

        let screen = if first_run::is_done() {
            Screen::Translate
        } else {
            Screen::Welcome
        };

        let root = Self {
            translate_view: cx.new(|cx| TranslateView::new(window, cx)),
            streamer_view: cx.new(|_| StreamerView::new()),
            settings_view: cx.new(|cx| SettingsView::new(setup_view.clone(), window, cx)),
            setup_view,
            screen,
        };

        root.prepare_translator(window, cx);

        root
    }

    fn prepare_translator(&self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let essentials_missing = cx
                .background_spawn(async {
                    let _ = ollama::ensure_running();

                    Component::ALL
                        .iter()
                        .any(|component| component.is_essential() && !component.is_installed())
                })
                .await;

            this.update_in(cx, |root, window, cx| {
                if essentials_missing {
                    root.show(Screen::Welcome, window, cx);
                } else if root.screen == Screen::Welcome {
                    root.setup_view
                        .update(cx, |setup, cx| setup.check_all(cx));
                }
            })
        })
        .detach();
    }

    fn finish_welcome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = first_run::mark_done();

        self.show(Screen::Translate, window, cx);
    }

    fn show(&mut self, screen: Screen, window: &mut Window, cx: &mut Context<Self>) {
        match screen {
            Screen::Settings => self
                .settings_view
                .update(cx, |settings, cx| settings.refresh(window, cx)),

            Screen::Translate => self
                .translate_view
                .update(cx, |view, cx| view.check_voicevox(cx)),

            Screen::Welcome | Screen::Streamer => {}
        }

        if matches!(screen, Screen::Welcome | Screen::Settings) {
            let is_welcome = screen == Screen::Welcome;

            self.setup_view.update(cx, |setup, cx| {
                setup.set_welcome(is_welcome, cx);
                setup.check_all(cx);
            });
        }

        self.screen = screen;
        cx.notify();
    }

    fn screen_button(
        &self,
        id: &'static str,
        label: String,
        screen: Screen,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .ghost()
            .label(label)
            .on_click(cx.listener(move |this, _, window, cx| this.show(screen, window, cx)))
    }

    fn render_header(&self, cx: &mut Context<Self>) -> Div {
        let title = div().text_size(px(TITLE_TEXT_SIZE)).child("KanniChan");

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child(title);

        let controls = div().flex().items_center().gap_3();

        let controls = match self.screen {
            Screen::Welcome => return header,

            Screen::Translate => {
                let direction_controls = self
                    .translate_view
                    .update(cx, |view, cx| view.render_direction_controls(cx));

                controls
                    .child(direction_controls)
                    .child(self.screen_button("streamer", t!("header.streamer").to_string(), Screen::Streamer, cx))
                    .child(self.screen_button("settings", t!("header.settings").to_string(), Screen::Settings, cx))
            }

            Screen::Streamer | Screen::Settings => controls.child(self.screen_button(
                "back",
                t!("header.back").to_string(),
                Screen::Translate,
                cx,
            )),
        };

        header.child(controls)
    }
}

impl Render for Root {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let page = div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .px_6()
            .py_5()
            .bg(theme.background)
            .text_color(theme.foreground)
            .font_family(theme.font_family.clone());

        let content = match self.screen {
            Screen::Welcome => self.setup_view.clone().into_any_element(),
            Screen::Translate => self.translate_view.clone().into_any_element(),
            Screen::Streamer => self.streamer_view.clone().into_any_element(),
            Screen::Settings => self.settings_view.clone().into_any_element(),
        };

        page.child(self.render_header(cx)).child(content)
    }
}
