use gpui::*;
use gpui_component::{Root, Theme, ThemeRegistry};

mod http;
mod runtime;
mod state;
mod storage;
mod ui;

/// Native entry point. On the web the entry point is `start` below, called by
/// the JS loader, so `main` does nothing there.
fn main() {
    #[cfg(not(target_family = "wasm"))]
    run(gpui_platform::application().with_assets(gpui_component_assets::Assets));
}

/// Web entry point, invoked from JavaScript once the wasm module is loaded.
///
/// Assets are fetched from a URL rather than embedded: the web build downloads
/// icons on demand to keep the wasm bundle small.
#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    gpui_web::init_logging();

    let platform = std::rc::Rc::new(gpui_web::WebPlatform::new(false));
    let app = Application::with_platform(platform)
        .with_assets(gpui_component_assets::Assets::default());
    run(app);
}

fn run(app: Application) {
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

        // Opened directly rather than from a spawned task. When window creation
        // fails the app tears down, and a pending spawned task then panics with
        // "app was released before async operation completed" — an unreachable
        // wasm trap that buries the real cause. On the web the real cause is
        // almost always that no WebGPU adapter was available.
        let opened = cx.open_window(window_options, |window, cx| {
            let view = cx.new(|cx| ui::AppView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        });

        if let Err(error) = opened {
            report_startup_failure(&format!("{error:#}"));
        }
    });
}

/// Surface a fatal startup error where the user will actually see it.
fn report_startup_failure(message: &str) {
    log::error!("failed to open the main window: {message}");

    // The canvas never appeared, so the only place left to show anything is the
    // loading element the page starts with.
    #[cfg(target_family = "wasm")]
    if let Some(document) = web_sys::window().and_then(|w| w.document())
        && let Some(status) = document.get_element_by_id("status")
    {
        status.set_class_name("");
        status.set_inner_html(&format!(
            "<div class=\"err\"><strong>Could not start the renderer</strong><br><br>\
             <code>{}</code></div>",
            html_escape(message)
        ));
    }

    #[cfg(not(target_family = "wasm"))]
    eprintln!("failed to open the main window: {message}");
}

/// Minimal escaping so an error message cannot inject markup into the page.
#[cfg(target_family = "wasm")]
fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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

    // Embedded themes are always available on both platforms.
    for file in ThemeAssets::iter() {
        if let Some(content) = ThemeAssets::get(&file)
            && let Ok(json_str) = std::str::from_utf8(content.data.as_ref())
        {
            let _ = ThemeRegistry::global_mut(cx).load_themes_from_str(json_str);
        }
    }

    // Hot-reloading themes from ./themes is desktop-only: there is no directory
    // to watch in a browser, and gpui-component omits `watch_dir` on wasm.
    #[cfg(not(target_family = "wasm"))]
    {
        let themes_dir = std::path::PathBuf::from("./themes");
        if themes_dir.exists() {
            let _ = ThemeRegistry::watch_dir(themes_dir, cx, {
                let theme_name = theme_name.clone();
                move |cx| {
                    for file in ThemeAssets::iter() {
                        if let Some(content) = ThemeAssets::get(&file)
                            && let Ok(json_str) = std::str::from_utf8(content.data.as_ref())
                        {
                            let _ = ThemeRegistry::global_mut(cx).load_themes_from_str(json_str);
                        }
                    }
                    apply_theme(&theme_name, cx);
                }
            });
        }
    }

    apply_theme(&theme_name, cx);
}

fn apply_theme(theme_name: &str, cx: &mut App) {
    if let Some(theme) = ThemeRegistry::global(cx).themes().get(theme_name).cloned() {
        Theme::global_mut(cx).apply_config(&theme);
    }
}
