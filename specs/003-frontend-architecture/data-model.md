# Data Model: Frontend Architecture

**Feature Branch**: `003-frontend-architecture`
**Generated**: 2026-01-21

## Overview

This document defines the data structures and entities for the Tusk frontend architecture. These are Rust structs implementing GPUI's Render trait, not database models.

---

## 1. Workspace

The root UI component containing all visual elements.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| state | Entity<TuskState> | Reference to application state |
| left_dock | Entity<Dock> | Left dock (schema browser) |
| right_dock | Option<Entity<Dock>> | Optional right dock (object details) |
| bottom_dock | Entity<Dock> | Bottom dock (results, messages) |
| center | Entity<PaneGroup> | Main content area with editor panes |
| status_bar | Entity<StatusBar> | Status bar at bottom |
| focus_handle | FocusHandle | Current focus tracking |
| bounds | Bounds<Pixels> | Workspace dimensions |

### Relationships
- Contains 1 TuskState (read-only reference)
- Contains 2-3 Dock entities
- Contains 1 PaneGroup
- Contains 1 StatusBar

### State Transitions
- N/A (static container)

### Validation Rules
- Left dock and bottom dock always exist
- Right dock is optional

---

## 2. Dock

Collapsible panel container at workspace edges.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| position | DockPosition | Left, Right, or Bottom |
| size | Pixels | Current width (side) or height (bottom) |
| min_size | Pixels | Minimum allowed size |
| max_size | Pixels | Maximum allowed size |
| visible | bool | Collapsed or expanded state |
| panels | Vec<Entity<dyn Panel>> | Registered panels |
| active_panel_index | usize | Currently visible panel |
| focus_handle | FocusHandle | Focus tracking |

### DockPosition Enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockPosition {
    Left,
    Right,
    Bottom,
}
```

### Size Constraints
| Position | Min Size | Max Size |
|----------|----------|----------|
| Left | 120px | 600px |
| Right | 120px | 600px |
| Bottom | 100px | 50% viewport height |

### State Transitions
- `visible: true` ↔ `visible: false` (toggle)
- `size` changes within [min_size, max_size]

### Validation Rules
- FR-005: `size >= min_size`
- FR-005/FR-006: `size <= max_size`
- FR-004: `size` persists across sessions

---

## 3. PaneGroup

Hierarchical structure managing multiple panes with split support.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| root | PaneNode | Root of pane tree |
| active_pane | Entity<Pane> | Currently focused pane |

### PaneNode Enum
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

### Axis Enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,  // Side by side
    Vertical,    // Stacked
}
```

### State Transitions
- `Single` → `Split` (on split operation)
- `Split` → `Single` (when only one pane remains)
- `ratios` change during resize

### Validation Rules
- FR-009: Splits allowed in both directions
- `ratios` sum to 1.0
- At least one pane always exists

---

## 4. Pane

Container for tabs within the workspace.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| tabs | Vec<TabItem> | Ordered list of tabs |
| editors | HashMap<Uuid, Entity<QueryEditor>> | Editor instances by tab ID |
| active_tab_index | usize | Currently active tab |
| focus_handle | FocusHandle | Focus tracking |

### State Transitions
- `tabs` grows (add tab)
- `tabs` shrinks (close tab)
- `active_tab_index` changes (switch tab)

### Validation Rules
- FR-008: Multiple tabs per pane allowed
- FR-013: Empty state shown when `tabs.is_empty()`
- `active_tab_index < tabs.len()`

---

## 5. TabItem

Metadata for a single tab.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| id | Uuid | Unique tab identifier |
| title | SharedString | Display name |
| icon | Option<IconName> | Tab icon |
| dirty | bool | Unsaved changes indicator |
| closable | bool | Whether tab can be closed |

### State Transitions
- `dirty: false` → `dirty: true` (content modified)
- `dirty: true` → `dirty: false` (content saved)

### Validation Rules
- FR-010: `dirty` shown as indicator when true
- FR-011: Prompt on close when `dirty && closable`

---

## 6. Panel (Trait)

Interface for dock panel content.

### Required Methods
| Method | Signature | Description |
|--------|-----------|-------------|
| panel_id | `fn panel_id(&self) -> &'static str` | Unique identifier |
| title | `fn title(&self, cx: &App) -> SharedString` | Display name |
| icon | `fn icon(&self) -> IconName` | Panel tab icon |
| focus | `fn focus(&self, cx: &mut App)` | Focus primary element |

