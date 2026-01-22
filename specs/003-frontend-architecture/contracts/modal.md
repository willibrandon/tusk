# Contract: Modal System

**Component**: `Modal`
**Module**: `tusk_ui::modal`
**Implements**: `Render`, `Focusable`, `EventEmitter<ModalEvent>`

## Public Interface

```rust
pub struct Modal {
    id: ElementId,
    title: SharedString,
    children: SmallVec<[AnyElement; 2]>,
    actions: Vec<ModalAction>,
    width: Pixels,
    closable: bool,
    focus_handle: FocusHandle,
}

impl Modal {
    /// Create a new modal with title
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self;

    /// Set modal width (default: 480px)
    pub fn width(self, width: Pixels) -> Self;

    /// Set whether modal can be closed via X button or Escape (default: true)
    pub fn closable(self, closable: bool) -> Self;

    /// Add body content
    pub fn child(self, child: impl IntoElement) -> Self;

    /// Add multiple body children
    pub fn children(self, children: impl IntoIterator<Item = impl IntoElement>) -> Self;

    /// Add footer action button
    pub fn action(self, action: ModalAction) -> Self;

    /// Add multiple footer actions
    pub fn actions(self, actions: impl IntoIterator<Item = ModalAction>) -> Self;
}
```

## ModalAction Structure

```rust
pub struct ModalAction {
    pub label: SharedString,
    pub variant: ButtonVariant,
    pub disabled: bool,
    pub handler: Box<dyn Fn(&mut App)>,
}

impl ModalAction {
    pub fn new(label: impl Into<SharedString>, handler: impl Fn(&mut App) + 'static) -> Self;
    pub fn variant(self, variant: ButtonVariant) -> Self;
    pub fn disabled(self, disabled: bool) -> Self;
    pub fn primary(label: impl Into<SharedString>, handler: impl Fn(&mut App) + 'static) -> Self;
    pub fn cancel(handler: impl Fn(&mut App) + 'static) -> Self;
}
```

## Events

```rust
#[derive(Debug, Clone)]
pub enum ModalEvent {
    /// Modal close requested (Escape, backdrop click, close button)
    Close,
}

impl EventEmitter<ModalEvent> for Modal {}
```

## Actions

```rust
actions!(modal, [
    Dismiss,        // Escape
    Confirm,        // Enter (triggers primary action)
]);
```

## Usage Pattern

```rust
// Create and show a confirmation modal
let modal = Modal::new("confirm-delete", "Delete Table?")
    .width(px(400.0))
    .child(
        div().child("Are you sure you want to delete this table? This action cannot be undone.")
    )
    .action(ModalAction::cancel(|cx| {
        // Close modal
    }))
    .action(ModalAction::primary("Delete", |cx| {
        // Perform delete
    }).variant(ButtonVariant::Danger));
```

## Modal Layer

```rust
/// Modal layer manages overlay and focus trapping
pub struct ModalLayer {
    active_modals: Vec<Entity<Modal>>,
}

impl ModalLayer {
    pub fn new() -> Self;

    /// Show a modal (pushes to stack)
    pub fn show(&mut self, modal: Entity<Modal>, cx: &mut App);

    /// Dismiss the topmost modal
    pub fn dismiss(&mut self, cx: &mut App);

    /// Check if any modal is open
    pub fn has_modal(&self) -> bool;
}

impl Global for ModalLayer {}
```

## Requirements Mapping

| Requirement | Implementation |
|-------------|----------------|
| FR-032 | Modal renders above all content via `ModalLayer` |
| FR-033 | Focus trapped via `track_focus()` + `focus_handle` |
| FR-034 | `Dismiss` action bound to Escape |
| FR-035 | Header (title), body (children), footer (actions) |

## Focus Trapping

Focus is trapped within modal by:
1. Storing previous focus before modal opens
2. Using `focus_handle.focus(cx)` on modal open
3. Tab navigation cycles within modal content
4. Restoring previous focus on close

```rust
impl Modal {
    fn handle_tab(&mut self, _: &Tab, cx: &mut Context<Self>) {
        // Cycle focus within modal
        let focusable = self.collect_focusable_children();
        // ... cycle logic
    }
}
```

## Stacking

Multiple modals stack with increasing z-index:
- First modal: z-index 1000
- Second modal: z-index 1001
- etc.

Escape dismisses only the topmost modal.
