use gpui::{px, size, App, AppContext, Application, Bounds, WindowBounds, WindowOptions};
use network_manager_ui::{data::SqliteRepository, NetworkManagerApp};

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                titlebar: None,
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| NetworkManagerApp::new(SqliteRepository::default())),
        )
        .expect("open Network Manager desktop window");
        cx.activate(true);
    });
}
