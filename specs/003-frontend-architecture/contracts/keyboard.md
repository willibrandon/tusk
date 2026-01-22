# Contract: Keyboard Navigation

**Module**: `tusk_ui::key_bindings`

## Global Actions

```rust
actions!(workspace, [
    // Tab management
    NewQueryTab,        // Cmd+N
    CloseActiveTab,     // Cmd+W
    CloseAllTabs,       // Cmd+Shift+W
    NextTab,            // Cmd+Shift+]
    PreviousTab,        // Cmd+Shift+[
    ActivateTab1,       // Cmd+1
    ActivateTab2,       // Cmd+2
    ActivateTab3,       // Cmd+3
    ActivateTab4,       // Cmd+4
    ActivateTab5,       // Cmd+5
    ActivateTab6,       // Cmd+6
    ActivateTab7,       // Cmd+7
    ActivateTab8,       // Cmd+8
    ActivateTab9,       // Cmd+9 (last tab)

    // Dock toggles
    ToggleLeftDock,     // Cmd+B
    ToggleRightDock,    // Cmd+Shift+B
    ToggleBottomDock,   // Cmd+J

    // Pane management
    SplitRight,         // Cmd+\
    SplitDown,          // Cmd+Shift+\
    FocusNextPane,      // Cmd+K Cmd+Right
    FocusPreviousPane,  // Cmd+K Cmd+Left
    ClosePane,          // Cmd+K Cmd+W

    // Panel focus
    FocusSchemaBrowser, // Cmd+Shift+E
    FocusResults,       // Cmd+Shift+R
    FocusMessages,      // Cmd+Shift+M

    // Global
    CommandPalette,     // Cmd+Shift+P
    Settings,           // Cmd+,
]);
```

## Key Binding Registration

```rust
pub fn register_key_bindings(cx: &mut App) {
    cx.bind_keys([
        // Tab management
        KeyBinding::new("cmd-n", NewQueryTab, Some("Workspace")),
        KeyBinding::new("cmd-w", CloseActiveTab, Some("Workspace")),
        KeyBinding::new("cmd-shift-w", CloseAllTabs, Some("Workspace")),
        KeyBinding::new("cmd-shift-]", NextTab, Some("Workspace")),
        KeyBinding::new("cmd-shift-[", PreviousTab, Some("Workspace")),
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
        KeyBinding::new("cmd-shift-\\", SplitDown, Some("Workspace")),
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
    ]);
}
```

## Key Context System

Key contexts determine which actions are available based on focus:

```rust
// In render methods
div()
    .key_context("Workspace")
    .on_action(cx.listener(Self::handle_new_query_tab))
    // ...

div()
    .key_context("Pane")
    .on_action(cx.listener(Self::handle_close_tab))
    // ...

div()
    .key_context("Tree")
    .on_action(cx.listener(Self::handle_expand))
    // ...
```

## Component-Specific Actions

### Tree Navigation

```rust
actions!(tree, [
    SelectPrevious,     // Up
    SelectNext,         // Down
    ExpandSelected,     // Right
    CollapseSelected,   // Left
    ActivateSelected,   // Enter
    ExpandAll,          // Cmd+Shift+Right
    CollapseAll,        // Cmd+Shift+Left
]);

cx.bind_keys([
    KeyBinding::new("up", SelectPrevious, Some("Tree")),
    KeyBinding::new("down", SelectNext, Some("Tree")),
    KeyBinding::new("right", ExpandSelected, Some("Tree")),
    KeyBinding::new("left", CollapseSelected, Some("Tree")),
    KeyBinding::new("enter", ActivateSelected, Some("Tree")),
    KeyBinding::new("cmd-shift-right", ExpandAll, Some("Tree")),
    KeyBinding::new("cmd-shift-left", CollapseAll, Some("Tree")),
]);
```

### Select/Dropdown

```rust
actions!(select, [
    Open,               // Space, Enter, Down
    Close,              // Escape
    SelectNext,         // Down
    SelectPrevious,     // Up
    Confirm,            // Enter
]);

cx.bind_keys([
    KeyBinding::new("space", Open, Some("Select")),
    KeyBinding::new("enter", Open, Some("Select")),
    KeyBinding::new("down", Open, Some("Select")),
    KeyBinding::new("escape", Close, Some("SelectPopover")),
    KeyBinding::new("down", SelectNext, Some("SelectPopover")),
    KeyBinding::new("up", SelectPrevious, Some("SelectPopover")),
    KeyBinding::new("enter", Confirm, Some("SelectPopover")),
]);
```

### Modal

```rust
actions!(modal, [
    Dismiss,            // Escape
    Confirm,            // Enter
]);

cx.bind_keys([
    KeyBinding::new("escape", Dismiss, Some("Modal")),
    KeyBinding::new("enter", Confirm, Some("Modal")),
]);
```

### Context Menu

```rust
actions!(context_menu, [
    SelectNext,         // Down
    SelectPrevious,     // Up
    Confirm,            // Enter
    Dismiss,            // Escape
    OpenSubmenu,        // Right
    CloseSubmenu,       // Left
]);

cx.bind_keys([
    KeyBinding::new("down", SelectNext, Some("ContextMenu")),
    KeyBinding::new("up", SelectPrevious, Some("ContextMenu")),
    KeyBinding::new("enter", Confirm, Some("ContextMenu")),
    KeyBinding::new("escape", Dismiss, Some("ContextMenu")),
    KeyBinding::new("right", OpenSubmenu, Some("ContextMenu")),
    KeyBinding::new("left", CloseSubmenu, Some("ContextMenu")),
]);
```

## Requirements Mapping

| Requirement | Implementation |
|-------------|----------------|
| FR-040 | Global shortcuts registered in `register_key_bindings()` |
| FR-041 | Panel-specific shortcuts with key contexts |
| FR-042 | Tab navigation via standard browser behavior |
| FR-043 | Focus indicators via `track_focus()` + CSS |

## Focus Indicators

All focusable elements must have visible focus indicators:

```rust
fn render_focus_ring(&self, focused: bool, theme: &TuskTheme) -> impl IntoElement {
    if focused {
        div()
            .absolute()
            .inset_0()
            .rounded(px(4.0))
            .border_2()
            .border_color(theme.colors.accent)
    } else {
        div()
    }
}
```

WCAG 2.1 AA requires:
- Focus indicators with 3:1 contrast ratio minimum
- Focus indicators at least 2px in size
- Focus visible on all interactive elements
