use gpui_kit::component::Root;
use gpui_kit::*;

mod theme;
mod ui;

fn main() {
    let config = silvadl::DesignTokens::load().expect("built-in design tokens");
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets::new(""))
        .run(move |cx| {
        gpui_kit::init(cx);
        theme::initialize(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(config.window_width), px(config.window_height)), cx))),
            titlebar: Some(TitlebarOptions { title: Some("SilvaDL".into()), ..Default::default() }),
            ..Default::default()
        };
        cx.spawn(async move |cx| {
            cx.open_window(options, |window, cx| {
                let view = cx.new(|cx| ui::DownloaderView::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            }).expect("open SilvaDL window");
        }).detach();
    });
}
