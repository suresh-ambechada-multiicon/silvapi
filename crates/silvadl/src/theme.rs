use gpui_kit::*;
use gpui_kit::component::{Theme, ThemeRegistry};
use std::path::PathBuf;

#[derive(rust_embed::RustEmbed)]
#[folder = "../../themes/"]
struct ThemeAssets;

pub fn initialize(cx: &mut App) {
    for file in ThemeAssets::iter() {
        if let Some(content) = ThemeAssets::get(&file)
            && let Ok(json) = std::str::from_utf8(content.data.as_ref())
        {
            let _ = ThemeRegistry::global_mut(cx).load_themes_from_str(json);
        }
    }
    let preferred = std::env::var("SILVADL_THEME").unwrap_or_else(|_| "Ayu Dark".into());
    if let Some(theme) = ThemeRegistry::global(cx).themes().get(preferred.as_str()).cloned() {
        Theme::global_mut(cx).apply_config(&theme);
    }

    let directory = PathBuf::from("themes");
    if directory.exists() {
        let _ = ThemeRegistry::watch_dir(directory, cx, |_| {});
    }
}
