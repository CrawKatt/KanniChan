use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::progress::Progress;
use gpui_kit::component::{ActiveTheme as _, Disableable as _};
use gpui_kit::*;

use crate::app::paths::data_dir;
use crate::app::preferences::Preferences;
use crate::app::widgets::{SECONDARY_TEXT_SIZE, TITLE_TEXT_SIZE, panel};
use crate::setup_mode::components::{Component, Step};

const NAME_TEXT_SIZE: f32 = 20.;

enum Status {
    Checking,
    Installed,
    Missing,
    Installing { label: String, fraction: Option<f32> },
    Failed(String),
}

pub struct SetupFinished;

impl EventEmitter<SetupFinished> for SetupView {}

enum InstallEvent {
    Step(Step),
    Finished(Result<(), String>),
}

pub struct SetupView {
    statuses: HashMap<Component, Status>,
    queue: VecDeque<Component>,
    is_installing: bool,
    is_welcome: bool,
    install_dir: Option<PathBuf>,
    folder_error: Option<String>,
}

impl SetupView {
    pub fn new() -> Self {
        let statuses = Component::ALL
            .iter()
            .map(|component| (*component, Status::Checking))
            .collect();

        Self {
            statuses,
            queue: VecDeque::new(),
            is_installing: false,
            is_welcome: true,
            install_dir: Preferences::load().install_dir,
            folder_error: None,
        }
    }

    pub fn set_welcome(&mut self, is_welcome: bool, cx: &mut Context<Self>) {
        self.is_welcome = is_welcome;
        cx.notify();
    }

    pub fn check_all(&mut self, cx: &mut Context<Self>) {
        if self.is_installing {
            return;
        }

        for status in self.statuses.values_mut() {
            *status = Status::Checking;
        }

        cx.notify();

        cx.spawn(async move |this, cx| {
            let results: Vec<(Component, bool)> = cx
                .background_spawn(async {
                    Component::ALL
                        .iter()
                        .map(|component| (*component, component.is_installed()))
                        .collect()
                })
                .await;

            this.update(cx, |view, cx| {
                for (component, installed) in results {
                    let status = if installed {
                        Status::Installed
                    } else {
                        Status::Missing
                    };

                    view.statuses.insert(component, status);
                }

                cx.notify();
            })
        })
        .detach();
    }

