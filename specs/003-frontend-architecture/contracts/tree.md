# Contract: Tree and TreeItem

**Components**: `Tree<T>`, `TreeItem` trait
**Module**: `tusk_ui::tree`
**Implements**: `Render`, `Focusable`

## TreeItem Trait

```rust
/// Trait for tree node data
pub trait TreeItem: Clone + 'static {
    /// Associated type for unique identification
    type Id: Clone + Eq + Hash + 'static;

    /// Get the unique identifier for this item
    fn id(&self) -> Self::Id;

    /// Get the display label
    fn label(&self) -> SharedString;

    /// Get the icon for this item
    fn icon(&self) -> Option<IconName>;

    /// Get child items (None = not expandable, Some([]) = expandable but empty)
    fn children(&self) -> Option<&[Self]>;

    /// Check if this item is expandable (default: has children)
    fn is_expandable(&self) -> bool {
        self.children().map_or(false, |c| !c.is_empty())
    }

    /// Get depth for indentation (computed during render)
    fn depth(&self) -> usize { 0 }
}
```

## Tree Public Interface

```rust
impl<T: TreeItem> Tree<T> {
    /// Create a new tree with root items
    pub fn new(items: Vec<T>, cx: &mut Context<Self>) -> Self;

    /// Get all root-level items
    pub fn items(&self) -> &[T];

    /// Set root-level items (triggers re-render)
    pub fn set_items(&mut self, items: Vec<T>, cx: &mut Context<Self>);

    /// Get the selected item ID
    pub fn selected(&self) -> Option<&T::Id>;

    /// Select an item by ID
    pub fn select(&mut self, id: Option<T::Id>, cx: &mut Context<Self>);

    /// Check if an item is expanded
    pub fn is_expanded(&self, id: &T::Id) -> bool;

    /// Expand an item
    pub fn expand(&mut self, id: T::Id, cx: &mut Context<Self>);

    /// Collapse an item
    pub fn collapse(&mut self, id: &T::Id, cx: &mut Context<Self>);

    /// Toggle expand/collapse
    pub fn toggle_expanded(&mut self, id: &T::Id, cx: &mut Context<Self>);

    /// Expand all items
    pub fn expand_all(&mut self, cx: &mut Context<Self>);

    /// Collapse all items
    pub fn collapse_all(&mut self, cx: &mut Context<Self>);

    /// Set filter text (filters visible items)
    pub fn set_filter(&mut self, filter: String, cx: &mut Context<Self>);

    /// Get current filter
    pub fn filter(&self) -> &str;

    /// Get visible items (flattened, respecting expand state and filter)
    pub fn visible_items(&self) -> Vec<(T, usize)>; // (item, depth)

    /// Scroll to item
    pub fn scroll_to(&mut self, id: &T::Id, cx: &mut Context<Self>);
}
```

## Events

```rust
#[derive(Debug, Clone)]
pub enum TreeEvent<Id> {
    /// Item was selected
    Selected { id: Id },

    /// Item was expanded
    Expanded { id: Id },

    /// Item was collapsed
    Collapsed { id: Id },

    /// Item was double-clicked
    Activated { id: Id },

    /// Context menu requested for item
    ContextMenu { id: Id, position: Point<Pixels> },
}

impl<T: TreeItem> EventEmitter<TreeEvent<T::Id>> for Tree<T> {}
```

## Actions

```rust
actions!(tree, [
    SelectPrevious,     // Up arrow
    SelectNext,         // Down arrow
    ExpandSelected,     // Right arrow
    CollapseSelected,   // Left arrow
    ActivateSelected,   // Enter
    ExpandAll,          // Cmd+Shift+Right
    CollapseAll,        // Cmd+Shift+Left
]);
```

## SchemaItem Implementation

```rust
#[derive(Clone)]
pub enum SchemaItem {
    Connection { id: Uuid, name: String, databases: Vec<SchemaItem> },
    Database { name: String, schemas: Vec<SchemaItem> },
    Schema { name: String, tables: Vec<SchemaItem>, views: Vec<SchemaItem>, functions: Vec<SchemaItem> },
    Table { name: String, columns: Vec<SchemaItem> },
    View { name: String, columns: Vec<SchemaItem> },
    Function { name: String, signature: String },
    Column { name: String, data_type: String, nullable: bool, primary_key: bool },
}

impl TreeItem for SchemaItem {
    type Id = String; // Path-based ID: "conn/db/schema/table/column"

    fn id(&self) -> String { /* construct path */ }
    fn label(&self) -> SharedString { /* name */ }
    fn icon(&self) -> Option<IconName> {
        Some(match self {
            Self::Connection { .. } => IconName::Database,
            Self::Database { .. } => IconName::Database,
            Self::Schema { .. } => IconName::Schema,
            Self::Table { .. } => IconName::Table,
            Self::View { .. } => IconName::View,
            Self::Function { .. } => IconName::Function,
            Self::Column { primary_key: true, .. } => IconName::Key,
            Self::Column { .. } => IconName::Column,
        })
    }
    fn children(&self) -> Option<&[Self]> { /* return children slice */ }
}
```

## Requirements Mapping

| Requirement | Method/Event |
|-------------|--------------|
| FR-017 | `SchemaItem` enum with hierarchy |
| FR-018 | `toggle_expanded()`, `Expanded`/`Collapsed` events |
| FR-019 | `set_filter()` for text search |
| FR-020 | `ContextMenu` event |

## Virtualization

Tree uses `UniformList` internally for virtualization:

```rust
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let visible = self.visible_items();

    uniform_list(
        "tree-items",
        visible.len(),
        |range, window, cx| {
            range.map(|i| self.render_item(&visible[i], cx)).collect()
        },
    )
    .track_scroll(&self.scroll_handle)
}
```

Supports 1000+ items at 60fps per SC-004.
