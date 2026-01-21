//! Tusk application root component.

use gpui::{AppContext, Context, Entity, IntoElement, Render, Window};
use tusk_ui::key_bindings::register_key_bindings;
use tusk_ui::Workspace;

/// Root application component that manages the main window.
pub struct TuskApp {
    workspace: Entity<Workspace>,
}

impl TuskApp {
    /// Create a new TuskApp instance with a workspace.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Register global key bindings
        register_key_bindings(cx);

        // Create the workspace
        let workspace = cx.new(|cx| Workspace::new(window, cx));

        Self { workspace }
    }
}

impl Render for TuskApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.workspace.clone()
    }
}
