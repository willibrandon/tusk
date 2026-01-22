# Contract: Workspace

**Component**: `Workspace`
**Module**: `tusk_ui::workspace`
**Implements**: `Render`, `Focusable`

## Public Interface

```rust
impl Workspace {
    /// Create a new workspace with default layout
    pub fn new(cx: &mut Context<Self>) -> Self;

    /// Get the left dock entity
    pub fn left_dock(&self) -> &Entity<Dock>;

    /// Get the right dock entity (if present)
    pub fn right_dock(&self) -> Option<&Entity<Dock>>;

    /// Get the bottom dock entity
    pub fn bottom_dock(&self) -> &Entity<Dock>;

    /// Get the center pane group
    pub fn center(&self) -> &Entity<PaneGroup>;

    /// Get the active pane
    pub fn active_pane(&self, cx: &App) -> Entity<Pane>;

    /// Toggle dock visibility by position
    pub fn toggle_dock(&mut self, position: DockPosition, cx: &mut Context<Self>);

    /// Open a new tab in the active pane
    pub fn open_tab(&mut self, item: TabItem, cx: &mut Context<Self>);

    /// Split the active pane
    pub fn split_pane(&mut self, axis: Axis, cx: &mut Context<Self>);

    /// Save workspace state to storage
    pub fn save_state(&self, cx: &App) -> WorkspaceState;

    /// Restore workspace state from storage
    pub fn restore_state(&mut self, state: WorkspaceState, cx: &mut Context<Self>);
}
```

## Events

```rust
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    /// Dock visibility changed
    DockToggled { position: DockPosition, visible: bool },

    /// Active pane changed
    ActivePaneChanged { pane: Entity<Pane> },

    /// Layout changed (split, close, resize)
    LayoutChanged,
}

impl EventEmitter<WorkspaceEvent> for Workspace {}
```

## Actions

```rust
actions!(workspace, [
    ToggleLeftDock,      // Cmd+B
    ToggleRightDock,     // Cmd+Shift+B
    ToggleBottomDock,    // Cmd+J
    NewQueryTab,         // Cmd+N
    CloseActiveTab,      // Cmd+W
    SplitRight,          // Cmd+\
    SplitDown,           // Cmd+Shift+\
    FocusNextPane,       // Cmd+K Cmd+Right
    FocusPreviousPane,   // Cmd+K Cmd+Left
]);
```

## Requirements Mapping

| Requirement | Method/Event |
|-------------|--------------|
| FR-001 | `new()` creates workspace with docks |
| FR-002 | `save_state()`, `restore_state()` |
| FR-003 | `DockPosition` enum |
| FR-016 | `toggle_dock()` with keyboard shortcuts |

## Persistence

```rust
#[derive(Serialize, Deserialize)]
pub struct WorkspaceState {
    pub left_dock_size: f32,
    pub left_dock_visible: bool,
    pub right_dock_size: Option<f32>,
    pub right_dock_visible: Option<bool>,
    pub bottom_dock_size: f32,
    pub bottom_dock_visible: bool,
    pub pane_layout: PaneLayout,
}
```
