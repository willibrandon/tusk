# Research: Frontend Architecture

**Feature Branch**: `003-frontend-architecture`
**Generated**: 2026-01-21

## Overview

This document consolidates research findings for implementing the GPUI frontend architecture for Tusk. All patterns and APIs verified against the authoritative source: `/Users/brandon/src/zed`.

---

## 1. GPUI Render Trait Patterns

### Decision: Use Render trait for stateful views, RenderOnce for stateless components

**Rationale**: GPUI distinguishes between:
- `Render` trait: For stateful views that need mutable access (`&mut self`)
- `RenderOnce` trait: For stateless configuration objects consumed once

**Alternatives considered**:
- Single trait approach: Would require boxing for all returns
- Custom trait: Unnecessary complexity when GPUI patterns are well-established

### Render Trait Signature
```rust
pub trait Render: 'static + Sized {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement;
}
```

### RenderOnce Trait Signature
```rust
pub trait RenderOnce: 'static {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement;
}
```

**Source**: `/Users/brandon/src/zed/crates/gpui/src/view.rs`, `/Users/brandon/src/zed/crates/gpui/examples/hello_world.rs`

---

## 2. Entity and Global State Management

### Decision: Use Entity<T> for component handles, Global trait for application-wide state

**Rationale**:
- `Entity<T>` provides automatic lifecycle management and dependency tracking
- `Global` trait enables type-safe global access from any component
- Matches patterns already established in tusk_core's `TuskState`

**Source**: `/Users/brandon/src/zed/crates/gpui/src/global.rs`, `/Users/brandon/src/zed/crates/gpui/src/view.rs`

### Entity Operations
```rust
// Read access
let value = entity.read(cx);

// Mutable update (triggers re-render)
entity.update(cx, |view, cx| {
    view.modify_state();
    cx.notify();
});
```

### Global Pattern
```rust
impl Global for TuskState {}

// Access anywhere
let state = cx.global::<TuskState>();
```

**Note**: `TuskState` already implements `Global` conditionally (`#[cfg(feature = "gpui")]`) in `/Users/brandon/src/tusk/crates/tusk_core/src/state.rs`.

---

## 3. Focus Management with FocusHandle

### Decision: Create FocusHandle per focusable component, use track_focus() for element binding

**Rationale**: GPUI's focus system is built around FocusHandle which:
- Provides `focus()`, `is_focused()`, `contains_focused()` methods
- Integrates with key dispatch (actions dispatched to focused element)
- Supports focus containment for modals

**Source**: `/Users/brandon/src/zed/crates/gpui/src/window.rs` (lines 332-467)

### Focus Pattern
```rust
pub struct Pane {
    focus_handle: FocusHandle,
    // ...
}

impl Pane {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            // ...
        }
    }
}

impl Render for Pane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .key_context("Pane")
            // ...
    }
}
```

### Focusable Trait
```rust
pub trait Focusable: 'static {
    fn focus_handle(&self, cx: &App) -> FocusHandle;
}
```

---

## 4. Event System with EventEmitter

### Decision: Use EventEmitter<E> for component communication, Context::subscribe() for listeners

**Rationale**: GPUI's event system provides:
- Type-safe event emission and handling
- Subscription lifecycle management
- Decoupled component communication

**Source**: `/Users/brandon/src/zed/crates/gpui/src/gpui.rs`, `/Users/brandon/src/zed/crates/gpui/src/subscription.rs`

### EventEmitter Pattern
```rust
#[derive(Debug, Clone)]
pub enum DockEvent {
    Resize { delta: Pixels },
    PanelChanged,
}

impl EventEmitter<DockEvent> for Dock {}

// Emit event
cx.emit(DockEvent::Resize { delta: px(10.0) });

// Subscribe
let subscription = cx.subscribe(&dock_entity, |this, _dock, event: &DockEvent, cx| {
    match event {
        DockEvent::Resize { delta } => this.handle_resize(*delta, cx),
        DockEvent::PanelChanged => cx.notify(),
    }
});
```

### ManagedView for Modals
```rust
pub trait ManagedView: Focusable + EventEmitter<DismissEvent> + Render {}
```

Modals should implement `ManagedView` to integrate with GPUI's overlay system.

**Source**: `/Users/brandon/src/zed/crates/ui/src/components/popover_menu.rs`

---

## 5. Actions and Keybindings

### Decision: Use actions! macro for simple actions, derive Action for complex actions with data

**Rationale**: Actions provide:
- Named, type-safe command identifiers
- Serialization support for keybinding configuration
- Integration with key dispatch to focused elements

