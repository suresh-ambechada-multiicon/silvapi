use gpui_kit::component::{Root, Theme, ThemeRegistry};
use gpui_kit::*;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;


mod http;
mod runtime;
mod state;
mod storage;
mod ui;

fn main() {
    let app = gpui_kit::application().with_assets(gpui_kit::assets::AllAssets::new(""));

    app.run(move |cx| {
        gpui_kit::init(cx);

        let fonts = vec![
            std::borrow::Cow::Borrowed(include_bytes!("../Roboto-Regular.ttf") as &[u8]),
            std::borrow::Cow::Borrowed(include_bytes!("../Roboto-Bold.ttf") as &[u8]),
            std::borrow::Cow::Borrowed(include_bytes!("../RobotoMono-Regular.ttf") as &[u8]),
            std::borrow::Cow::Borrowed(include_bytes!("../DejaVuSans.ttf") as &[u8]),
            std::borrow::Cow::Borrowed(include_bytes!("../DejaVuSans-Bold.ttf") as &[u8]),
            std::borrow::Cow::Borrowed(include_bytes!("../DejaVuSansMono.ttf") as &[u8]),
        ];
        let _ = cx.text_system().add_fonts(fonts);
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

fn init_theme(cx: &mut App) {
    let theme_name = SharedString::from(
        crate::storage::load_theme_name()
            .ok()
            .flatten()
            .unwrap_or_else(|| "Default Dark".to_string()),
    );

    #[cfg(not(target_arch = "wasm32"))]
    {
        let config_theme_dir = std::env::var("XDG_CONFIG_HOME")
            .map(|p| PathBuf::from(p).join("silvapi").join("theme"))
            .or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".config").join("silvapi").join("theme"))
            })
            .ok();

        if let Some(dir) = config_theme_dir {
            let _ = std::fs::create_dir_all(&dir);
            let theme_name_clone = theme_name.clone();
            let _ = ThemeRegistry::watch_dir(dir, cx, move |cx| {
                apply_theme(&theme_name_clone, cx);
            });
        }
    }

    apply_theme(&theme_name, cx);
}

fn apply_theme(theme_name: &str, cx: &mut App) {
    if let Some(theme) = ThemeRegistry::global(cx).themes().get(theme_name).cloned() {
        Theme::global_mut(cx).apply_config(&theme);
    }
    Theme::global_mut(cx).font_family = "Roboto".into();
    Theme::global_mut(cx).mono_font_family = "Roboto Mono".into();
}
