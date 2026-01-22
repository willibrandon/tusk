//! Tusk - A fast, native PostgreSQL client built with GPUI.

mod app;
mod app_menus;

use app::TuskApp;
use gpui::{
    px, size, App, AppContext, Application, Bounds, PromptLevel, Size, WindowBounds, WindowOptions,
};
use tusk_core::logging::{init_logging, LogConfig};
use tusk_core::state::TuskState;
use tusk_ui::key_bindings::{About, CloseWindow, Minimize, Quit, ShowKeyboardShortcuts, Zoom};
use tusk_ui::{show_keyboard_shortcuts, TuskTheme};

fn main() {
    // Initialize logging before TuskState (FR-022, FR-023, FR-024)
    let log_config = LogConfig::new(tusk_core::logging::log_dir());
    let _logging_guard = init_logging(log_config);

    tracing::info!("Starting Tusk");

    Application::new().run(|cx: &mut App| {
        // Initialize TuskState and set as global (FR-005, SC-002)
        match TuskState::new() {
            Ok(state) => {
                cx.set_global(state);
                tracing::info!("TuskState initialized successfully");
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to initialize TuskState");
                // Continue without state - app will have limited functionality
            }
        }

        // Register TuskTheme as global state
        cx.set_global(TuskTheme::default());

        // Set up application menus
        let menus = app_menus::app_menus(cx);
        cx.set_menus(menus);

        // Register global action handlers
        register_global_actions(cx);

        // Configure window bounds: 1400x900 centered on primary display
        let window_size = size(px(1400.0), px(900.0));
        let bounds = Bounds::centered(None, window_size, cx);

        // Configure window options
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(Size { width: px(400.0), height: px(300.0) }),
            focus: true,
            show: true,
            ..Default::default()
        };

        // Open the main window
        cx.open_window(window_options, |window, cx| {
            // Handle window close manually to avoid Windows race condition.
            // Returning false prevents the standard Windows close sequence (which
            // triggers WM_ACTIVATE messages that race with window destruction).
            // Instead, we manually remove the window and quit the app.
            window.on_window_should_close(cx, |window, cx| {
                window.remove_window();
                cx.quit();
                false // Prevent standard close, we handled it manually
            });

            cx.new(|cx| TuskApp::new(window, cx))
        })
        .expect("Failed to open window");

        // Activate the application (bring to front)
        cx.activate(true);
    });
}

/// Register handlers for global application actions.
///
/// These actions work at the application level, independent of which
/// component has focus. Menu items are only enabled when their corresponding
/// action has a registered handler.
fn register_global_actions(cx: &mut App) {
    // Quit application
    cx.on_action(|_: &Quit, cx| {
        cx.quit();
    });

    // About Tusk dialog
    cx.on_action(|_: &About, cx| {
        // Defer to run after current dispatch completes (window may be borrowed during menu action)
        cx.defer(|cx| {
            if let Some(window_handle) = cx.windows().first().copied() {
                let result = window_handle.update(cx, |_, window, cx| {
                    let version = env!("CARGO_PKG_VERSION");
                    let message = format!("Tusk {version}");
                    let detail = "A fast, native PostgreSQL client built with GPUI.";
                    let prompt =
                        window.prompt(PromptLevel::Info, &message, Some(detail), &["OK"], cx);
                    cx.background_executor()
                        .spawn(async move {
                            let _ = prompt.await;
                        })
                        .detach();
                });
                if let Err(e) = result {
                    tracing::error!("About dialog failed: {e}");
                }
            }
        });
    });

    // Window management actions - deferred to run after current dispatch
    cx.on_action(|_: &Minimize, cx| {
        cx.defer(|cx| {
            if let Some(window_handle) = cx.windows().first().copied() {
                window_handle
                    .update(cx, |_, window, _cx| {
                        window.minimize_window();
                    })
                    .ok();
            }
        });
    });

    cx.on_action(|_: &Zoom, cx| {
        cx.defer(|cx| {
            if let Some(window_handle) = cx.windows().first().copied() {
                window_handle
                    .update(cx, |_, window, _cx| {
                        window.zoom_window();
                    })
                    .ok();
            }
        });
    });

    cx.on_action(|_: &CloseWindow, cx| {
        cx.defer(|cx| {
            if let Some(window_handle) = cx.windows().first().copied() {
                window_handle
                    .update(cx, |_, window, _cx| {
                        window.remove_window();
                    })
                    .ok();
            }
        });
    });

    // Keyboard shortcuts dialog
    cx.on_action(|_: &ShowKeyboardShortcuts, cx| {
        cx.defer(|cx| {
            show_keyboard_shortcuts(cx);
        });
    });
}