**Source**: `/Users/brandon/src/zed/crates/gpui/src/action.rs`, `/Users/brandon/src/zed/crates/gpui/src/interactive.rs`

### Simple Actions
```rust
actions!(workspace, [
    ToggleLeftDock,
    ToggleBottomDock,
    NewQueryTab,
    CloseTab,
]);
```

### Complex Actions with Data
```rust
#[derive(Clone, PartialEq, serde::Deserialize, Action)]
#[action(namespace = "editor")]
pub struct ActivateTab {
    pub index: usize,
}
```

### Keybinding Registration
```rust
cx.bind_keys([
    KeyBinding::new("cmd-b", ToggleLeftDock, Some("Workspace")),
    KeyBinding::new("cmd-t", NewQueryTab, Some("Workspace")),
    KeyBinding::new("cmd-w", CloseTab, Some("Workspace")),
]);
```

### Action Handling
```rust
impl Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("Workspace")
            .on_action(cx.listener(Self::handle_toggle_left_dock))
            .on_action(cx.listener(Self::handle_new_query_tab))
            // ...
    }

    fn handle_toggle_left_dock(&mut self, _: &ToggleLeftDock, _window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_dock(DockPosition::Left, cx);
    }
}
```

---

## 6. Animation Patterns

### Decision: Use with_animation() with Animation::new() for loading spinners

**Rationale**: GPUI's animation system provides:
- Declarative animation definitions
- Built-in easing functions
- Element transformation during render

**Source**: `/Users/brandon/src/zed/crates/gpui/examples/animation.rs`, `/Users/brandon/src/zed/crates/gpui/src/elements/animation.rs`

### Spinner Animation
```rust
use gpui::{Animation, AnimationExt, percentage};
use std::time::Duration;

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();
        let size = self.size.pixels();

        div()
            .size(size)
            .rounded_full()
            .border_2()
            .border_color(theme.colors.border)
            .border_t_color(theme.colors.accent)
            .with_animation(
                "spin",
                Animation::new(Duration::from_secs(1))
                    .repeat()
                    .with_easing(gpui::linear),
                move |this, progress| {
                    this.rotate(gpui::Angle::from_radians(progress * std::f32::consts::TAU))
                },
            )
    }
}
```

---

## 7. List Virtualization with UniformList

### Decision: Use UniformList for schema tree and results grid (1000+ items)

**Rationale**:
- Renders only visible items for performance
- Supports smooth scrolling with scroll handle
- Meets performance target: 60fps for 1000+ item lists

**Source**: `/Users/brandon/src/zed/crates/gpui/src/elements/uniform_list.rs`

### UniformList Pattern
```rust
use gpui::{uniform_list, UniformListScrollHandle, ScrollStrategy};

let scroll_handle = UniformListScrollHandle::new();

uniform_list(
    "schema-tree",
    self.visible_items.len(),
    |range, window, cx| {
        range.map(|i| self.render_tree_item(&self.visible_items[i], cx))
            .collect()
    },
)
.track_scroll(&scroll_handle)
.on_scroll(cx.listener(|this, scroll_event, cx| {
    // Handle scroll
}))
```

### Scroll Control
```rust
scroll_handle.scroll_to_item(index, ScrollStrategy::Center);
```

---

## 8. Interactive Element Pattern

### Decision: Use InteractiveElement trait methods for mouse/keyboard events

**Rationale**: GPUI provides a fluent API for event handling on elements.

**Source**: `/Users/brandon/src/zed/crates/gpui/src/elements/div.rs`

### Common Event Handlers
```rust
div()
    .on_click(cx.listener(|this, event: &ClickEvent, cx| {
        // Handle click
    }))
    .on_mouse_down(MouseButton::Left, cx.listener(|this, event, cx| {
        // Handle mouse down
    }))
    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, cx| {
        // Handle drag
    }))
    .on_hover(cx.listener(|this, is_hovered: &bool, cx| {
        this.hovered = *is_hovered;
        cx.notify();
    }))
    .cursor_style(CursorStyle::Pointer)
```

---

## 9. Component Composition

### Decision: Use IntoElement and FluentBuilder for composable components

**Rationale**: GPUI's element system supports fluent method chaining for readable component construction.

**Source**: `/Users/brandon/src/zed/crates/gpui/src/element.rs`, `/Users/brandon/src/zed/crates/ui/src/components/`

