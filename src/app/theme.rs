use gpui_kit::App;
use gpui_kit::component::{Theme, ThemeRegistry};

const WASHI_THEMES: &str = include_str!("../../assets/washi-theme.json");
const WINDOWS_SERIF: &str = "Yu Mincho";

pub fn init(cx: &mut App) {
    let registry = ThemeRegistry::global_mut(cx);

    let themes = if cfg!(windows) {
        WASHI_THEMES.replace("Noto Serif CJK JP", WINDOWS_SERIF)
    } else {
        WASHI_THEMES.to_string()
    };

    registry
        .load_themes_from_str(&themes)
        .expect("assets/washi-theme.json is valid");

    let washi = registry.themes()["Washi"].clone();
    let sumi = registry.themes()["Sumi"].clone();

    let theme = Theme::global_mut(cx);
    theme.light_theme = washi;
    theme.dark_theme = sumi;
}