### Optional Methods (with defaults)
| Method | Signature | Default |
|--------|-----------|---------|
| closable | `fn closable(&self) -> bool` | `true` |
| is_dirty | `fn is_dirty(&self, cx: &App) -> bool` | `false` |

### PanelEvent Enum
```rust
#[derive(Debug, Clone)]
pub enum PanelEvent {
    Focus,
    Close,
    ActivateTab(usize),
}
```

### Implementors
- SchemaBrowserPanel
- ResultsPanel
- MessagesPanel

---

## 7. TreeItem (Trait)

Interface for tree node data.

### Required Methods
| Method | Signature | Description |
|--------|-----------|-------------|
| id | `fn id(&self) -> Self::Id` | Unique identifier |
| label | `fn label(&self) -> SharedString` | Display text |
| icon | `fn icon(&self) -> Option<IconName>` | Node icon |
| children | `fn children(&self) -> Option<&[Self]>` | Child items |

### Optional Methods
| Method | Signature | Default |
|--------|-----------|---------|
| is_expandable | `fn is_expandable(&self) -> bool` | `children.is_some() && !children.is_empty()` |

### Associated Type
```rust
type Id: Clone + Eq + Hash;
```

---

## 8. Tree<T: TreeItem>

Generic tree view component.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| items | Vec<T> | Root-level items |
| expanded | HashSet<T::Id> | Expanded node IDs |
| selected | Option<T::Id> | Currently selected item |
| focus_handle | FocusHandle | Focus tracking |

### State Transitions
- `expanded` gains/loses IDs (expand/collapse)
- `selected` changes (selection)

### Validation Rules
- FR-018: Expand/collapse indicators shown
- FR-019: Filtering applied to visible items

---

## 9. StatusBar

Bottom status bar showing connection and query state.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| state | Entity<TuskState> | Application state reference |

### Displayed Information
- Connection status (connected/disconnected)
- Database name when connected
- Query execution state and timing
- Row count from last query
- Cursor position

### Validation Rules
- FR-021: Connection status always visible
- FR-022: Database name shown when connected
- FR-023: Query state updates in real-time
- FR-024: Left/right alignment supported

---

## 10. Button

Reusable button component.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| label | Option<SharedString> | Button text |
| icon | Option<IconName> | Button icon |
| icon_position | IconPosition | Left or Right |
| variant | ButtonVariant | Visual style |
| size | ButtonSize | Dimensions |
| disabled | bool | Interaction disabled |
| loading | bool | Loading state |
| on_click | Option<Callback> | Click handler |

### ButtonVariant Enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}
```

### ButtonSize Enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Small,    // 28px height
    #[default]
    Medium,   // 32px height
    Large,    // 40px height
}
```

### Validation Rules
- FR-025: All variants render distinctly
- FR-026: All sizes render correctly
- FR-027: Disabled state visually indicated

---

## 11. TextInput

Text input field component.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| value | String | Current text |
| placeholder | SharedString | Placeholder text |
| disabled | bool | Interaction disabled |
| password | bool | Mask input |
| focus_handle | FocusHandle | Focus tracking |
| on_change | Option<Callback> | Change handler |
| on_submit | Option<Callback> | Enter key handler |

### Validation Rules
- FR-028: Placeholder shown when empty
- FR-029: Change events emitted on input

---

## 12. Select<T>

Dropdown selection component.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| options | Vec<SelectOption<T>> | Available choices |
| selected | Option<T> | Current selection |
| placeholder | SharedString | Placeholder text |
| open | bool | Dropdown expanded |
| focus_handle | FocusHandle | Focus tracking |
| on_change | Option<Callback> | Selection handler |

### SelectOption<T> Structure
```rust
pub struct SelectOption<T: Clone> {
    pub value: T,
    pub label: SharedString,
    pub disabled: bool,
}
```

### Validation Rules
- FR-030: Single selection supported
- FR-031: Keyboard navigation (arrows, enter, escape)

---

## 13. Modal

Dialog overlay component.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| title | SharedString | Header title |
| content | AnyElement | Body content |
| actions | Vec<ModalAction> | Footer buttons |
| width | Pixels | Modal width |
| closable | bool | Show close button |
| focus_handle | FocusHandle | Focus trapping |

### ModalAction Structure
```rust
pub struct ModalAction {
    pub label: SharedString,
    pub variant: ButtonVariant,
    pub handler: Box<dyn Fn(&mut App)>,
}
```

### ModalEvent Enum
```rust
#[derive(Debug, Clone)]
pub enum ModalEvent {
    Close,
}
```

### Validation Rules
- FR-032: Renders above all content with backdrop
- FR-033: Focus trapped within modal
- FR-034: Escape closes modal
- FR-035: Header, body, footer sections supported