### Modal Component Pattern
```rust
#[derive(IntoElement)]
pub struct Modal {
    id: ElementId,
    title: SharedString,
    children: SmallVec<[AnyElement; 2]>,
    actions: Vec<ModalAction>,
}

impl Modal {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self { ... }
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
    pub fn action(mut self, action: ModalAction) -> Self {
        self.actions.push(action);
        self
    }
}
```

### Button Component Pattern
```rust
impl Button {
    pub fn new() -> Self { ... }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self { ... }
    pub fn icon(mut self, icon: IconName) -> Self { ... }
    pub fn variant(mut self, variant: ButtonVariant) -> Self { ... }
    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut App) + 'static) -> Self { ... }
}
```

---

## 10. Context Menu Pattern

### Decision: Use ContextMenuItem enum with Action/Separator/Submenu variants

**Rationale**: Matches Zed's proven context menu architecture.

**Source**: `/Users/brandon/src/zed/crates/ui/src/components/context_menu.rs`

### ContextMenuItem Structure
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

---

## 11. Workspace and Pane Architecture

### Decision: Use PaneGroup with Member enum (Single/Split) for recursive pane layout

**Rationale**: Zed's PaneGroup pattern handles:
- Arbitrary split configurations
- Proportional resizing
- Dynamic pane addition/removal

**Source**: `/Users/brandon/src/zed/crates/workspace/src/pane_group.rs`

### PaneGroup Structure
```rust
pub struct PaneGroup {
    root: PaneNode,
    active_pane: Entity<Pane>,
}

pub enum PaneNode {
    Single(Entity<Pane>),
    Split {
        axis: Axis,
        children: SmallVec<[PaneNode; 2]>,
        ratios: SmallVec<[f32; 2]>,
    },
}
```

---

## 12. Theme System Extension

### Decision: Extend existing TuskTheme with additional colors for new components

**Rationale**: Current theme has basic colors; need to add:
- Tab bar colors (active/inactive/hover)
- Panel/dock backgrounds
- List selection colors
- Modal overlay
- Element/button backgrounds

**Source**: `/Users/brandon/src/tusk/crates/tusk_ui/src/theme.rs`, `/Users/brandon/src/zed/crates/theme/src/`

### Additional Colors Needed
```rust
pub struct ThemeColors {
    // Existing colors...

    // Tab bar
    pub tab_bar_background: Hsla,
    pub tab_active_background: Hsla,
    pub tab_inactive_background: Hsla,
    pub tab_hover_background: Hsla,

    // Panels and docks
    pub panel_background: Hsla,
    pub editor_background: Hsla,
    pub status_bar_background: Hsla,

    // Lists and selections
    pub list_active_selection_background: Hsla,
    pub list_hover_background: Hsla,

    // Buttons and elements
    pub element_background: Hsla,
    pub element_hover: Hsla,
    pub ghost_element_hover: Hsla,
    pub on_accent: Hsla,  // Text on accent backgrounds

    // Inputs
    pub input_background: Hsla,

    // Elevated surfaces (modals, dropdowns)
    pub elevated_surface_background: Hsla,

    // Semantic
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
}
```

---

## 13. Dependency Updates

### Decision: Add smallvec, uuid to tusk_ui dependencies

**Rationale**: Required for:
- `smallvec`: Efficient small collections for PaneNode children
- `uuid`: Tab and pane identification

### Updated Cargo.toml for tusk_ui
```toml
[dependencies]
gpui.workspace = true
smallvec = { version = "1.11", features = ["union"] }
uuid = { version = "1.6", features = ["v4"] }
tusk_core = { path = "../tusk_core" }

[target.'cfg(target_os = "macos")'.dependencies]
core-text.workspace = true
core-graphics.workspace = true
```

---

## Summary

All research items resolved:

| Topic | Decision | Source |
|-------|----------|--------|
| View rendering | Render/RenderOnce traits | gpui/src/view.rs |
| State management | Entity<T> + Global trait | gpui/src/global.rs |
| Focus handling | FocusHandle + track_focus | gpui/src/window.rs |
| Events | EventEmitter + subscribe | gpui/src/subscription.rs |
| Actions | actions! macro + on_action | gpui/src/action.rs |
| Animation | with_animation + Animation | gpui/examples/animation.rs |
| Virtualization | UniformList | gpui/src/elements/uniform_list.rs |
| Interactions | InteractiveElement trait | gpui/src/elements/div.rs |
| Context menus | ContextMenuItem enum | ui/src/components/context_menu.rs |
| Pane layout | PaneGroup with PaneNode | workspace/src/pane_group.rs |
| Theme colors | Extended ThemeColors | theme/src/ |

No NEEDS CLARIFICATION items remain.
