//! Tusk - A fast, native PostgreSQL client built with GPUI.

mod app;

use app::TuskApp;
use gpui::{px, size, App, AppContext, Application, Bounds, Size, WindowBounds, WindowOptions};
use tracing_subscriber::EnvFilter;
use tusk_ui::TuskTheme;

fn main() {
    // Initialize tracing with RUST_LOG environment variable support
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting Tusk");

    Application::new().run(|cx: &mut App| {
        // Register TuskTheme as global state
        cx.set_global(TuskTheme::default());

        // Quit when all windows are closed
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        // Configure window bounds: 1400x900 centered on primary display
        let window_size = size(px(1400.0), px(900.0));
        let bounds = Bounds::centered(None, window_size, cx);

        // Configure window options
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(Size {
                width: px(800.0),
                height: px(600.0),
            }),
            focus: true,
            show: true,
            ..Default::default()
        };

        // Open the main window
        cx.open_window(window_options, |_window, cx| cx.new(|_| TuskApp::new()))
            .expect("Failed to open window");

        // Activate the application (bring to front)
        cx.activate(true);
    });
}
