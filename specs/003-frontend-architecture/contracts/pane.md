# Contract: Pane and PaneGroup

**Components**: `Pane`, `PaneGroup`, `PaneNode`
**Module**: `tusk_ui::pane`
**Implements**: `Render`, `Focusable`

## Pane Public Interface

```rust
impl Pane {
    /// Create a new empty pane
    pub fn new(cx: &mut Context<Self>) -> Self;

    /// Get all tabs
    pub fn tabs(&self) -> &[TabItem];

    /// Get the active tab index
    pub fn active_tab_index(&self) -> usize;

    /// Get the active tab item
    pub fn active_tab(&self) -> Option<&TabItem>;

    /// Add a new tab
    pub fn add_tab(&mut self, item: TabItem, cx: &mut Context<Self>);

    /// Close a tab by index
    pub fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) -> Option<TabItem>;

    /// Activate a tab by index
    pub fn activate_tab(&mut self, index: usize, cx: &mut Context<Self>);

    /// Move tab from one index to another (reorder)
    pub fn move_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>);

    /// Check if any tab has unsaved changes
    pub fn has_dirty_tabs(&self) -> bool;

    /// Get editor entity for a tab
    pub fn editor_for_tab(&self, tab_id: Uuid) -> Option<&Entity<QueryEditor>>;

    /// Check if pane is empty
    pub fn is_empty(&self) -> bool;
}
```

## PaneGroup Public Interface

```rust
impl PaneGroup {
    /// Create a new pane group with a single pane
    pub fn new(cx: &mut Context<Self>) -> Self;

    /// Get the active pane
    pub fn active_pane(&self) -> &Entity<Pane>;

    /// Set the active pane
    pub fn set_active_pane(&mut self, pane: Entity<Pane>, cx: &mut Context<Self>);

    /// Split the active pane along an axis
    pub fn split(&mut self, axis: Axis, cx: &mut Context<Self>) -> Entity<Pane>;

    /// Close a pane (collapses split if needed)
    pub fn close_pane(&mut self, pane: Entity<Pane>, cx: &mut Context<Self>);

    /// Resize split ratios
    pub fn resize_split(&mut self, index: usize, ratio: f32, cx: &mut Context<Self>);

    /// Get all panes (flattened)
    pub fn panes(&self) -> Vec<Entity<Pane>>;

    /// Navigate to next/previous pane
    pub fn focus_next_pane(&mut self, cx: &mut Context<Self>);
    pub fn focus_previous_pane(&mut self, cx: &mut Context<Self>);
}
```

## Events

```rust
#[derive(Debug, Clone)]
pub enum PaneEvent {
    /// Tab was added
    TabAdded { tab: TabItem },

    /// Tab was closed
    TabClosed { tab_id: Uuid },

    /// Active tab changed
    ActiveTabChanged { index: usize },

    /// Tab reordered
    TabMoved { from: usize, to: usize },

    /// Pane wants to close (all tabs closed)
    Close,
}

impl EventEmitter<PaneEvent> for Pane {}

#[derive(Debug, Clone)]
pub enum PaneGroupEvent {
    /// A pane was split
    Split { axis: Axis, new_pane: Entity<Pane> },

    /// A pane was closed
    PaneClosed { pane: Entity<Pane> },

    /// Active pane changed
    ActivePaneChanged { pane: Entity<Pane> },

    /// Layout ratios changed
    RatiosChanged,
}

impl EventEmitter<PaneGroupEvent> for PaneGroup {}
```

## TabItem Structure

```rust
pub struct TabItem {
    pub id: Uuid,
    pub title: SharedString,
    pub icon: Option<IconName>,
    pub dirty: bool,
    pub closable: bool,
}

impl TabItem {
    pub fn new(title: impl Into<SharedString>) -> Self;
    pub fn with_icon(self, icon: IconName) -> Self;
    pub fn with_dirty(self, dirty: bool) -> Self;
    pub fn with_closable(self, closable: bool) -> Self;
}
```

## PaneNode Enum

```rust
pub enum PaneNode {
    Single(Entity<Pane>),
    Split {
        axis: Axis,
        children: SmallVec<[PaneNode; 2]>,
        ratios: SmallVec<[f32; 2]>,
    },
}
```

## Requirements Mapping

| Requirement | Method/Event |
|-------------|--------------|
| FR-008 | `Pane::add_tab()`, multiple tabs per pane |
| FR-009 | `PaneGroup::split()` with Axis::Horizontal/Vertical |
| FR-010 | `TabItem::dirty` field |
| FR-011 | `Pane::close_tab()` checks dirty state |
| FR-012 | `Pane::move_tab()` for reordering |
| FR-013 | `Pane::is_empty()` triggers empty state UI |

## Invariants

1. `active_tab_index < tabs.len()` (unless empty)
2. `ratios` in Split always sum to 1.0
3. At least one pane always exists in PaneGroup
4. Split nodes always have at least 2 children
