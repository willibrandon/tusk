//! Keyboard shortcuts panel for displaying all available key bindings.
//!
//! This module provides a modal dialog that shows all keyboard shortcuts
//! organized by category, helping users discover and learn the available
//! key bindings in Tusk.

use gpui::{div, prelude::*, px, App, Context, Render, Window};

use crate::modal::{Modal, ModalAction, ModalLayer};
use crate::TuskTheme;

/// A single keyboard shortcut entry.
struct ShortcutEntry {
    /// The key combination (e.g., "Cmd+N").
    keys: &'static str,
    /// Description of what the shortcut does.
    description: &'static str,
}

/// A category of shortcuts.
struct ShortcutCategory {
    /// Category name.
    name: &'static str,
    /// Shortcuts in this category.
    shortcuts: &'static [ShortcutEntry],
}

/// All keyboard shortcuts organized by category.
const SHORTCUTS: &[ShortcutCategory] = &[
    ShortcutCategory {
        name: "General",
        shortcuts: &[
            ShortcutEntry { keys: "Cmd+Q", description: "Quit Tusk" },
            ShortcutEntry { keys: "Cmd+,", description: "Open Settings" },
            ShortcutEntry { keys: "Cmd+/", description: "Show Keyboard Shortcuts" },
            ShortcutEntry { keys: "Cmd+Shift+P", description: "Command Palette" },
        ],
    },
    ShortcutCategory {
        name: "Tabs",
        shortcuts: &[
            ShortcutEntry { keys: "Cmd+N", description: "New Query Tab" },
            ShortcutEntry { keys: "Cmd+W", description: "Close Tab" },
            ShortcutEntry { keys: "Cmd+Shift+W", description: "Close All Tabs" },
            ShortcutEntry { keys: "Cmd+}", description: "Next Tab" },
            ShortcutEntry { keys: "Cmd+{", description: "Previous Tab" },
            ShortcutEntry { keys: "Cmd+1-9", description: "Activate Tab 1-9" },
        ],
    },
    ShortcutCategory {
        name: "Panels",
        shortcuts: &[
            ShortcutEntry { keys: "Cmd+B", description: "Toggle Schema Browser" },
            ShortcutEntry { keys: "Cmd+Shift+B", description: "Toggle Right Dock" },
            ShortcutEntry { keys: "Cmd+J", description: "Toggle Results Panel" },
            ShortcutEntry { keys: "Cmd+Shift+E", description: "Focus Schema Browser" },
            ShortcutEntry { keys: "Cmd+Shift+R", description: "Focus Results" },
            ShortcutEntry { keys: "Cmd+Shift+M", description: "Focus Messages" },
        ],
    },
    ShortcutCategory {
        name: "Editor Layout",
        shortcuts: &[
            ShortcutEntry { keys: "Cmd+\\", description: "Split Right" },
            ShortcutEntry { keys: "Cmd+|", description: "Split Down" },
            ShortcutEntry { keys: "Cmd+K Cmd+Right", description: "Focus Next Pane" },
            ShortcutEntry { keys: "Cmd+K Cmd+Left", description: "Focus Previous Pane" },
            ShortcutEntry { keys: "Cmd+K Cmd+W", description: "Close Pane" },
        ],
    },
    ShortcutCategory {
        name: "Query Editor",
        shortcuts: &[
            ShortcutEntry { keys: "Cmd+Enter", description: "Run Query" },
            ShortcutEntry { keys: "Cmd+Shift+E", description: "Explain Query" },
            ShortcutEntry { keys: "Cmd+Shift+F", description: "Format Query" },
            ShortcutEntry { keys: "Escape", description: "Cancel Query" },
        ],
    },
    ShortcutCategory {
        name: "Editing",
        shortcuts: &[
            ShortcutEntry { keys: "Cmd+Z", description: "Undo" },
            ShortcutEntry { keys: "Cmd+Shift+Z", description: "Redo" },
            ShortcutEntry { keys: "Cmd+X", description: "Cut" },
            ShortcutEntry { keys: "Cmd+C", description: "Copy" },
            ShortcutEntry { keys: "Cmd+V", description: "Paste" },
            ShortcutEntry { keys: "Cmd+A", description: "Select All" },
        ],
    },
    ShortcutCategory {
        name: "Tree Navigation",
        shortcuts: &[
            ShortcutEntry { keys: "Up/Down", description: "Navigate Items" },
            ShortcutEntry { keys: "Right", description: "Expand Item" },
            ShortcutEntry { keys: "Left", description: "Collapse Item" },
            ShortcutEntry { keys: "Enter", description: "Activate Item" },
            ShortcutEntry { keys: "Cmd+Shift+Right", description: "Expand All" },
            ShortcutEntry { keys: "Cmd+Shift+Left", description: "Collapse All" },
        ],
    },
];

/// View that renders the keyboard shortcuts content.
pub struct KeyboardShortcutsContent;

impl KeyboardShortcutsContent {
    /// Create a new keyboard shortcuts content view.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for KeyboardShortcutsContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();
        let text_color = theme.colors.text;
        let text_muted = theme.colors.text_muted;
        let border_color = theme.colors.border;
        let surface_bg = theme.colors.element_background;

        div()
            .id("keyboard-shortcuts-content")
            .flex()
            .flex_col()
            .gap(px(16.0))
            .overflow_y_scroll()
            .max_h(px(400.0))
            .children(SHORTCUTS.iter().map(|category| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    // Category header
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(text_color)
                            .child(category.name),
                    )
                    // Shortcuts list
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .border_1()
                            .border_color(border_color)
                            .rounded(px(6.0))
                            .overflow_hidden()
                            .children(category.shortcuts.iter().enumerate().map(
                                |(idx, shortcut)| {
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .px(px(12.0))
                                        .py(px(8.0))
                                        .when(idx > 0, |d| {
                                            d.border_t_1().border_color(border_color)
                                        })
                                        // Description
                                        .child(
                                            div()
                                                .text_size(px(13.0))
                                                .text_color(text_muted)
                                                .child(shortcut.description),
                                        )
                                        // Key combination
                                        .child(div().flex().gap(px(4.0)).children(
                                            shortcut.keys.split('+').map(|key| {
                                                div()
                                                    .px(px(6.0))
                                                    .py(px(2.0))
                                                    .bg(surface_bg)
                                                    .border_1()
                                                    .border_color(border_color)
                                                    .rounded(px(4.0))
                                                    .text_size(px(11.0))
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(text_color)
                                                    .child(key)
                                            }),
                                        ))
                                },
                            )),
                    )
            }))
    }
}

/// Show the keyboard shortcuts modal.
///
/// This function creates and displays a modal containing all keyboard shortcuts.
pub fn show_keyboard_shortcuts(cx: &mut App) {
    let content = cx.new(|cx| KeyboardShortcutsContent::new(cx));

    let modal = cx.new(|cx| {
        Modal::new("Keyboard Shortcuts", cx)
            .subtitle("Quick reference for all available shortcuts")
            .width(550.0)
            .body(content.into())
            .action(ModalAction::confirm("Close"))
    });

    cx.update_global::<ModalLayer, _>(|layer, cx| {
        layer.show(modal, cx);
    });
}
