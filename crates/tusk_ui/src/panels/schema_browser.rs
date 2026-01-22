//! Schema browser panel for navigating database structure.
//!
//! The schema browser lives in the left dock and provides a tree view of:
//! - Schemas
//! - Tables (with columns)
//! - Views (with columns)
//! - Functions

use gpui::{
    div, prelude::*, px, App, ClipboardItem, Context, Entity, EventEmitter, FocusHandle, Point,
    Render, SharedString, Subscription, Window,
};

use tusk_core::models::schema::DatabaseSchema;

use crate::context_menu::{ContextMenu, ContextMenuItem, ContextMenuLayer};
use crate::icon::{Icon, IconName, IconSize};
use crate::layout::spacing;
use crate::panel::{DockPosition, Focusable, Panel, PanelEvent};
use crate::text_input::{TextInput, TextInputEvent};
use crate::tree::{Tree, TreeEvent, TreeItem};
use crate::TuskTheme;

/// Schema item types for the tree view.
#[derive(Clone, Debug)]
pub enum SchemaItem {
    /// A database schema (namespace).
    Schema {
        id: String,
        name: String,
        children: Vec<SchemaItem>,
    },
    /// Folder for tables within a schema.
    TablesFolder {
        id: String,
        children: Vec<SchemaItem>,
    },
    /// Folder for views within a schema.
    ViewsFolder {
        id: String,
        children: Vec<SchemaItem>,
    },
    /// Folder for functions within a schema.
    FunctionsFolder {
        id: String,
        children: Vec<SchemaItem>,
    },
    /// A table within a schema.
    Table {
        id: String,
        name: String,
        children: Vec<SchemaItem>,
    },
    /// A view within a schema.
    View {
        id: String,
        name: String,
        is_materialized: bool,
        children: Vec<SchemaItem>,
    },
    /// A function within a schema.
    Function {
        id: String,
        name: String,
        arguments: String,
        return_type: String,
    },
    /// A column within a table or view.
    Column {
        id: String,
        name: String,
        data_type: String,
        is_nullable: bool,
        is_primary_key: bool,
    },
}

impl TreeItem for SchemaItem {
    type Id = String;

    fn id(&self) -> String {
        match self {
            SchemaItem::Schema { id, .. } => id.clone(),
            SchemaItem::TablesFolder { id, .. } => id.clone(),
            SchemaItem::ViewsFolder { id, .. } => id.clone(),
            SchemaItem::FunctionsFolder { id, .. } => id.clone(),
            SchemaItem::Table { id, .. } => id.clone(),
            SchemaItem::View { id, .. } => id.clone(),
            SchemaItem::Function { id, .. } => id.clone(),
            SchemaItem::Column { id, .. } => id.clone(),
        }
    }

    fn label(&self) -> SharedString {
        match self {
            SchemaItem::Schema { name, .. } => name.clone().into(),
            SchemaItem::TablesFolder { children, .. } => {
                format!("Tables ({})", children.len()).into()
            }
            SchemaItem::ViewsFolder { children, .. } => {
                format!("Views ({})", children.len()).into()
            }
            SchemaItem::FunctionsFolder { children, .. } => {
                format!("Functions ({})", children.len()).into()
            }
            SchemaItem::Table { name, .. } => name.clone().into(),
            SchemaItem::View {
                name,
                is_materialized,
                ..
            } => {
                if *is_materialized {
                    format!("{} (materialized)", name).into()
                } else {
                    name.clone().into()
                }
            }
            SchemaItem::Function {
                name,
                arguments,
                return_type,
                ..
            } => {
                if arguments.is_empty() {
                    format!("{}() -> {}", name, return_type).into()
                } else {
                    format!("{}({}) -> {}", name, arguments, return_type).into()
                }
            }
            SchemaItem::Column {
                name,
                data_type,
                is_nullable,
                is_primary_key,
                ..
            } => {
                let mut label = format!("{}: {}", name, data_type);
                if *is_primary_key {
                    label.push_str(" PK");
                }
                if !is_nullable {
                    label.push_str(" NOT NULL");
                }
                label.into()
            }
        }
    }

