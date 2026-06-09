use gpui::{px, size, App, AppContext, Application, Bounds, WindowBounds, WindowOptions};
use network_manager_ui::{install_app_hotkeys, NetworkManagerApp, NetworkManagerAssets};

fn main() {
    Application::new()
        .with_assets(NetworkManagerAssets)
        .run(|cx: &mut App| {
            install_app_hotkeys(cx);
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
            cx.open_window(
                WindowOptions {
                    titlebar: None,
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(640.0), px(480.0))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| NetworkManagerApp::live()),
            )
            .expect("open Network Manager desktop window");
            cx.activate(true);
        });
}
