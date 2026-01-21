# Quickstart Guide: Frontend Architecture

**Feature Branch**: `003-frontend-architecture`
**Generated**: 2026-01-21

## Prerequisites

- Rust 1.80+ installed
- Xcode Command Line Tools (macOS)
- Zed reference available at `/Users/brandon/src/zed`

## Quick Setup

```bash
# Clone and checkout branch
cd /Users/brandon/src/tusk
git checkout -b 003-frontend-architecture

# Verify GPUI dependency
grep -A2 'gpui.workspace' crates/tusk_ui/Cargo.toml

# Build to verify setup
cargo build -p tusk_ui
```

## File Structure to Create

```
crates/tusk_ui/src/
├── lib.rs              # Update exports
├── workspace.rs        # NEW
├── dock.rs             # NEW
├── pane.rs             # NEW
├── resizer.rs          # NEW
├── status_bar.rs       # NEW
├── panel.rs            # NEW
├── tree.rs             # NEW
├── icon.rs             # Rename from icons.rs
├── button.rs           # NEW
├── input.rs            # NEW
├── select.rs           # NEW
├── modal.rs            # NEW
├── context_menu.rs     # NEW
├── spinner.rs          # NEW
├── key_bindings.rs     # NEW
├── layout.rs           # NEW
├── theme.rs            # Expand existing
└── panels/
    ├── mod.rs          # NEW
    ├── schema_browser.rs  # NEW
    ├── results.rs      # NEW
    └── messages.rs     # NEW
```

## Implementation Order

### Phase 1: Foundation (Tasks 1-5)
1. Update `Cargo.toml` with dependencies (smallvec, uuid)
2. Extend `theme.rs` with new colors
3. Expand `icon.rs` with full IconName enum
4. Create `spinner.rs` with animation
5. Create `layout.rs` utilities

### Phase 2: Core Components (Tasks 6-10)
6. Create `button.rs`
7. Create `input.rs`
8. Create `select.rs`
9. Create `resizer.rs`
10. Create `key_bindings.rs`

### Phase 3: Workspace Structure (Tasks 11-16)
11. Create `panel.rs` trait
12. Create `dock.rs`
13. Create `pane.rs` with PaneGroup
14. Create `workspace.rs`
15. Create `status_bar.rs`
16. Update `lib.rs` exports

### Phase 4: Advanced Components (Tasks 17-20)
17. Create `modal.rs`
18. Create `context_menu.rs`
19. Create `tree.rs`
20. Create `panels/` module

### Phase 5: Integration (Tasks 21-24)
21. Wire up workspace in `app.rs`
22. Add persistence layer
23. Integration tests
24. Performance validation

## Key Patterns

### Creating a Stateful Component

```rust
use gpui::prelude::*;

pub struct MyComponent {
    value: String,
    focus_handle: FocusHandle,
}

impl MyComponent {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            value: String::new(),
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Render for MyComponent {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .child(self.value.clone())
    }
}

impl Focusable for MyComponent {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
```

### Creating a Stateless Component

```rust
use gpui::prelude::*;

#[derive(IntoElement)]
pub struct MyButton {
    label: SharedString,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut App)>>,
}

impl MyButton {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
        }
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for MyButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        div()
            .px_3()
            .py_1()
            .bg(theme.colors.accent)
            .rounded(px(4.0))
            .child(self.label)
            .when_some(self.on_click, |this, handler| {
                this.on_click(move |event, cx| handler(event, cx))
            })
    }
}
```

### Adding Events

```rust
#[derive(Debug, Clone)]
pub enum MyEvent {
    Changed { value: String },
    Submitted,
}

impl EventEmitter<MyEvent> for MyComponent {}

// Emit event
cx.emit(MyEvent::Changed { value: self.value.clone() });

// Subscribe to events
cx.subscribe(&entity, |this, _emitter, event: &MyEvent, cx| {
    match event {
        MyEvent::Changed { value } => { /* handle */ }
        MyEvent::Submitted => { /* handle */ }
    }
});
```

### Registering Actions

```rust
actions!(my_component, [DoSomething, DoSomethingElse]);

// In render
div()
    .key_context("MyComponent")
    .on_action(cx.listener(Self::handle_do_something))

// Handler
fn handle_do_something(&mut self, _: &DoSomething, _window: &mut Window, cx: &mut Context<Self>) {
    // Handle action
    cx.notify();
}

// Key binding
cx.bind_keys([
    KeyBinding::new("cmd-d", DoSomething, Some("MyComponent")),
]);
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_my_component(cx: &mut TestAppContext) {
        let component = cx.new(|cx| MyComponent::new(cx));

        component.update(cx, |comp, cx| {
            comp.set_value("test".to_string(), cx);
        });

        component.read_with(cx, |comp, _| {
            assert_eq!(comp.value(), "test");
        });
    }
}
```

## Common Issues

### "Entity not found"
Ensure you're not trying to read an entity that has been dropped. Keep entity references alive in parent components.

### "Cannot borrow mutably"
GPUI requires careful borrowing. Use `cx.spawn()` for async operations that need to update state:

```rust
cx.spawn(|this, mut cx| async move {
    // async work
    this.update(&mut cx, |this, cx| {
        // update state
        cx.notify();
    });
}).detach();
```

### Focus not working
Ensure:
1. Component has `FocusHandle`
2. Element uses `.track_focus(&self.focus_handle)`
3. Component implements `Focusable` trait

### Actions not dispatching
Ensure:
1. Element has `.key_context("ContextName")`
2. Action handler registered with `.on_action()`
3. Key binding registered with correct context

## Reference Paths

| What | Where |
|------|-------|
| GPUI core | `/Users/brandon/src/zed/crates/gpui/src/` |
| GPUI examples | `/Users/brandon/src/zed/crates/gpui/examples/` |
| Zed UI components | `/Users/brandon/src/zed/crates/ui/src/` |
| Zed workspace | `/Users/brandon/src/zed/crates/workspace/src/` |
| Existing tusk_ui | `/Users/brandon/src/tusk/crates/tusk_ui/src/` |
| TuskState | `/Users/brandon/src/tusk/crates/tusk_core/src/state.rs` |
