#![cfg_attr(windows, windows_subsystem = "windows")]

#[macro_use]
extern crate rust_i18n;

mod app;
mod language;
mod settings;
mod setup_mode;
mod stream_mode;
mod translate_mode;
mod voice;

use gpui_kit::*;

i18n!("locales", fallback = "en");

fn main() {
    app::paths::move_legacy_folders();
    app::locale::use_system_language();

    application().run(|cx| {
        init(cx);
        app::theme::init(cx);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1100.), px(700.)), cx)),
            titlebar: Some(TitlebarOptions {
                title: Some("KanniChan".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        open_window(options, cx, |window, cx| {
            cx.new(|cx| app::root::Root::new(window, cx))
        })
        .expect("failed to open window");
    });
}
