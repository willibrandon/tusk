//! UI components and theming for Tusk PostgreSQL client.
//!
//! This crate provides the core UI building blocks for Tusk, including:
//! - Theme system with Catppuccin color palette
//! - Layout utilities (spacing, sizing, radius)
//! - Core components (buttons, icons, spinners)
//! - Workspace architecture (docks, panes, panels)
//! - Keyboard bindings and actions

// Core modules
pub mod button;
pub mod confirm_dialog;
pub mod context_menu;
pub mod dock;
pub mod icon;
pub mod key_bindings;
pub mod layout;
pub mod modal;
pub mod pane;
pub mod panel;
pub mod panels;
pub mod resizer;
pub mod select;
pub mod spinner;
pub mod status_bar;
pub mod text_input;
pub mod theme;
pub mod tooltip;
pub mod tree;
pub mod workspace;

// Re-exports for convenience
pub use button::{Button, ButtonSize, ButtonVariant, IconPosition};
pub use confirm_dialog::{ConfirmDialog, ConfirmDialogEvent, ConfirmDialogKind};
pub use context_menu::{ContextMenu, ContextMenuEvent, ContextMenuItem, ContextMenuLayer};
pub use dock::{Dock, DockEvent};
pub use icon::{Icon, IconName, IconSize};
pub use key_bindings::register_key_bindings;
pub use layout::{radius, sizes, spacing};
pub use modal::{Modal, ModalAction, ModalEvent, ModalLayer};
pub use pane::{
    Pane, PaneEvent, PaneGroup, PaneGroupEvent, PaneLayout, PaneNode, SerializedAxis, TabItem,
};
pub use panel::{DockPosition, Focusable, Panel, PanelEntry, PanelEvent, PanelHandle};
pub use panels::{
    database_schema_to_tree, Message, MessageSeverity, MessagesPanel, ResultsPanel, ResultsState,
    SchemaBrowserPanel, SchemaItem,
};
pub use resizer::Resizer;
pub use select::{Select, SelectEvent, SelectOption};
pub use spinner::{Spinner, SpinnerSize};
pub use status_bar::{ConnectionStatus, ExecutionState, StatusBar};
pub use text_input::{
    register_text_input_bindings, Copy, Cut, Paste, Redo, SelectAll, TextInput, TextInputEvent,
    Undo,
};
pub use theme::{ThemeColors, TuskTheme};
pub use tooltip::Tooltip;
pub use tree::{Tree, TreeEvent, TreeItem, VisibleEntry};
pub use workspace::{Workspace, WorkspaceEvent, WorkspaceState};
