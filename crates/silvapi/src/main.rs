use gpui::*;
use gpui_component::{Root, Theme, ThemeRegistry};

#[cfg(target_family = "wasm")]
thread_local! {
    /// Keeps the application alive for the lifetime of the page.
    ///
    /// `Application::run` blocks forever on a desktop event loop, but on the web
    /// it must return control to the browser, which drops the `Application` and
    /// with it the last strong reference to the `App` — the window, its DOM
    /// callbacks and everything else go too, and gpui then logs "app was
    /// released". `run_embedded` hands back a handle holding that reference, so
    /// parking it here is what keeps the app running.
    static APPLICATION: std::cell::RefCell<Option<ApplicationHandle>> =
        const { std::cell::RefCell::new(None) };
}

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

/// Icons for the web build, embedded rather than fetched.
///
/// gpui-component's wasm asset source downloads each icon over `fetch` and, on a
/// cache miss, returns an error so gpui retries next frame. Any icon that does
/// not land in its cache therefore logs "Wasm assets loading, will be available
/// soon..." on *every* frame, forever. Embedding is 396 KB for all 99 icons and
/// removes the retry loop, the request storm and the log spam outright — an icon
/// is either present in the binary or it is not.
#[cfg(target_family = "wasm")]
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*.svg"]
struct WebAssets;

#[cfg(target_family = "wasm")]
impl AssetSource for WebAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }
        // `Ok(None)` rather than an error for a genuinely missing asset: gpui
        // treats the error case as "retry", which is what causes the per-frame
        // spam we are removing.
        Ok(Self::get(path).map(|file| file.data))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|candidate| candidate.starts_with(path).then(|| candidate.into()))
            .collect())
    }
}

/// Web entry point, invoked from JavaScript once the wasm module is loaded.
///
/// Assets are fetched from a URL rather than embedded: the web build downloads
/// icons on demand to keep the wasm bundle small.
#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    // Installs the panic hook and logging; the same call upstream's web example
    // makes before starting the app.
    gpui_platform::web_init();

    let app = gpui_platform::single_threaded_web().with_assets(WebAssets);

    APPLICATION.with(|application| {
        *application.borrow_mut() = Some(app.run_embedded(launch));
    });
}

fn run(app: Application) {
    app.run(launch);
}

/// Everything that happens once the platform is ready, shared by both targets.
fn launch(cx: &mut App) {
    {
        gpui_component::init(cx);
        init_theme(cx);

        // On the web the canvas is sized by the page, so asking for explicit
        // bounds fights the viewport; upstream's web example passes defaults.
        #[cfg(target_family = "wasm")]
        let window_options = WindowOptions::default();

        #[cfg(not(target_family = "wasm"))]
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

        match opened {
            Ok(_) => {
                cx.activate(true);
                dismiss_loading_overlay();
            }
            Err(error) => report_startup_failure(&format!("{error:#}")),
        }
    }
}

/// Remove the page's loading element once the window is up, so the canvas is
/// the only thing on screen. Driven from Rust rather than from the JS loader
/// because this is the point at which the app is genuinely running.
#[cfg(target_family = "wasm")]
fn dismiss_loading_overlay() {
    if let Some(document) = web_sys::window().and_then(|w| w.document())
        && let Some(status) = document.get_element_by_id("status")
    {
        status.remove();
    }
}

#[cfg(not(target_family = "wasm"))]
fn dismiss_loading_overlay() {}

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

    // A theme names font families that exist on a desktop; in the browser the
    // only faces available are the ones gpui_web bundles, so anything else
    // resolves to nothing and text does not draw. Pin the theme to those.
    #[cfg(target_family = "wasm")]
    {
        let theme = Theme::global_mut(cx);
        theme.font_family = "IBM Plex Sans".into();
        theme.mono_font_family = "Lilex".into();
    }
}
