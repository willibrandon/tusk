# Contract: Component Library

**Module**: `tusk_ui` (various submodules)

## Button Component

**File**: `tusk_ui::button`
**Implements**: `RenderOnce`

```rust
pub struct Button {
    label: Option<SharedString>,
    icon: Option<IconName>,
    icon_position: IconPosition,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    loading: bool,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut App)>>,
}

impl Button {
    pub fn new() -> Self;
    pub fn label(self, label: impl Into<SharedString>) -> Self;
    pub fn icon(self, icon: IconName) -> Self;
    pub fn icon_position(self, position: IconPosition) -> Self;
    pub fn variant(self, variant: ButtonVariant) -> Self;
    pub fn size(self, size: ButtonSize) -> Self;
    pub fn disabled(self, disabled: bool) -> Self;
    pub fn loading(self, loading: bool) -> Self;
    pub fn on_click(self, handler: impl Fn(&ClickEvent, &mut App) + 'static) -> Self;
}
```

### Enums

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Small,    // 28px height
    #[default]
    Medium,   // 32px height
    Large,    // 40px height
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconPosition {
    #[default]
    Left,
    Right,
}
```

### Requirements: FR-025, FR-026, FR-027

---

## TextInput Component

**File**: `tusk_ui::input`
**Implements**: `Render`, `Focusable`

```rust
pub struct TextInput {
    value: String,
    placeholder: SharedString,
    disabled: bool,
    password: bool,
    focus_handle: FocusHandle,
    on_change: Option<Box<dyn Fn(&str, &mut App)>>,
    on_submit: Option<Box<dyn Fn(&str, &mut App)>>,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>) -> Self;
    pub fn value(&self) -> &str;
    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>);
    pub fn placeholder(self, text: impl Into<SharedString>) -> Self;
    pub fn disabled(self, disabled: bool) -> Self;
    pub fn password(self, password: bool) -> Self;
    pub fn on_change(self, handler: impl Fn(&str, &mut App) + 'static) -> Self;
    pub fn on_submit(self, handler: impl Fn(&str, &mut App) + 'static) -> Self;
}
```

### Events

```rust
#[derive(Debug, Clone)]
pub enum TextInputEvent {
    Changed { value: String },
    Submitted { value: String },
    Focus,
    Blur,
}

impl EventEmitter<TextInputEvent> for TextInput {}
```

### Requirements: FR-028, FR-029

---

## Select Component

**File**: `tusk_ui::select`
**Implements**: `Render`, `Focusable`

```rust
pub struct Select<T: Clone + PartialEq + 'static> {
    options: Vec<SelectOption<T>>,
    selected: Option<T>,
    placeholder: SharedString,
    open: bool,
    focus_handle: FocusHandle,
    on_change: Option<Box<dyn Fn(&T, &mut App)>>,
}

pub struct SelectOption<T: Clone> {
    pub value: T,
    pub label: SharedString,
    pub disabled: bool,
}

impl<T: Clone + PartialEq + 'static> Select<T> {
    pub fn new(options: Vec<SelectOption<T>>, cx: &mut Context<Self>) -> Self;
    pub fn selected(&self) -> Option<&T>;
    pub fn set_selected(&mut self, value: Option<T>, cx: &mut Context<Self>);
    pub fn placeholder(self, text: impl Into<SharedString>) -> Self;
    pub fn on_change(self, handler: impl Fn(&T, &mut App) + 'static) -> Self;
}
```

### Events

```rust
#[derive(Debug, Clone)]
pub enum SelectEvent<T> {
    Changed { value: T },
    Opened,
    Closed,
}

impl<T: Clone + PartialEq + 'static> EventEmitter<SelectEvent<T>> for Select<T> {}
```

### Actions

```rust
actions!(select, [
    Open,           // Space, Enter, Down
    Close,          // Escape
    SelectNext,     // Down arrow
    SelectPrevious, // Up arrow
    Confirm,        // Enter
]);
```

### Requirements: FR-030, FR-031

---

## StatusBar Component

**File**: `tusk_ui::status_bar`
**Implements**: `Render`

```rust
pub struct StatusBar {
    state: Entity<TuskState>,
}

impl StatusBar {
    pub fn new(state: Entity<TuskState>, cx: &mut Context<Self>) -> Self;
}
```

### Displayed Information

| Section | Content | Alignment |
|---------|---------|-----------|
| Connection | Status icon + name or "Disconnected" | Left |
| Database | Current database name | Left |
| Query State | Spinner + "Executing..." or timing | Right |
| Row Count | "N rows" from last query | Right |
| Cursor | Line:Column position | Right |

### Requirements: FR-021, FR-022, FR-023, FR-024

---

## Icon Component

**File**: `tusk_ui::icon`
**Implements**: `RenderOnce`

```rust
pub struct Icon {
    name: IconName,
    size: IconSize,
    color: Option<Hsla>,
}

impl Icon {
    pub fn new(name: IconName) -> Self;
    pub fn size(self, size: IconSize) -> Self;
    pub fn color(self, color: Hsla) -> Self;
}
```

### IconName Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconName {
    // Navigation
    ChevronRight, ChevronDown, ChevronLeft, ChevronUp,
    // Actions
    Plus, Close, Search, Refresh, Play, Stop, Save, Copy, Paste,
    // Objects
    Database, Table, Column, Key, Index, View, Function, Schema, Folder, File, Code,
    // Status
    Check, Warning, Error, Info,
    // UI
    Menu, Settings, VerticalDots, HorizontalDots,
}
```

### IconSize Enum

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

impl IconSize {
    pub fn pixels(&self) -> Pixels {
        match self {
            Self::XSmall => px(12.0),
            Self::Small => px(14.0),
            Self::Medium => px(16.0),
            Self::Large => px(20.0),
            Self::XLarge => px(24.0),
        }
    }
}
```

### Requirements: FR-044, FR-045, FR-046

---

## Spinner Component

**File**: `tusk_ui::spinner`
**Implements**: `RenderOnce`

```rust
pub struct Spinner {
    size: SpinnerSize,
}

impl Spinner {
    pub fn new() -> Self;
    pub fn size(self, size: SpinnerSize) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerSize {
    Small,   // 14px
    #[default]
    Medium,  // 20px
    Large,   // 32px
}
```

Uses `with_animation()` for continuous rotation.
