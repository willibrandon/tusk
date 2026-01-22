//! Application menu definitions for Tusk.
//!
//! Defines the native application menu bar (File, Edit, View, etc.)
//! that appears in the OS menu bar on macOS and the window on other platforms.

use gpui::{App, Menu, MenuItem, OsAction};
use tusk_ui::key_bindings::{
    About, CloseActiveTab, CloseWindow, Minimize, NewQueryTab, Quit, Settings, SplitDown,
    SplitRight, ToggleBottomDock, ToggleLeftDock, Zoom,
};
use tusk_ui::{Copy, Cut, Paste, Redo, SelectAll, Undo};

/// Build the application menu structure.
pub fn app_menus(_cx: &mut App) -> Vec<Menu> {
    vec![
        // Application menu (Tusk)
        Menu {
            name: "Tusk".into(),
            items: vec![
                MenuItem::action("About Tusk", About),
                MenuItem::separator(),
                MenuItem::action("Settings...", Settings),
                MenuItem::separator(),
                #[cfg(target_os = "macos")]
                MenuItem::os_submenu("Services", gpui::SystemMenuType::Services),
                #[cfg(target_os = "macos")]
                MenuItem::separator(),
                MenuItem::action("Quit Tusk", Quit),
            ],
        },
        // File menu
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Query Tab", NewQueryTab),
                MenuItem::separator(),
                MenuItem::action("Close Tab", CloseActiveTab),
                MenuItem::action("Close Window", CloseWindow),
            ],
        },
        // Edit menu
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, OsAction::Cut),
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
                MenuItem::separator(),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
            ],
        },
        // View menu
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Schema Browser", ToggleLeftDock),
                MenuItem::action("Toggle Results Panel", ToggleBottomDock),
                MenuItem::separator(),
                MenuItem::submenu(Menu {
                    name: "Editor Layout".into(),
                    items: vec![
                        MenuItem::action("Split Right", SplitRight),
                        MenuItem::action("Split Down", SplitDown),
                    ],
                }),
            ],
        },
        // Window menu
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Minimize", Minimize),
                MenuItem::action("Zoom", Zoom),
                MenuItem::separator(),
            ],
        },
        // Help menu
        Menu { name: "Help".into(), items: vec![MenuItem::action("About Tusk", About)] },
    ]
}
