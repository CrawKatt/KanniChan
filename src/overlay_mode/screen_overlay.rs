use std::cmp::Ordering;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::app::preferences::Preferences;
use crate::language::furigana::Segment;
use crate::stream_mode::streamer::Subtitle;
use crate::stream_mode::subtitle_style::SubtitleStyle;
use crate::translate_mode::translate::Translation;

const VISIBLE_TIME: Duration = Duration::from_secs(8);
const SCREEN_HEIGHT_FRACTION: f32 = 0.5;
const BOTTOM_MARGIN_FRACTION: f32 = 0.06;
const SIDE_MARGIN: f32 = 48.;
const TRANSLATION_SCALE: f32 = 0.73;
const READING_SCALE: f32 = 0.45;
const OUTLINE_DIRECTIONS: [(f32, f32); 8] = [
    (-1., -1.),
    (0., -1.),
    (1., -1.),
    (-1., 0.),
    (1., 0.),
    (-1., 1.),
    (0., 1.),
    (1., 1.),
];

pub struct ScreenOverlay {
    subtitle: Option<Subtitle>,
    style: SubtitleStyle,
    shown: u64,
}

pub fn screens(cx: &App) -> Vec<Rc<dyn PlatformDisplay>> {
    let mut screens = cx.displays();

    screens.sort_by(|a, b| {
        let (a, b) = (a.bounds().origin, b.bounds().origin);

        (a.y, a.x).partial_cmp(&(b.y, b.x)).unwrap_or(Ordering::Equal)
    });

    screens
}

pub fn screen_id(screen: &dyn PlatformDisplay) -> Option<String> {
    screen.uuid().ok().map(|uuid| uuid.to_string())
}

pub fn screen_label(number: usize, screen: &dyn PlatformDisplay) -> String {
    let bounds = screen.bounds();

    t!(
        "subtitle.screen_option",
        number = number,
        width = f32::from(bounds.size.width),
        height = f32::from(bounds.size.height),
        x = f32::from(bounds.origin.x),
        y = f32::from(bounds.origin.y)
    )
    .to_string()
}

/// Wayland has no primary display, so "automatic" falls back to the first screen.
fn chosen_screen(cx: &App) -> Option<Rc<dyn PlatformDisplay>> {
    let screens = screens(cx);
    let saved = Preferences::load().overlay_screen;

    saved
        .and_then(|id| {
            screens
                .iter()
                .find(|screen| screen_id(screen.as_ref()).as_ref() == Some(&id))
                .cloned()
        })
        .or_else(|| cx.primary_display())
        .or_else(|| screens.first().cloned())
}

impl ScreenOverlay {
    pub fn open(cx: &mut App) -> Result<WindowHandle<Self>, String> {
        // On X11 a layer surface would open as an ordinary, opaque window.
        #[cfg(target_os = "linux")]
        if std::env::var_os("WAYLAND_DISPLAY").is_none() {
            return Err(t!("error.screen_overlay_wayland").to_string());
        }

        let display = chosen_screen(cx)
            .ok_or_else(|| t!("error.screen_overlay", error = "no display").to_string())?;

        let screen = display.bounds();
        let height = screen.size.height * SCREEN_HEIGHT_FRACTION;
        let bounds = Bounds::new(
            point(screen.origin.x, screen.origin.y + screen.size.height - height),
            size(screen.size.width, height),
        );

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: false,
            kind: overlay_kind(),
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            display_id: Some(display.id()),
            window_background: WindowBackgroundAppearance::Transparent,
            app_id: Some("kannichan-subtitles".into()),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            ignore_mouse(window);

            cx.new(|_| Self {
                subtitle: None,
                style: Preferences::load().subtitle_style,
                shown: 0,
            })
        })
        .map_err(|error| t!("error.screen_overlay", error = error).to_string())
    }

    pub fn show(&mut self, subtitle: Subtitle, cx: &mut Context<Self>) {
        self.style = Preferences::load().subtitle_style;
        self.subtitle = Some(subtitle);
        self.shown += 1;

        let shown = self.shown;

        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(VISIBLE_TIME).await;

            let _ = this.update(cx, |overlay, cx| {
                if overlay.shown == shown {
                    overlay.subtitle = None;
                    cx.notify();
                }
            });
        })
        .detach();

        cx.notify();
    }
}

