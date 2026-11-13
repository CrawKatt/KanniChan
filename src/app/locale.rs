const TRANSLATED: [&str; 2] = ["es", "ja"];

pub fn use_system_language() {
    let system = sys_locale::get_locale().unwrap_or_default();

    let language = TRANSLATED
        .into_iter()
        .find(|language| system.starts_with(language))
        .unwrap_or("en");

    rust_i18n::set_locale(language);
}