    fn icon(&self) -> Option<IconName> {
        Some(match self {
            SchemaItem::Schema { .. } => IconName::Schema,
            SchemaItem::TablesFolder { .. } => IconName::Folder,
            SchemaItem::ViewsFolder { .. } => IconName::Folder,
            SchemaItem::FunctionsFolder { .. } => IconName::Folder,
            SchemaItem::Table { .. } => IconName::Table,
            SchemaItem::View { is_materialized, .. } => {
                if *is_materialized {
                    IconName::Table // Materialized views are more like tables
                } else {
                    IconName::View
                }
            }
            SchemaItem::Function { .. } => IconName::Function,
            SchemaItem::Column { is_primary_key, .. } => {
                if *is_primary_key {
                    IconName::Key
                } else {
                    IconName::Column
                }
            }
        })
    }

    fn children(&self) -> Option<&[Self]> {
        match self {
            SchemaItem::Schema { children, .. } => Some(children),
            SchemaItem::TablesFolder { children, .. } => Some(children),
            SchemaItem::ViewsFolder { children, .. } => Some(children),
            SchemaItem::FunctionsFolder { children, .. } => Some(children),
            SchemaItem::Table { children, .. } => Some(children),
            SchemaItem::View { children, .. } => Some(children),
            SchemaItem::Function { .. } => None,
            SchemaItem::Column { .. } => None,
        }
    }
}

