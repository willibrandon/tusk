//! Tusk application root component.

use gpui::{AppContext, Context, Entity, Global, IntoElement, Render, Window};
use tusk_ui::key_bindings::register_key_bindings;
use tusk_ui::{register_text_input_bindings, ContextMenuLayer, ModalLayer, Workspace};

/// Global reference to the workspace entity for menu action dispatching.
pub struct WorkspaceHandle(pub Entity<Workspace>);

impl Global for WorkspaceHandle {}

/// Root application component that manages the main window.
pub struct TuskApp {
    workspace: Entity<Workspace>,
}

impl TuskApp {
    /// Create a new TuskApp instance with a workspace.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Register global key bindings
        register_key_bindings(cx);
        register_text_input_bindings(cx);

        // Register ModalLayer as global for modal management (T093)
        cx.set_global(ModalLayer::new());

        // Register ContextMenuLayer as global for context menu management (T103)
        cx.set_global(ContextMenuLayer::new());

        // Create the workspace
        let workspace = cx.new(|cx| Workspace::new(window, cx));

        // Store workspace handle globally for menu action dispatching
        cx.set_global(WorkspaceHandle(workspace.clone()));

        // App starts disconnected - user connects via File > New Connection (Cmd+N)

        Self { workspace }
    }
}

impl Render for TuskApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.workspace.clone()
    }
}