    fn choose_install_dir(&mut self, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(t!("setup.choose_folder").to_string().into()),
        });

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut folders))) = picked.await
                && let Some(folder) = folders.pop()
            {
                let _ = this.update(cx, |view, cx| view.set_install_dir(Some(folder), cx));
            }
        })
        .detach();
    }

    fn set_install_dir(&mut self, install_dir: Option<PathBuf>, cx: &mut Context<Self>) {
        let mut preferences = Preferences::load();
        preferences.install_dir = install_dir.clone();

        self.folder_error = preferences.save().err();

        if self.folder_error.is_none() {
            self.install_dir = install_dir;
            self.check_all(cx);
        }

        cx.notify();
    }

    fn render_install_folder(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let folder = self.install_dir.clone().unwrap_or_else(data_dir);

        let description = div()
            .flex()
            .flex_col()
            .gap_1()
            .min_w_0()
            .child(t!("setup.install_folder").to_string())
            .child(div().text_color(theme.muted_foreground).child(folder.display().to_string()))
            .children(
                self.folder_error
                    .clone()
                    .map(|error| div().text_color(theme.danger).child(error)),
            );

        let default_button = self.install_dir.is_some().then(|| {
            Button::new("default-folder")
                .ghost()
                .label(t!("setup.default_folder").to_string())
                .disabled(self.is_installing)
                .on_click(cx.listener(|this, _, _, cx| this.set_install_dir(None, cx)))
        });

        let choose_button = Button::new("choose-folder")
            .ghost()
            .label(t!("setup.choose_folder").to_string())
            .disabled(self.is_installing)
            .on_click(cx.listener(|this, _, _, cx| this.choose_install_dir(cx)));

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .text_size(px(SECONDARY_TEXT_SIZE))
            .child(description)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_shrink_0()
                    .children(default_button)
                    .child(choose_button),
            )
    }

    fn needs_install(&self, component: Component) -> bool {
        matches!(
            self.statuses.get(&component),
            Some(Status::Missing | Status::Failed(_))
        )
    }

    fn install(&mut self, component: Component, cx: &mut Context<Self>) {
        if !self.queue.contains(&component) {
            self.queue.push_back(component);
        }

        self.install_next(cx);
    }

    fn install_everything_missing(&mut self, cx: &mut Context<Self>) {
        for component in Component::ALL {
            if self.needs_install(component) && !self.queue.contains(&component) {
                self.queue.push_back(component);
            }
        }

        self.install_next(cx);
    }

    fn install_next(&mut self, cx: &mut Context<Self>) {
        if self.is_installing {
            return;
        }

        let Some(component) = self.queue.pop_front() else {
            return;
        };

        self.is_installing = true;
        self.statuses.insert(
            component,
            Status::Installing {
                label: t!("setup.preparing").to_string(),
                fraction: None,
            },
        );
        cx.notify();

        let (sender, events) = mpsc::channel();

        thread::spawn(move || {
            let step_sender = sender.clone();

            let result = component.install(&mut |step| {
                let _ = step_sender.send(InstallEvent::Step(step));
            });

            let _ = sender.send(InstallEvent::Finished(result));
        });

        self.receive_events(component, events, cx);
    }

    fn receive_events(
        &mut self,
        component: Component,
        events: Receiver<InstallEvent>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(150))
                    .await;

                let mut received = Vec::new();
                let mut disconnected = false;

                loop {
                    match events.try_recv() {
                        Ok(event) => received.push(event),
                        Err(TryRecvError::Empty) => break,

                        Err(TryRecvError::Disconnected) => {
                            disconnected = true;
                            break;
                        }
                    }
                }

                let mut finished = disconnected;

                let still_open = this.update(cx, |view, cx| {
                    for event in received {
                        finished |= view.handle_event(component, event);
                    }

                    if finished {
                        view.is_installing = false;
                        view.install_next(cx);
                    }

                    cx.notify();
                });

                if finished || still_open.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn handle_event(&mut self, component: Component, event: InstallEvent) -> bool {
        let status = match event {
            InstallEvent::Step(Step::Downloading(fraction)) => Status::Installing {
                label: t!("setup.downloading", percent = format!("{:.0}", fraction * 100.)).to_string(),
                fraction: Some(fraction),
            },

            InstallEvent::Step(Step::Extracting) => Status::Installing {
                label: t!("setup.extracting").to_string(),
                fraction: None,
            },

            InstallEvent::Step(Step::Starting) => Status::Installing {
                label: t!("setup.starting").to_string(),
                fraction: None,
            },

            InstallEvent::Finished(Ok(())) => Status::Installed,

            InstallEvent::Finished(Err(error)) => Status::Failed(error),
        };

        let finished = matches!(status, Status::Installed | Status::Failed(_));

        self.statuses.insert(component, status);

        finished
    }

    fn render_row(&self, component: Component, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();

        let mut details = format!("{} · {}", component.purpose(), component.download_size());

        if component.is_essential() {
            details = format!("{details} · {}", t!("setup.required"));
        }

        let description = div()
            .flex()
            .flex_col()
            .gap_1()
            .min_w_0()
            .child(div().text_size(px(NAME_TEXT_SIZE)).child(component.name()))
            .child(
                div()
                    .text_size(px(SECONDARY_TEXT_SIZE))
                    .text_color(theme.muted_foreground)
                    .child(details),
            );

        let install_button = |label: String| {
            Button::new(SharedString::from(format!("install-{}", component.name())))
                .ghost()
                .label(label)
                .disabled(self.queue.contains(&component))
                .on_click(cx.listener(move |this, _, _, cx| this.install(component, cx)))
        };

        let status = match self.statuses.get(&component) {
            Some(Status::Checking) | None => div()
                .text_color(theme.muted_foreground)
                .child(t!("setup.checking").to_string()),

            Some(Status::Installed) => div()
                .text_color(theme.muted_foreground)
                .child(t!("setup.installed").to_string()),

            Some(Status::Missing) => div().child(install_button(t!("setup.install").to_string())),

            Some(Status::Installing { label, fraction }) => div()
                .flex()
                .items_center()
                .gap_3()
                .children(fraction.map(|fraction| {
                    div().w(px(160.)).child(
                        Progress::new(SharedString::from(format!("progress-{}", component.name())))
                            .value(fraction * 100.),
                    )
                }))
                .child(label.clone()),

            Some(Status::Failed(error)) => div()
                .flex()
                .items_center()
                .gap_3()
                .min_w_0()
                .child(div().text_color(theme.danger).child(error.clone()))
                .child(install_button(t!("setup.retry").to_string())),
        };

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_6()
            .py_3()
            .border_b_1()
            .border_color(theme.border)
            .child(description)
            .child(status.text_size(px(SECONDARY_TEXT_SIZE)).flex_shrink_0().max_w(px(420.)))
    }
}