---

## 14. ContextMenu

Right-click context menu.

### Fields
| Field | Type | Description |
|-------|------|-------------|
| items | Vec<ContextMenuItem> | Menu items |
| position | Point<Pixels> | Screen position |
| focus_handle | FocusHandle | Focus tracking |

### ContextMenuItem Enum
```rust
pub enum ContextMenuItem {
    Action {
        label: SharedString,
        icon: Option<IconName>,
        shortcut: Option<SharedString>,
        disabled: bool,
        handler: Box<dyn Fn(&mut App)>,
    },
    Separator,
    Submenu {
        label: SharedString,
        icon: Option<IconName>,
        items: Vec<ContextMenuItem>,
    },
}
```

### ContextMenuEvent Enum
```rust
#[derive(Debug, Clone)]
pub enum ContextMenuEvent {
    Close,
}
```

### Validation Rules
- FR-036: Appears at cursor position
- FR-037: Shortcuts shown aligned right
- FR-038: Submenus supported
- FR-039: Closes on click outside

---

## 15. IconName Enum

Complete icon set for UI elements.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconName {
    // Navigation
    ChevronRight, ChevronDown, ChevronLeft, ChevronUp,

    // Actions
    Plus, Close, Search, Refresh, Play, Stop, Save, Copy, Paste,

    // Objects
    Database, Table, Column, Key, Index, View, Function, Schema,
    Folder, File, Code,

    // Status
    Check, Warning, Error, Info,

    // UI
    Menu, Settings, VerticalDots, HorizontalDots,
}
```

---

## 16. IconSize Enum

Standard icon sizes.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconSize {
    XSmall,  // 12px
    Small,   // 14px
    #[default]
    Medium,  // 16px
    Large,   // 20px
    XLarge,  // 24px
}
```

### Validation Rules
- FR-044: Consistent icons across UI
- FR-045: All sizes supported
- FR-046: Custom colors via styling

---

## 17. SpinnerSize Enum

Loading spinner sizes.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerSize {
    Small,   // 14px
    #[default]
    Medium,  // 20px
    Large,   // 32px
}
```

---

## Entity Relationship Diagram

```
┌─────────────┐
│  Workspace  │
├─────────────┤
│ state ──────┼──────────────────────────────────┐
│ left_dock   │──┐                               │
│ right_dock  │──┼──┐                            │
│ bottom_dock │──┤  │                            ▼
│ center ─────┼──┼──┼──────────────────┐    ┌──────────┐
│ status_bar  │  │  │                  │    │TuskState │
└─────────────┘  │  │                  │    └──────────┘
                 │  │                  │
                 ▼  ▼                  ▼
            ┌────────┐           ┌───────────┐
            │  Dock  │───┐       │ PaneGroup │
            ├────────┤   │       ├───────────┤
            │panels[]│   │       │ root      │──┐
            └────────┘   │       │active_pane│  │
                         │       └───────────┘  │
                         ▼                      ▼
                   ┌─────────┐            ┌──────────┐
                   │  Panel  │            │ PaneNode │
                   │ (trait) │            ├──────────┤
                   └─────────┘            │ Single   │──┐
                         ▲                │ Split    │  │
                         │                └──────────┘  │
            ┌────────────┼───────────┐                  │
            │            │           │                  ▼
   ┌─────────────┐ ┌───────────┐ ┌──────────┐     ┌──────┐
   │SchemaBrowser│ │  Results  │ │ Messages │     │ Pane │
   │   Panel     │ │   Panel   │ │  Panel   │     ├──────┤
   └─────────────┘ └───────────┘ └──────────┘     │tabs[]│──┐
         │                                        │editors│  │
         ▼                                        └──────┘  │
   ┌───────────┐                                            │
   │ Tree<T>   │                                            ▼
   └───────────┘                                      ┌─────────┐
                                                      │ TabItem │
                                                      └─────────┘
```

---

## Persistence Model

### Persisted to SQLite (via TuskState/LocalStorage)
- Dock sizes and visibility states
- Workspace layout (which panes are split)
- Active tab per pane
- Recently closed tabs (for reopen feature)

### Not Persisted (Session-only)
- Tab content (editors manage their own state)
- Modal/context menu state
- Focus state
- Scroll positions

### Persistence Format
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

#[derive(Serialize, Deserialize)]
pub enum PaneLayout {
    Single,
    Split {
        axis: Axis,
        ratios: Vec<f32>,
        children: Vec<PaneLayout>,
    },
}
```
