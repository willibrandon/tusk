//! Tusk application root component.

use gpui::{AppContext, Context, Entity, IntoElement, Render, Window};
use tusk_core::state::TuskState;
use tusk_core::{ConnectionConfig, ConnectionPool, SchemaService};
use tusk_ui::key_bindings::register_key_bindings;
use tusk_ui::{database_schema_to_tree, register_text_input_bindings, Workspace};

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

        // Create the workspace
        let workspace = cx.new(|cx| Workspace::new(window, cx));

        // Start loading schema from the database
        Self::load_schema(workspace.clone(), cx);

        Self { workspace }
    }

    /// Load database schema asynchronously and update the schema browser.
    fn load_schema(workspace: Entity<Workspace>, cx: &mut Context<Self>) {
        // Set loading state
        workspace.update(cx, |ws, cx| {
            ws.schema_browser().update(cx, |sb, cx| {
                sb.set_loading(true, cx);
            });
        });

        // Get handle to the Tokio runtime from TuskState
        let runtime_handle = cx.global::<TuskState>().runtime().handle().clone();

        // Spawn GPUI async task that will coordinate with Tokio runtime
        cx.spawn(async move |_this, cx| {
            // Run database operations inside the Tokio runtime
            let result = runtime_handle
                .spawn(async move {
                    // Create connection config for local PostgreSQL
                    let config = ConnectionConfig::new(
                        "Local PostgreSQL",
                        "localhost",
                        "postgres",
                        "brandon",
                    );

                    // Connect to database
                    let pool = ConnectionPool::new(config, "2212").await?;

                    tracing::info!("Connected to database, loading schema...");

                    // Get a connection and load schema
                    let conn = pool.get().await.map_err(|e| {
                        tusk_core::TuskError::connection(format!("Pool error: {}", e))
                    })?;

                    // Load schema
                    let schema = SchemaService::load_schema(&conn).await?;

                    tracing::info!(
                        schemas = schema.schemas.len(),
                        tables = schema.tables.len(),
                        views = schema.views.len(),
                        functions = schema.functions.len(),
                        "Schema loaded successfully"
                    );

                    // Convert to tree items
                    let tree_items = database_schema_to_tree(&schema);

                    Ok::<_, tusk_core::TuskError>(tree_items)
                })
                .await;

            // Handle the result and update UI
            match result {
                Ok(Ok(tree_items)) => {
                    let _ = cx.update(|cx| {
                        workspace.update(cx, |ws, cx| {
                            ws.schema_browser().update(cx, |sb, cx| {
                                sb.set_loading(false, cx);
                                sb.set_error(None, cx);
                                sb.set_schema(tree_items, cx);
                            });
                        });
                    });
                }
                Ok(Err(e)) => {
                    tracing::error!(error = %e, "Failed to load schema");
                    let _ = cx.update(|cx| {
                        workspace.update(cx, |ws, cx| {
                            ws.schema_browser().update(cx, |sb, cx| {
                                sb.set_loading(false, cx);
                                sb.set_error(Some(format!("{}", e).into()), cx);
                            });
                        });
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "Task panicked");
                    let _ = cx.update(|cx| {
                        workspace.update(cx, |ws, cx| {
                            ws.schema_browser().update(cx, |sb, cx| {
                                sb.set_loading(false, cx);
                                sb.set_error(Some("Internal error".into()), cx);
                            });
                        });
                    });
                }
            }
        })
        .detach();
    }
}

impl Render for TuskApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.workspace.clone()
    }
}