impl Render for SetupView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let install_folder = self.render_install_folder(cx);

        let rows: Vec<Div> = Component::ALL
            .iter()
            .map(|component| self.render_row(*component, cx))
            .collect();

        let theme = cx.theme();

        let welcome = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_size(px(TITLE_TEXT_SIZE)).child(t!("setup.welcome_title").to_string()))
            .child(
                div()
                    .text_size(px(SECONDARY_TEXT_SIZE))
                    .text_color(theme.muted_foreground)
                    .child(t!("setup.welcome_text").to_string()),
            );

        // In Settings the list flows with the page; only the welcome frames it as its main content.
        let component_list = if self.is_welcome {
            panel(theme.border, theme.radius)
                .id("components")
                .overflow_y_scroll()
                .px_6()
                .children(rows)
                .into_any_element()
        } else {
            div().children(rows).into_any_element()
        };

        let something_missing = Component::ALL
            .iter()
            .any(|component| self.needs_install(*component));

        let essentials_ready = Component::ALL
            .iter()
            .filter(|component| component.is_essential())
            .all(|component| matches!(self.statuses.get(component), Some(Status::Installed)));

        let note = div()
            .text_size(px(SECONDARY_TEXT_SIZE))
            .text_color(theme.muted_foreground)
            .child(t!("setup.note").to_string());

        let install_all_button = something_missing.then(|| {
            Button::new("install-all")
                .primary()
                .label(t!("setup.install_all").to_string())
                .disabled(self.is_installing)
                .on_click(cx.listener(|this, _, _, cx| this.install_everything_missing(cx)))
        });

        let start_label = if something_missing {
            t!("setup.start_without_optional").to_string()
        } else {
            t!("setup.start").to_string()
        };

        let mut start_button = Button::new("start")
            .label(start_label)
            .disabled(!essentials_ready || self.is_installing)
            .on_click(cx.listener(|_, _, _, cx| cx.emit(SetupFinished)));

        start_button = if something_missing {
            start_button.ghost()
        } else {
            start_button.primary()
        };

        let buttons = div()
            .flex()
            .items_center()
            .gap_3()
            .children(install_all_button)
            .children(self.is_welcome.then_some(start_button));

        let footer = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(note)
            .child(buttons);

        let page = if self.is_welcome {
            div().flex_1().min_h_0()
        } else {
            div()
        };

        page.flex()
            .flex_col()
            .gap_4()
            .children(self.is_welcome.then_some(welcome))
            .child(install_folder)
            .child(component_list)
            .child(footer)
    }
}
