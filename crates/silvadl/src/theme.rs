use gpui_kit::*;
use gpui_kit::component::{Theme, ThemeRegistry};
use std::path::PathBuf;


pub fn initialize(cx: &mut App) {
    let preferred = std::env::var("SILVADL_THEME").unwrap_or_else(|_| "Ayu Dark".into());
    if let Some(theme) = ThemeRegistry::global(cx).themes().get(preferred.as_str()).cloned() {
        Theme::global_mut(cx).apply_config(&theme);
    }

    let directory = PathBuf::from("themes");
    if directory.exists() {
        let _ = ThemeRegistry::watch_dir(directory, cx, |_| {});
    }
}
