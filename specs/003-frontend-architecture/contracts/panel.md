# Contract: Panel Trait

**Trait**: `Panel`
**Module**: `tusk_ui::panel`

## Trait Definition

```rust
/// Trait for dock panel content
pub trait Panel: Render + Focusable + EventEmitter<PanelEvent> {
    /// Unique identifier for this panel type
    fn panel_id(&self) -> &'static str;

    /// Display title for the panel tab
    fn title(&self, cx: &App) -> SharedString;

    /// Icon for the panel tab
    fn icon(&self) -> IconName;

    /// Focus the primary interactive element
    fn focus(&self, cx: &mut App);

    /// Whether the panel can be closed (default: true)
    fn closable(&self) -> bool { true }

    /// Whether the panel has unsaved changes (default: false)
    fn is_dirty(&self, cx: &App) -> bool { false }

    /// Preferred position for this panel (default: Left)
    fn position(&self) -> DockPosition { DockPosition::Left }
}
```

## Panel Event

```rust
#[derive(Debug, Clone)]
pub enum PanelEvent {
    /// Panel requests focus
    Focus,

    /// Panel requests close
    Close,

    /// Panel wants to activate a specific tab
    ActivateTab(usize),
}
```

## Built-in Panel Implementations

### SchemaBrowserPanel

```rust
pub struct SchemaBrowserPanel {
    tree: Tree<SchemaItem>,
    filter: String,
    focus_handle: FocusHandle,
}

impl Panel for SchemaBrowserPanel {
    fn panel_id(&self) -> &'static str { "schema_browser" }
    fn title(&self, _cx: &App) -> SharedString { "Schema".into() }
    fn icon(&self) -> IconName { IconName::Database }
    fn position(&self) -> DockPosition { DockPosition::Left }
}
```

### ResultsPanel

```rust
pub struct ResultsPanel {
    // Results display component
    focus_handle: FocusHandle,
}

impl Panel for ResultsPanel {
    fn panel_id(&self) -> &'static str { "results" }
    fn title(&self, _cx: &App) -> SharedString { "Results".into() }
    fn icon(&self) -> IconName { IconName::Table }
    fn position(&self) -> DockPosition { DockPosition::Bottom }
}
```

### MessagesPanel

```rust
pub struct MessagesPanel {
    messages: Vec<Message>,
    focus_handle: FocusHandle,
}

impl Panel for MessagesPanel {
    fn panel_id(&self) -> &'static str { "messages" }
    fn title(&self, _cx: &App) -> SharedString { "Messages".into() }
    fn icon(&self) -> IconName { IconName::Info }
    fn position(&self) -> DockPosition { DockPosition::Bottom }
}
```

## Requirements Mapping

| Requirement | Implementation |
|-------------|----------------|
| FR-014 | `Panel` trait with required methods |
| FR-015 | `panel_id()`, `icon()`, `position()` |
| FR-016 | Panel visibility via Dock toggle |

## Usage Pattern

```rust
// Register panel with dock
let schema_panel = cx.new(|cx| SchemaBrowserPanel::new(cx));
workspace.left_dock().update(cx, |dock, cx| {
    dock.add_panel(Box::new(schema_panel), cx);
});

// Subscribe to panel events
cx.subscribe(&panel_entity, |workspace, _panel, event: &PanelEvent, cx| {
    match event {
        PanelEvent::Focus => { /* handle focus */ }
        PanelEvent::Close => { /* handle close */ }
        PanelEvent::ActivateTab(idx) => { /* activate tab */ }
    }
});
```
