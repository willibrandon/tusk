# Contract: Context Menu

**Component**: `ContextMenu`
**Module**: `tusk_ui::context_menu`
**Implements**: `Render`, `Focusable`, `EventEmitter<ContextMenuEvent>`

## Public Interface

```rust
pub struct ContextMenu {
    items: Vec<ContextMenuItem>,
    position: Point<Pixels>,
    focus_handle: FocusHandle,
    highlighted_index: Option<usize>,
}

impl ContextMenu {
    /// Create a new context menu at position
    pub fn new(position: Point<Pixels>, cx: &mut Context<Self>) -> Self;

    /// Add a menu item
    pub fn item(self, item: ContextMenuItem) -> Self;

    /// Add multiple items
    pub fn items(self, items: impl IntoIterator<Item = ContextMenuItem>) -> Self;

    /// Show the menu
    pub fn show(&mut self, cx: &mut Context<Self>);

    /// Get the menu position
    pub fn position(&self) -> Point<Pixels>;
}
```

## ContextMenuItem Enum

```rust
pub enum ContextMenuItem {
    /// Clickable action item
    Action {
        label: SharedString,
        icon: Option<IconName>,
        shortcut: Option<SharedString>,
        disabled: bool,
        handler: Box<dyn Fn(&mut App)>,
    },

    /// Visual separator line
    Separator,

    /// Nested submenu
    Submenu {
        label: SharedString,
        icon: Option<IconName>,
        items: Vec<ContextMenuItem>,
    },
}

impl ContextMenuItem {
    /// Create an action item
    pub fn action(
        label: impl Into<SharedString>,
        handler: impl Fn(&mut App) + 'static,
    ) -> Self;

    /// Create an action with all options
    pub fn action_full(
        label: impl Into<SharedString>,
        icon: Option<IconName>,
        shortcut: Option<impl Into<SharedString>>,
        disabled: bool,
        handler: impl Fn(&mut App) + 'static,
    ) -> Self;

    /// Create a separator
    pub fn separator() -> Self;

    /// Create a submenu
    pub fn submenu(
        label: impl Into<SharedString>,
        items: Vec<ContextMenuItem>,
    ) -> Self;

    /// Builder: add icon
    pub fn icon(self, icon: IconName) -> Self;

    /// Builder: add shortcut hint
    pub fn shortcut(self, shortcut: impl Into<SharedString>) -> Self;

    /// Builder: set disabled
    pub fn disabled(self, disabled: bool) -> Self;
}
```

## Events

```rust
#[derive(Debug, Clone)]
pub enum ContextMenuEvent {
    /// Menu should close
    Close,
}

impl EventEmitter<ContextMenuEvent> for ContextMenu {}
```

## Actions

```rust
actions!(context_menu, [
    SelectNext,     // Down arrow
    SelectPrevious, // Up arrow
    Confirm,        // Enter
    Dismiss,        // Escape
    OpenSubmenu,    // Right arrow
    CloseSubmenu,   // Left arrow
]);
```

## Context Menu Layer

```rust
/// Global context menu manager
pub struct ContextMenuLayer {
    active_menu: Option<Entity<ContextMenu>>,
    submenu_stack: Vec<Entity<ContextMenu>>,
}

impl ContextMenuLayer {
    pub fn new() -> Self;

    /// Show a context menu at position
    pub fn show(&mut self, menu: Entity<ContextMenu>, cx: &mut App);

    /// Dismiss all menus
    pub fn dismiss(&mut self, cx: &mut App);

    /// Check if any menu is open
    pub fn is_open(&self) -> bool;
}

impl Global for ContextMenuLayer {}
```

## Usage Pattern

```rust
// Show context menu on right-click
fn handle_context_menu(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
    if event.button == MouseButton::Right {
        let menu = cx.new(|cx| {
            ContextMenu::new(event.position, cx)
                .item(ContextMenuItem::action("Select Top 100", |cx| { /* ... */ })
                    .icon(IconName::Play)
                    .shortcut("Cmd+Return"))
                .item(ContextMenuItem::separator())
                .item(ContextMenuItem::action("Copy Name", |cx| { /* ... */ })
                    .icon(IconName::Copy)
                    .shortcut("Cmd+C"))
                .item(ContextMenuItem::submenu("Export", vec![
                    ContextMenuItem::action("As CSV", |cx| { /* ... */ }),
                    ContextMenuItem::action("As JSON", |cx| { /* ... */ }),
                    ContextMenuItem::action("As SQL", |cx| { /* ... */ }),
                ]))
        });

        cx.global::<ContextMenuLayer>().show(menu, cx);
    }
}
```

## Requirements Mapping

| Requirement | Implementation |
|-------------|----------------|
| FR-036 | Menu positioned at `event.position` |
| FR-037 | `shortcut` field displayed right-aligned |
| FR-038 | `Submenu` variant with nested items |
| FR-039 | Click outside triggers `ContextMenuEvent::Close` |

## Positioning

Context menus are positioned to avoid viewport overflow:
1. Default: top-left corner at cursor position
2. If would overflow right: anchor at top-right
3. If would overflow bottom: anchor at bottom-left
4. Submenus open to the right (or left if no space)

```rust
fn calculate_position(
    cursor: Point<Pixels>,
    menu_size: Size<Pixels>,
    viewport: Size<Pixels>,
) -> Point<Pixels> {
    let mut pos = cursor;

    // Adjust for right overflow
    if pos.x + menu_size.width > viewport.width {
        pos.x = cursor.x - menu_size.width;
    }

    // Adjust for bottom overflow
    if pos.y + menu_size.height > viewport.height {
        pos.y = cursor.y - menu_size.height;
    }

    pos
}
```

## Keyboard Navigation

| Key | Action |
|-----|--------|
| Down | Highlight next item (skip separators) |
| Up | Highlight previous item (skip separators) |
| Enter | Activate highlighted item |
| Right | Open submenu (if highlighted is submenu) |
| Left | Close current submenu, return to parent |
| Escape | Close all menus |
