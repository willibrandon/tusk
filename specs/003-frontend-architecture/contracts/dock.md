# Contract: Dock

**Component**: `Dock`
**Module**: `tusk_ui::dock`
**Implements**: `Render`, `Focusable`

## Public Interface

```rust
impl Dock {
    /// Create a new dock at the specified position
    pub fn new(position: DockPosition, cx: &mut Context<Self>) -> Self;

    /// Get the dock position
    pub fn position(&self) -> DockPosition;

    /// Get current size in pixels
    pub fn size(&self) -> Pixels;

    /// Set size (clamped to min/max)
    pub fn set_size(&mut self, size: Pixels, cx: &mut Context<Self>);

    /// Check if dock is visible (not collapsed)
    pub fn is_visible(&self) -> bool;

    /// Toggle visibility
    pub fn toggle_visibility(&mut self, cx: &mut Context<Self>);

    /// Get registered panels
    pub fn panels(&self) -> &[Entity<dyn Panel>];

    /// Add a panel to the dock
    pub fn add_panel(&mut self, panel: Entity<dyn Panel>, cx: &mut Context<Self>);

    /// Get the active panel index
    pub fn active_panel_index(&self) -> usize;

    /// Activate a panel by index
    pub fn activate_panel(&mut self, index: usize, cx: &mut Context<Self>);

    /// Get size constraints for this dock position
    pub fn size_constraints(&self) -> (Pixels, Pixels);
}
```

## Events

```rust
#[derive(Debug, Clone)]
pub enum DockEvent {
    /// Dock was resized
    Resized { size: Pixels },

    /// Visibility toggled
    VisibilityChanged { visible: bool },

    /// Active panel changed
    PanelChanged { index: usize },
}

impl EventEmitter<DockEvent> for Dock {}
```

## Size Constraints

| Position | Min | Max |
|----------|-----|-----|
| Left | 120px | 600px |
| Right | 120px | 600px |
| Bottom | 100px | 50% viewport height |

## Requirements Mapping

| Requirement | Method/Event |
|-------------|--------------|
| FR-004 | Resize via drag (handled by Resizer) |
| FR-005 | `size_constraints()` returns (120px, 600px) for side docks |
| FR-006 | `size_constraints()` returns (100px, 50vh) for bottom |
| FR-007 | `toggle_visibility()`, `VisibilityChanged` event |

## Persistence

Dock size and visibility are persisted as part of `WorkspaceState`:

```rust
// In WorkspaceState
pub left_dock_size: f32,
pub left_dock_visible: bool,
// etc.
```
