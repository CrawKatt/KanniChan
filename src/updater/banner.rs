use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::progress::Progress;
use gpui_kit::*;

use crate::app::widgets::SECONDARY_TEXT_SIZE;
use crate::updater::release::{self, Release};

enum InstallEvent {
    Progress(f32),
    Finished(Result<(), String>),
}

pub struct UpdateBanner {
    release: Option<Release>,
    progress: Option<f32>,
    error: Option<String>,
}

impl UpdateBanner {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            let release = cx.background_spawn(async { release::check() }).await;

            let _ = this.update(cx, |banner, cx| {
                banner.release = release;
                cx.notify();
            });
        })
        .detach();

        Self {
            release: None,
            progress: None,
            error: None,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.release.is_some()
    }

    fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.release = None;
        cx.notify();
    }

    fn install(&mut self, cx: &mut Context<Self>) {
        let Some(release) = self.release.clone() else {
            return;
        };

        self.progress = Some(0.);
        self.error = None;
        cx.notify();

        let (sender, events) = mpsc::channel();

        thread::spawn(move || {
            let progress_sender = sender.clone();

            let result = release::install(&release, &mut |fraction| {
                let _ = progress_sender.send(InstallEvent::Progress(fraction));
            });

            let _ = sender.send(InstallEvent::Finished(result));
        });

        self.receive_events(events, cx);
    }

    fn receive_events(&mut self, events: Receiver<InstallEvent>, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(150))
                    .await;

                let mut finished = None;
                let mut progress = None;

                loop {
                    match events.try_recv() {
                        Ok(InstallEvent::Progress(fraction)) => progress = Some(fraction),
                        Ok(InstallEvent::Finished(result)) => finished = Some(result),
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => break,
                    }
                }

                let still_open = this.update(cx, |banner, cx| {
                    if progress.is_some() {
                        banner.progress = progress;
                    }

                    match &finished {
                        Some(Ok(())) => cx.quit(),

                        Some(Err(error)) => {
                            banner.progress = None;
                            banner.error = Some(error.clone());
                        }

                        None => {}
                    }

                    cx.notify();
                });

                if finished.is_some() || still_open.is_err() {
                    break;
                }
            }
        })
        .detach();
    }
}

impl Render for UpdateBanner {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(release) = &self.release else {
            return div();
        };

        let theme = cx.theme();
        let is_installing = self.progress.is_some();

        let message = match self.progress {
            Some(fraction) => t!("update.downloading", percent = format!("{:.0}", fraction * 100.)).to_string(),
            None => t!("update.available", version = release.version).to_string(),
        };

        let status = div()
            .flex()
            .items_center()
            .gap_3()
            .min_w_0()
            .child(message)
            .children(self.progress.map(|fraction| {
                div()
                    .w(px(160.))
                    .child(Progress::new("update-progress").value(fraction * 100.))
            }))
            .children(
                self.error
                    .clone()
                    .map(|error| div().text_color(theme.danger).child(error)),
            );

        let page_url = release.page_url.clone();

        let notes_button = Button::new("update-notes")
            .ghost()
            .label(t!("update.notes").to_string())
            .on_click(move |_, _, cx| cx.open_url(&page_url));

        let later_button = (!is_installing).then(|| {
            Button::new("update-later")
                .ghost()
                .label(t!("update.later").to_string())
                .on_click(cx.listener(|banner, _, _, cx| banner.dismiss(cx)))
        });

        let install_button = (release.can_install() && !is_installing).then(|| {
            Button::new("update-install")
                .primary()
                .label(t!("update.install").to_string())
                .on_click(cx.listener(|banner, _, _, cx| banner.install(cx)))
        });

        let actions = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_shrink_0()
            .child(notes_button)
            .children(later_button)
            .children(install_button);

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .px_4()
            .py_2()
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .text_size(px(SECONDARY_TEXT_SIZE))
            .child(status)
            .child(actions)
    }
}
