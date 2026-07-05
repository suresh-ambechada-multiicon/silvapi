use gpui::*;
use gpui_component::{Root, Theme, ThemeRegistry};
use std::path::PathBuf;

mod http;
mod runtime;
mod state;
mod storage;
mod ui;

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        init_theme(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1440.), px(900.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some(SharedString::from("Silvapi")),
                appears_transparent: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| ui::AppView::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}

#[derive(rust_embed::RustEmbed)]
#[folder = "../../themes/"]
struct ThemeAssets;

fn init_theme(cx: &mut App) {
    let theme_name = SharedString::from(
        crate::storage::load_theme_name()
            .ok()
            .flatten()
            .unwrap_or_else(|| "Ayu Dark".to_string()),
    );
    let themes_dir = PathBuf::from("./themes");

    // Load embedded themes first so they are always available
    for file in ThemeAssets::iter() {
        if let Some(content) = ThemeAssets::get(&file) {
            if let Ok(json_str) = std::str::from_utf8(content.data.as_ref()) {
                let _ = ThemeRegistry::global_mut(cx).load_themes_from_str(json_str);
            }
        }
    }

    if themes_dir.exists() {
        let _ = ThemeRegistry::watch_dir(themes_dir, cx, {
            let theme_name = theme_name.clone();
            move |cx| {
                for file in ThemeAssets::iter() {
                    if let Some(content) = ThemeAssets::get(&file) {
                        if let Ok(json_str) = std::str::from_utf8(content.data.as_ref()) {
                            let _ = ThemeRegistry::global_mut(cx).load_themes_from_str(json_str);
                        }
                    }
                }
                apply_theme(&theme_name, cx);
            }
        });
    }
    apply_theme(&theme_name, cx);
}

fn apply_theme(theme_name: &str, cx: &mut App) {
    if let Some(theme) = ThemeRegistry::global(cx).themes().get(theme_name).cloned() {
        Theme::global_mut(cx).apply_config(&theme);
    }
}