/// Convert a DatabaseSchema into a hierarchical Vec<SchemaItem> for the tree view.
///
/// The hierarchy is:
/// - Schema
///   - Tables (folder)
///     - Table
///       - Column
///   - Views (folder)
///     - View
///       - Column
///   - Functions (folder)
///     - Function
pub fn database_schema_to_tree(schema: &DatabaseSchema) -> Vec<SchemaItem> {
    schema
        .schemas
        .iter()
        .map(|schema_info| {
            let schema_name = &schema_info.name;

            // Collect tables for this schema
            let tables: Vec<SchemaItem> = schema
                .tables
                .iter()
                .filter(|t| &t.schema == schema_name)
                .map(|table| {
                    // Get columns for this table
                    let columns: Vec<SchemaItem> = schema
                        .table_columns
                        .get(&(schema_name.clone(), table.name.clone()))
                        .map(|cols| {
                            cols.iter()
                                .map(|col| SchemaItem::Column {
                                    id: format!("{}.{}.{}", schema_name, table.name, col.name),
                                    name: col.name.clone(),
                                    data_type: col.data_type.clone(),
                                    is_nullable: col.is_nullable,
                                    is_primary_key: col.is_primary_key,
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    SchemaItem::Table {
                        id: format!("{}.{}", schema_name, table.name),
                        name: table.name.clone(),
                        children: columns,
                    }
                })
                .collect();

            // Collect views for this schema
            let views: Vec<SchemaItem> = schema
                .views
                .iter()
                .filter(|v| &v.schema == schema_name)
                .map(|view| {
                    // Get columns for this view
                    let columns: Vec<SchemaItem> = schema
                        .view_columns
                        .get(&(schema_name.clone(), view.name.clone()))
                        .map(|cols| {
                            cols.iter()
                                .map(|col| SchemaItem::Column {
                                    id: format!("{}.{}.{}", schema_name, view.name, col.name),
                                    name: col.name.clone(),
                                    data_type: col.data_type.clone(),
                                    is_nullable: col.is_nullable,
                                    is_primary_key: col.is_primary_key,
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    SchemaItem::View {
                        id: format!("{}.{}", schema_name, view.name),
                        name: view.name.clone(),
                        is_materialized: view.is_materialized,
                        children: columns,
                    }
                })
                .collect();

            // Collect functions for this schema
            let functions: Vec<SchemaItem> = schema
                .functions
                .iter()
                .filter(|f| &f.schema == schema_name)
                .map(|func| SchemaItem::Function {
                    id: format!("{}.{}({})", schema_name, func.name, func.arguments),
                    name: func.name.clone(),
                    arguments: func.arguments.clone(),
                    return_type: func.return_type.clone(),
                })
                .collect();

            // Build the schema item with folders
            let mut children = Vec::new();

            if !tables.is_empty() {
                children.push(SchemaItem::TablesFolder {
                    id: format!("{}.tables", schema_name),
                    children: tables,
                });
            }

            if !views.is_empty() {
                children.push(SchemaItem::ViewsFolder {
                    id: format!("{}.views", schema_name),
                    children: views,
                });
            }

            if !functions.is_empty() {
                children.push(SchemaItem::FunctionsFolder {
                    id: format!("{}.functions", schema_name),
                    children: functions,
                });
            }

            SchemaItem::Schema {
                id: schema_name.clone(),
                name: schema_name.clone(),
                children,
            }
        })
        .collect()
}

/// Schema browser panel for navigating database objects.
pub struct SchemaBrowserPanel {
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// The tree component for displaying schema items.
    tree: Option<Entity<Tree<SchemaItem>>>,
    /// Subscription to tree events.
    _tree_subscription: Option<Subscription>,
    /// The filter input component.
    filter_input: Entity<TextInput>,
    /// Subscription to filter input events.
    _filter_subscription: Subscription,
    /// Whether the panel is currently loading schema data.
    is_loading: bool,
    /// Optional error message if schema loading failed.
    error: Option<SharedString>,
}

impl SchemaBrowserPanel {
    /// Create a new schema browser panel.
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Create the tree component with empty items - schema will be populated when connected
        let tree = cx.new(|cx| Tree::new(Vec::new(), cx));

        // Subscribe to tree events
        let tree_subscription = cx.subscribe(&tree, Self::handle_tree_event);

        // Create the filter input
        let filter_input = cx.new(|cx| TextInput::new("Filter schema...", cx));

        // Subscribe to filter input events
        let filter_subscription = cx.subscribe(&filter_input, Self::handle_filter_event);

        Self {
            focus_handle: cx.focus_handle(),
            tree: Some(tree),
            _tree_subscription: Some(tree_subscription),
            filter_input,
            _filter_subscription: filter_subscription,
            is_loading: false,
            error: None,
        }
    }

    /// Handle events from the filter input.
    fn handle_filter_event(
        &mut self,
        _input: Entity<TextInput>,
        event: &TextInputEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            TextInputEvent::Changed(text) => {
                if let Some(tree) = &self.tree {
                    tree.update(cx, |tree, cx| {
                        tree.set_filter(text.clone(), cx);
                    });
                }
            }
            TextInputEvent::Submitted(_) => {
                // Could focus the tree on submit
            }
            TextInputEvent::Focus | TextInputEvent::Blur => {
                // Focus/blur events - no action needed for filter input
            }
        }
    }

    /// Handle events from the tree component.
    fn handle_tree_event(
        &mut self,
        tree: Entity<Tree<SchemaItem>>,
        event: &TreeEvent<String>,
        cx: &mut Context<Self>,
    ) {
        match event {
            TreeEvent::Selected { id: _ } => {
                // Item selected - could update details panel
            }
            TreeEvent::Activated { id: _ } => {
                // Item activated (double-click or Enter)
                // Future: Open table data, show view definition, etc.
            }
            TreeEvent::Expanded { id: _ } => {
                // Item expanded
            }
            TreeEvent::Collapsed { id: _ } => {
                // Item collapsed
            }
            TreeEvent::ContextMenu { id, position } => {
                // Find the item by ID and show appropriate context menu
                self.show_context_menu(tree.clone(), id.clone(), *position, cx);
            }
        }
    }

    /// Show a context menu for the given schema item.
    fn show_context_menu(
        &mut self,
        tree: Entity<Tree<SchemaItem>>,
        id: String,
        position: Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        // Find the item in the tree's visible entries
        let item = tree.read(cx).visible_entries().iter().find_map(|entry| {
            if entry.item.id() == id {
                Some(entry.item.clone())
            } else {
                None
            }
        });

        let Some(item) = item else {
            return;
        };

        // Create menu items based on the item type
        let menu_items = self.create_menu_items_for_item(&item, &id);

        if menu_items.is_empty() {
            return;
        }

        // Create and show the context menu
        let menu = cx.new(|cx| ContextMenu::new(position, cx).items(menu_items));

        cx.update_global::<ContextMenuLayer, _>(|layer, cx| {
            layer.show_deferred(menu, cx);
        });
    }

    /// Create context menu items based on the schema item type.
    fn create_menu_items_for_item(&self, item: &SchemaItem, id: &str) -> Vec<ContextMenuItem> {
        match item {
            SchemaItem::Table { name, .. } => {
                let table_name = name.clone();
                let table_id = id.to_string();
                let copy_name = name.clone();

                vec![
                    ContextMenuItem::action("Select Top 100", move |_cx| {
                        // Future: Execute SELECT * FROM table LIMIT 100
                        tracing::info!(table = %table_name, "Select Top 100 requested");
                    })
                    .icon(IconName::Play)
                    .shortcut("Cmd+Return"),
                    ContextMenuItem::separator(),
                    ContextMenuItem::action("View DDL", move |_cx| {
                        // Future: Show CREATE TABLE statement
                        tracing::info!(table = %table_id, "View DDL requested");
                    })
                    .icon(IconName::File),
                    ContextMenuItem::separator(),
                    ContextMenuItem::action("Copy Name", move |cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(copy_name.clone()));
                        tracing::info!(name = %copy_name, "Copied table name to clipboard");
                    })
                    .icon(IconName::Copy)
                    .shortcut("Cmd+C"),
                ]
            }
            SchemaItem::View {
                name,
                is_materialized,
                ..
            } => {
                let view_name = name.clone();
                let view_id = id.to_string();
                let copy_name = name.clone();
                let is_mat = *is_materialized;

                vec![
                    ContextMenuItem::action("Select Top 100", move |_cx| {
                        tracing::info!(view = %view_name, "Select Top 100 requested");
                    })
                    .icon(IconName::Play)
                    .shortcut("Cmd+Return"),
                    ContextMenuItem::separator(),
                    ContextMenuItem::action("View DDL", move |_cx| {
                        tracing::info!(view = %view_id, "View DDL requested");
                    })
                    .icon(IconName::File),
                    ContextMenuItem::action(
                        if is_mat {
                            "Refresh Materialized View"
                        } else {
                            "View Definition"
                        },
                        move |_cx| {
                            if is_mat {
                                tracing::info!("Refresh materialized view requested");
                            } else {
                                tracing::info!("View definition requested");
                            }
                        },
                    )
                    .icon(IconName::Refresh),
                    ContextMenuItem::separator(),
                    ContextMenuItem::action("Copy Name", move |cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(copy_name.clone()));
                        tracing::info!(name = %copy_name, "Copied view name to clipboard");
                    })
                    .icon(IconName::Copy)
                    .shortcut("Cmd+C"),
                ]
            }
            SchemaItem::Function {
                name,
                arguments,
                return_type,
                ..
            } => {
                let func_name = name.clone();
                let func_id = id.to_string();
                let func_sig = format!("{}({})", name, arguments);
                let _return_type = return_type.clone();

                vec![
                    ContextMenuItem::action("View DDL", move |_cx| {
                        tracing::info!(function = %func_id, "View DDL requested");
                    })
                    .icon(IconName::File),
                    ContextMenuItem::separator(),
                    ContextMenuItem::action("Copy Name", move |cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(func_name.clone()));
                        tracing::info!(name = %func_name, "Copied function name to clipboard");
                    })
                    .icon(IconName::Copy)
                    .shortcut("Cmd+C"),
                    ContextMenuItem::action("Copy Signature", move |cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(func_sig.clone()));
                        tracing::info!(signature = %func_sig, "Copied function signature to clipboard");
                    })
                    .icon(IconName::Copy),
                ]
            }
            SchemaItem::Column {
                name, data_type, ..
            } => {
                let col_name = name.clone();
                let col_type = data_type.clone();

                vec![
                    ContextMenuItem::action("Copy Name", move |cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(col_name.clone()));
                        tracing::info!(name = %col_name, "Copied column name to clipboard");
                    })
                    .icon(IconName::Copy)
                    .shortcut("Cmd+C"),
                    ContextMenuItem::action("Copy Type", move |cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(col_type.clone()));
                        tracing::info!(data_type = %col_type, "Copied column type to clipboard");
                    })
                    .icon(IconName::Copy),
                ]
            }
            SchemaItem::Schema { name, .. } => {
                let schema_name = name.clone();

                vec![ContextMenuItem::action("Copy Name", move |cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(schema_name.clone()));
                    tracing::info!(name = %schema_name, "Copied schema name to clipboard");
                })
                .icon(IconName::Copy)
                .shortcut("Cmd+C")]
            }
            // Folder items don't have context menu actions
            SchemaItem::TablesFolder { .. }
            | SchemaItem::ViewsFolder { .. }
            | SchemaItem::FunctionsFolder { .. } => {
                vec![]
            }
        }
    }

    /// Set the loading state.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.is_loading = loading;
        cx.notify();
    }

    /// Set an error message.
    pub fn set_error(&mut self, error: Option<SharedString>, cx: &mut Context<Self>) {
        self.error = error;
        cx.notify();
    }

    /// Set the schema items to display.
    pub fn set_schema(&mut self, items: Vec<SchemaItem>, cx: &mut Context<Self>) {
        if let Some(tree) = &self.tree {
            tree.update(cx, |tree, cx| {
                tree.set_items(items, cx);
            });
        }
        cx.notify();
    }

    /// Set filter text for the tree.
    pub fn set_filter(&mut self, filter: String, cx: &mut Context<Self>) {
        self.filter_input.update(cx, |input, cx| {
            input.set_text(filter.clone(), cx);
        });
        if let Some(tree) = &self.tree {
            tree.update(cx, |tree, cx| {
                tree.set_filter(filter, cx);
            });
        }
        cx.notify();
    }

    /// Render the filter input.
    fn render_filter_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        div()
            .h(px(32.0))
            .w_full()
            .flex()
            .items_center()
            .gap(spacing::SM)
            .px(spacing::SM)
            .border_b_1()
            .border_color(theme.colors.border)
            .child(
                Icon::new(IconName::Search)
                    .size(IconSize::Small)
                    .color(theme.colors.text_muted),
            )
            .child(div().flex_1().child(self.filter_input.clone()))
    }

    /// Render the empty state when not connected.
    fn render_empty_state(&self, theme: &TuskTheme) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(48.0))
                    .rounded(px(8.0))
                    .bg(theme.colors.element_background)
                    .child(
                        Icon::new(IconName::Database)
                            .size(IconSize::XLarge)
                            .color(theme.colors.text_muted),
                    ),
            )
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(13.0))
                    .child("No connection"),
            )
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(12.0))
                    .child("Connect to a database to browse schema"),
            )
    }

    /// Render the loading state.
    fn render_loading_state(&self, theme: &TuskTheme) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(12.0))
            .child(crate::spinner::Spinner::new().size(crate::spinner::SpinnerSize::Large))
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(13.0))
                    .child("Loading schema..."),
            )
    }

    /// Render an error state.
    fn render_error_state(&self, error: &str, theme: &TuskTheme) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(12.0))
            .p(px(16.0))
            .child(
                Icon::new(IconName::Warning)
                    .size(IconSize::XLarge)
                    .color(theme.colors.error),
            )
            .child(
                div()
                    .text_color(theme.colors.error)
                    .text_size(px(13.0))
                    .text_center()
                    .child(error.to_string()),
            )
    }
}