#[cfg(target_os = "linux")]
fn overlay_kind() -> WindowKind {
    use gpui_kit::layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions};

    WindowKind::LayerShell(LayerShellOptions {
        namespace: "kannichan-subtitles".into(),
        layer: Layer::Overlay,
        anchor: Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        keyboard_interactivity: KeyboardInteractivity::None,
        ..Default::default()
    })
}

#[cfg(not(target_os = "linux"))]
fn overlay_kind() -> WindowKind {
    WindowKind::PopUp
}

#[cfg(not(windows))]
fn ignore_mouse(window: &Window) {
    window.set_input_region(Some(&[]));
}

#[cfg(windows)]
fn ignore_mouse(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetWindowLongPtrW(window: isize, index: i32) -> isize;
        fn SetWindowLongPtrW(window: isize, index: i32, value: isize) -> isize;
        fn SetLayeredWindowAttributes(window: isize, key: u32, alpha: u8, flags: u32) -> i32;
    }

    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TRANSPARENT: isize = 0x0000_0020;
    const WS_EX_LAYERED: isize = 0x0008_0000;
    const LWA_ALPHA: u32 = 0x2;

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };

    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };

    let hwnd = handle.hwnd.get();

    // SAFETY: `hwnd` is the live window GPUI just created on this thread.
    // A layered window stays invisible until its attributes are set, hence the full alpha.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);

        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED | WS_EX_TRANSPARENT);
        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
    }
}

fn outlined(text: impl Fn(Hsla) -> Div, style: SubtitleStyle) -> Div {
    let width = style.outline_width as f32;
    let outline_color: Hsla = rgb(style.outline_color).into();

    let outline = OUTLINE_DIRECTIONS
        .iter()
        .filter(|_| width > 0.)
        .map(|(x, y)| {
            text(outline_color)
                .absolute()
                .w_full()
                .left(px(x * width))
                .top(px(y * width))
        });

    div()
        .relative()
        .children(outline)
        .child(text(rgb(style.text_color).into()))
}

fn subtitle_text(text: &Translation, text_size: f32, color: Hsla, furigana: bool) -> Div {
    let lines: Vec<Div> = match text {
        Translation::Spanish(spanish) => spanish
            .lines()
            .map(|line| div().child(line.to_string()))
            .collect(),

        Translation::Japanese(lines) => lines
            .iter()
            .map(|line| ruby_line(line, text_size, furigana))
            .collect(),
    };

    div()
        .flex()
        .flex_col()
        .items_center()
        .text_center()
        .text_size(px(text_size))
        .text_color(color)
        .children(lines)
}

fn ruby_line(line: &[Segment], text_size: f32, furigana: bool) -> Div {
    let reading_size = text_size * READING_SCALE;

    let segments = line.iter().map(|segment| {
        let reading = furigana.then(|| {
            div()
                .h(px(reading_size * 1.2))
                .text_size(px(reading_size))
                .child(segment.ruby.clone().unwrap_or_default())
        });

        div()
            .flex()
            .flex_col()
            .items_center()
            .children(reading)
            .child(segment.text.clone())
    });

    div()
        .flex()
        .flex_wrap()
        .justify_center()
        .children(segments)
}

impl Render for ScreenOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = div()
            .size_full()
            .flex()
            .flex_col()
            .justify_end()
            .items_center()
            .px(px(SIDE_MARGIN))
            .pb(window.viewport_size().height * BOTTOM_MARGIN_FRACTION / SCREEN_HEIGHT_FRACTION)
            .font_family(cx.theme().font_family.clone());

        let Some(subtitle) = &self.subtitle else {
            return page;
        };

        let style = self.style;
        let original_size = style.font_size as f32;
        let translation_size = original_size * TRANSLATION_SCALE;

        let original = outlined(
            |color| subtitle_text(&subtitle.original, original_size, color, style.furigana),
            style,
        );

        let translation = subtitle.translation.as_ref().ok().map(|translation| {
            outlined(
                |color| subtitle_text(translation, translation_size, color, style.furigana),
                style,
            )
        });

        page.child(original).children(translation)
    }
}
