//! Keyboard shortcuts and action definitions for Tusk.
//!
//! This module defines all global actions and registers key bindings.

use gpui::{actions, App, KeyBinding};

// ============================================================================
// Workspace Actions
// ============================================================================

actions!(
    workspace,
    [
        // Tab management
        NewQueryTab,
        CloseActiveTab,
        CloseAllTabs,
        NextTab,
        PreviousTab,
        ActivateTab1,
        ActivateTab2,
        ActivateTab3,
        ActivateTab4,
        ActivateTab5,
        ActivateTab6,
        ActivateTab7,
        ActivateTab8,
        ActivateTab9,
        // Dock toggles
        ToggleLeftDock,
        ToggleRightDock,
        ToggleBottomDock,
        // Pane management
        SplitRight,
        SplitDown,
        FocusNextPane,
        FocusPreviousPane,
        ClosePane,
        // Panel focus
        FocusSchemaBrowser,
        FocusResults,
        FocusMessages,
        // Global
        CommandPalette,
        Settings,
        // Application
        Quit,
        About,
        CloseWindow,
        Minimize,
        Zoom,
    ]
);

// ============================================================================
// Query Actions
// ============================================================================

actions!(query, [RunQuery, ExplainQuery, FormatQuery, CancelQuery,]);

// ============================================================================
// Tree Navigation Actions
// ============================================================================

pub mod tree {
    use gpui::actions;
    actions!(
        tree,
        [
            SelectPrevious,
            SelectNext,
            ExpandSelected,
            CollapseSelected,
            ActivateSelected,
            ExpandAll,
            CollapseAll,
        ]
    );
}

// ============================================================================
// Select/Dropdown Actions
// ============================================================================

pub mod select {
    use gpui::actions;
    actions!(select, [Open, Close, SelectNextOption, SelectPreviousOption, Confirm,]);
}

// ============================================================================
// Modal Actions
// ============================================================================

/// Modal actions module.
pub mod modal {
    use gpui::actions;
    actions!(modal, [Dismiss, ConfirmAction,]);
}

// ============================================================================
// Context Menu Actions
// ============================================================================

pub mod context_menu {
    use gpui::actions;
    actions!(
        context_menu,
        [SelectNextItem, SelectPreviousItem, ConfirmItem, DismissMenu, OpenSubmenu, CloseSubmenu,]
    );
}

// ============================================================================
// Key Binding Registration
// ============================================================================

/// Register all global key bindings.
///
/// This should be called once during application initialization.
pub fn register_key_bindings(cx: &mut App) {
    cx.bind_keys([
        // Tab management
        KeyBinding::new("cmd-n", NewQueryTab, Some("Workspace")),
        KeyBinding::new("cmd-w", CloseActiveTab, Some("Workspace")),
        KeyBinding::new("cmd-shift-w", CloseAllTabs, Some("Workspace")),
        KeyBinding::new("cmd-}", NextTab, Some("Workspace")),
        KeyBinding::new("cmd-{", PreviousTab, Some("Workspace")),
        KeyBinding::new("cmd-1", ActivateTab1, Some("Workspace")),
        KeyBinding::new("cmd-2", ActivateTab2, Some("Workspace")),
        KeyBinding::new("cmd-3", ActivateTab3, Some("Workspace")),
        KeyBinding::new("cmd-4", ActivateTab4, Some("Workspace")),
        KeyBinding::new("cmd-5", ActivateTab5, Some("Workspace")),
        KeyBinding::new("cmd-6", ActivateTab6, Some("Workspace")),
        KeyBinding::new("cmd-7", ActivateTab7, Some("Workspace")),
        KeyBinding::new("cmd-8", ActivateTab8, Some("Workspace")),
        KeyBinding::new("cmd-9", ActivateTab9, Some("Workspace")),
        // Dock toggles
        KeyBinding::new("cmd-b", ToggleLeftDock, Some("Workspace")),
        KeyBinding::new("cmd-shift-b", ToggleRightDock, Some("Workspace")),
        KeyBinding::new("cmd-j", ToggleBottomDock, Some("Workspace")),
        // Pane management
        KeyBinding::new("cmd-\\", SplitRight, Some("Workspace")),
        KeyBinding::new("cmd-|", SplitDown, Some("Workspace")),
        KeyBinding::new("cmd-k cmd-right", FocusNextPane, Some("Workspace")),
        KeyBinding::new("cmd-k cmd-left", FocusPreviousPane, Some("Workspace")),
        KeyBinding::new("cmd-k cmd-w", ClosePane, Some("Workspace")),
        // Panel focus
        KeyBinding::new("cmd-shift-e", FocusSchemaBrowser, Some("Workspace")),
        KeyBinding::new("cmd-shift-r", FocusResults, Some("Workspace")),
        KeyBinding::new("cmd-shift-m", FocusMessages, Some("Workspace")),
        // Global
        KeyBinding::new("cmd-shift-p", CommandPalette, Some("Workspace")),
        KeyBinding::new("cmd-,", Settings, Some("Workspace")),
        KeyBinding::new("cmd-q", Quit, None),
    ]);

    // Query bindings
    cx.bind_keys([
        KeyBinding::new("cmd-enter", RunQuery, Some("QueryEditor")),
        KeyBinding::new("cmd-shift-e", ExplainQuery, Some("QueryEditor")),
        KeyBinding::new("cmd-shift-f", FormatQuery, Some("QueryEditor")),
        KeyBinding::new("escape", CancelQuery, Some("QueryEditor")),
    ]);

    // Tree navigation bindings
    cx.bind_keys([
        KeyBinding::new("up", tree::SelectPrevious, Some("Tree")),
        KeyBinding::new("down", tree::SelectNext, Some("Tree")),
        KeyBinding::new("right", tree::ExpandSelected, Some("Tree")),
        KeyBinding::new("left", tree::CollapseSelected, Some("Tree")),
        KeyBinding::new("enter", tree::ActivateSelected, Some("Tree")),
        KeyBinding::new("cmd-shift-right", tree::ExpandAll, Some("Tree")),
        KeyBinding::new("cmd-shift-left", tree::CollapseAll, Some("Tree")),
    ]);

    // Select/dropdown bindings
    cx.bind_keys([
        KeyBinding::new("space", select::Open, Some("Select")),
        KeyBinding::new("enter", select::Open, Some("Select")),
        KeyBinding::new("down", select::Open, Some("Select")),
        KeyBinding::new("escape", select::Close, Some("SelectPopover")),
        KeyBinding::new("down", select::SelectNextOption, Some("SelectPopover")),
        KeyBinding::new("up", select::SelectPreviousOption, Some("SelectPopover")),
        KeyBinding::new("enter", select::Confirm, Some("SelectPopover")),
    ]);

    // Modal bindings
    cx.bind_keys([
        KeyBinding::new("escape", self::modal::Dismiss, Some("Modal")),
        KeyBinding::new("enter", self::modal::ConfirmAction, Some("Modal")),
    ]);

    // Context menu bindings
    cx.bind_keys([
        KeyBinding::new("down", context_menu::SelectNextItem, Some("ContextMenu")),
        KeyBinding::new("up", context_menu::SelectPreviousItem, Some("ContextMenu")),
        KeyBinding::new("enter", context_menu::ConfirmItem, Some("ContextMenu")),
        KeyBinding::new("escape", context_menu::DismissMenu, Some("ContextMenu")),
        KeyBinding::new("right", context_menu::OpenSubmenu, Some("ContextMenu")),
        KeyBinding::new("left", context_menu::CloseSubmenu, Some("ContextMenu")),
    ]);
}