impl EventEmitter<PanelEvent> for SchemaBrowserPanel {}

impl Focusable for SchemaBrowserPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for SchemaBrowserPanel {
    fn panel_id(&self) -> &'static str {
        "schema_browser"
    }

    fn title(&self, _cx: &App) -> SharedString {
        "Schema".into()
    }

    fn icon(&self, _cx: &App) -> IconName {
        IconName::Database
    }

    fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
    }

    fn closable(&self, _cx: &App) -> bool {
        false // Schema browser is always visible
    }

    fn position(&self, _cx: &App) -> DockPosition {
        DockPosition::Left
    }
}

impl Render for SchemaBrowserPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        let content = if self.is_loading {
            self.render_loading_state(theme).into_any_element()
        } else if let Some(error) = &self.error {
            self.render_error_state(error, theme).into_any_element()
        } else if let Some(tree) = &self.tree {
            // Show empty state when tree has no items
            if tree.read(cx).items().is_empty() {
                self.render_empty_state(theme).into_any_element()
            } else {
                tree.clone().into_any_element()
            }
        } else {
            self.render_empty_state(theme).into_any_element()
        };

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.panel_background)
            .child(
                // Panel header
                div()
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(Icon::new(IconName::Database).size(IconSize::Small))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text)
                                    .child("Schema Browser"),
                            ),
                    ),
            )
            // Filter input (only show when there's data)
            .when(
                self.tree
                    .as_ref()
                    .map(|t| !t.read(cx).items().is_empty())
                    .unwrap_or(false),
                |d| d.child(self.render_filter_input(cx)),
            )
            .child(
                // Panel content
                div().flex_1().overflow_hidden().child(content),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_item_id() {
        let item = SchemaItem::Table {
            id: "test-table".to_string(),
            name: "users".to_string(),
            children: vec![],
        };
        assert_eq!(item.id(), "test-table");
    }

    #[test]
    fn test_schema_item_label() {
        let item = SchemaItem::Table {
            id: "test-table".to_string(),
            name: "users".to_string(),
            children: vec![],
        };
        assert_eq!(item.label().as_ref(), "users");
    }

    #[test]
    fn test_schema_item_icon() {
        let table = SchemaItem::Table {
            id: "t".to_string(),
            name: "users".to_string(),
            children: vec![],
        };
        assert_eq!(table.icon(), Some(IconName::Table));

        let view = SchemaItem::View {
            id: "v".to_string(),
            name: "active_users".to_string(),
            is_materialized: false,
            children: vec![],
        };
        assert_eq!(view.icon(), Some(IconName::View));
    }

    #[test]
    fn test_schema_item_expandable() {
        let table = SchemaItem::Table {
            id: "t".to_string(),
            name: "users".to_string(),
            children: vec![],
        };
        assert!(table.is_expandable()); // Tables can have children (columns)

        let column = SchemaItem::Column {
            id: "c".to_string(),
            name: "id".to_string(),
            data_type: "integer".to_string(),
            is_nullable: false,
            is_primary_key: true,
        };
        assert!(!column.is_expandable()); // Columns are leaves
    }

    #[test]
    fn test_column_label_formatting() {
        let pk_column = SchemaItem::Column {
            id: "c1".to_string(),
            name: "id".to_string(),
            data_type: "bigint".to_string(),
            is_nullable: false,
            is_primary_key: true,
        };
        assert_eq!(pk_column.label().as_ref(), "id: bigint PK NOT NULL");

        let nullable_column = SchemaItem::Column {
            id: "c2".to_string(),
            name: "email".to_string(),
            data_type: "varchar(255)".to_string(),
            is_nullable: true,
            is_primary_key: false,
        };
        assert_eq!(nullable_column.label().as_ref(), "email: varchar(255)");
    }

    #[test]
    fn test_function_label_formatting() {
        let func_no_args = SchemaItem::Function {
            id: "f1".to_string(),
            name: "now".to_string(),
            arguments: "".to_string(),
            return_type: "timestamp".to_string(),
        };
        assert_eq!(func_no_args.label().as_ref(), "now() -> timestamp");

        let func_with_args = SchemaItem::Function {
            id: "f2".to_string(),
            name: "get_user".to_string(),
            arguments: "id bigint".to_string(),
            return_type: "users".to_string(),
        };
        assert_eq!(
            func_with_args.label().as_ref(),
            "get_user(id bigint) -> users"
        );
    }
}
