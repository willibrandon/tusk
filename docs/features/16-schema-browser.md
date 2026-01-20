# Feature 16: Schema Browser

## Overview

The schema browser provides a hierarchical tree view of all database objects, enabling users to navigate schemas, tables, views, functions, and other objects. Built entirely in Rust with GPUI, it includes context menus for common operations, DDL generation, and a global object search (command palette).

## Goals

- Display database objects in a navigable tree structure
- Support lazy loading for large schemas
- Provide context menus with object-specific actions
- Generate DDL for any object (CREATE, DROP, ALTER)
- Enable fuzzy search across all objects (command palette)
- Show object details on selection

## Dependencies

- Feature 10: Schema Introspection (schema metadata)
- Feature 07: Connection Management (connection state)
- Feature 13: Tabs Management (opening objects in tabs)
- Feature 03: Frontend Architecture (GPUI components)

## Technical Specification

### 16.1 Schema Tree Models

```rust
// src/schema_browser/models.rs

use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Node types in the schema tree
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Connection,
    SchemasFolder,
    Schema,
    TablesFolder,
    Table,
    ViewsFolder,
    View,
    MaterializedView,
    FunctionsFolder,
    Function,
    SequencesFolder,
    Sequence,
    TypesFolder,
    Type,
    ColumnsFolder,
    Column,
    IndexesFolder,
    Index,
    ForeignKeysFolder,
    ForeignKey,
    TriggersFolder,
    Trigger,
    PoliciesFolder,
    Policy,
    ExtensionsFolder,
    Extension,
    RolesFolder,
    Role,
    TablespacesFolder,
    Tablespace,
}

impl NodeType {
    pub fn icon_name(&self) -> &'static str {
        match self {
            NodeType::Connection => "database",
            NodeType::SchemasFolder => "folder-tree",
            NodeType::Schema => "box",
            NodeType::TablesFolder => "table",
            NodeType::Table => "table",
            NodeType::ViewsFolder => "eye",
            NodeType::View | NodeType::MaterializedView => "eye",
            NodeType::FunctionsFolder | NodeType::Function => "zap",
            NodeType::SequencesFolder | NodeType::Sequence => "hash",
            NodeType::TypesFolder | NodeType::Type => "box",
            NodeType::ColumnsFolder | NodeType::Column => "columns",
            NodeType::IndexesFolder | NodeType::Index => "key",
            NodeType::ForeignKeysFolder | NodeType::ForeignKey => "git-branch",
            NodeType::TriggersFolder | NodeType::Trigger => "zap",
            NodeType::PoliciesFolder | NodeType::Policy => "lock",
            NodeType::ExtensionsFolder | NodeType::Extension => "puzzle",
            NodeType::RolesFolder | NodeType::Role => "users",
            NodeType::TablespacesFolder | NodeType::Tablespace => "hard-drive",
        }
    }

    pub fn is_openable(&self) -> bool {
        matches!(self,
            NodeType::Table |
            NodeType::View |
            NodeType::MaterializedView |
            NodeType::Function
        )
    }

    pub fn is_folder(&self) -> bool {
        matches!(self,
            NodeType::SchemasFolder |
            NodeType::TablesFolder |
            NodeType::ViewsFolder |
            NodeType::FunctionsFolder |
            NodeType::SequencesFolder |
            NodeType::TypesFolder |
            NodeType::ColumnsFolder |
            NodeType::IndexesFolder |
            NodeType::ForeignKeysFolder |
            NodeType::TriggersFolder |
            NodeType::PoliciesFolder |
            NodeType::ExtensionsFolder |
            NodeType::RolesFolder |
            NodeType::TablespacesFolder
        )
    }
}

/// A node in the schema tree
#[derive(Clone, Debug)]
pub struct SchemaTreeNode {
    pub id: String,
    pub name: String,
    pub label: Option<String>,
    pub node_type: NodeType,
    pub schema: Option<String>,
    pub tooltip: Option<String>,
    pub badge: Option<String>,
    pub extra: Option<String>,
    pub children: Vec<SchemaTreeNode>,
    pub has_lazy_children: bool,
    pub data: Option<NodeData>,
}

impl SchemaTreeNode {
    pub fn new(id: impl Into<String>, name: impl Into<String>, node_type: NodeType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            label: None,
            node_type,
            schema: None,
            tooltip: None,
            badge: None,
            extra: None,
            children: Vec::new(),
            has_lazy_children: false,
            data: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    pub fn with_badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    pub fn with_extra(mut self, extra: impl Into<String>) -> Self {
        self.extra = Some(extra.into());
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn with_children(mut self, children: Vec<SchemaTreeNode>) -> Self {
        self.children = children;
        self
    }

    pub fn with_data(mut self, data: NodeData) -> Self {
        self.data = Some(data);
        self
    }

    pub fn display_name(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.name)
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty() || self.has_lazy_children
    }
}

/// Additional data stored with certain node types
#[derive(Clone, Debug)]
pub enum NodeData {
    Table(crate::schema::Table),
    Column(crate::schema::Column),
    Index(crate::schema::Index),
    ForeignKey(crate::schema::ForeignKey),
    View(crate::schema::View),
    Function(crate::schema::Function),
    Sequence(crate::schema::Sequence),
    Trigger(crate::schema::Trigger),
    Policy(crate::schema::Policy),
}

/// Context menu item
#[derive(Clone, Debug)]
pub enum ContextMenuItem {
    Action {
        label: String,
        action: ContextMenuAction,
        shortcut: Option<String>,
        danger: bool,
    },
    Separator,
}

/// Actions that can be performed from context menu
#[derive(Clone, Debug)]
pub enum ContextMenuAction {
    ViewData,
    NewQuery,
    EditObject,
    CreateSimilar,
    CopyName,
    CopyQualifiedName,
    ViewDdl,
    Truncate,
    Drop,
    Refresh,
    RefreshMaterializedView,
    ExecuteFunction,
    Reindex,
    AddToQuery,
    FilterByColumn,
    CreateTable,
    CreateView,
    CreateFunction,
}
```

### 16.2 Schema Browser State

```rust
// src/schema_browser/state.rs

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use gpui::Global;
use parking_lot::RwLock;
use tokio::runtime::Handle;

use crate::schema::{Schema, SchemaService};
use crate::error::Result;
use super::models::*;
use super::tree_builder::TreeBuilder;

/// Global schema browser state
pub struct SchemaBrowserState {
    schema_service: Arc<SchemaService>,
    runtime: Handle,

    /// Schema data per connection
    schemas: RwLock<HashMap<Uuid, Vec<Schema>>>,

    /// Built tree per connection
    trees: RwLock<HashMap<Uuid, SchemaTreeNode>>,

    /// Expanded node IDs per connection
    expanded_nodes: RwLock<HashMap<Uuid, HashSet<String>>>,

    /// Currently selected node ID per connection
    selected_node: RwLock<HashMap<Uuid, Option<String>>>,

    /// Loading state per connection
    loading: RwLock<HashSet<Uuid>>,

    /// Search query
    search_query: RwLock<String>,
}

impl Global for SchemaBrowserState {}

impl SchemaBrowserState {
    pub fn new(schema_service: Arc<SchemaService>, runtime: Handle) -> Self {
        Self {
            schema_service,
            runtime,
            schemas: RwLock::new(HashMap::new()),
            trees: RwLock::new(HashMap::new()),
            expanded_nodes: RwLock::new(HashMap::new()),
            selected_node: RwLock::new(HashMap::new()),
            loading: RwLock::new(HashSet::new()),
            search_query: RwLock::new(String::new()),
        }
    }

    /// Check if schema is loaded for connection
    pub fn has_schema(&self, connection_id: Uuid) -> bool {
        self.schemas.read().contains_key(&connection_id)
    }

    /// Check if currently loading
    pub fn is_loading(&self, connection_id: Uuid) -> bool {
        self.loading.read().contains(&connection_id)
    }

    /// Get the tree for a connection
    pub fn get_tree(&self, connection_id: Uuid) -> Option<SchemaTreeNode> {
        self.trees.read().get(&connection_id).cloned()
    }

    /// Get expanded nodes for a connection
    pub fn get_expanded_nodes(&self, connection_id: Uuid) -> HashSet<String> {
        self.expanded_nodes.read()
            .get(&connection_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get selected node for a connection
    pub fn get_selected_node(&self, connection_id: Uuid) -> Option<String> {
        self.selected_node.read()
            .get(&connection_id)
            .cloned()
            .flatten()
    }

    /// Load schema for a connection
    pub fn load_schema(&self, connection_id: Uuid) -> Result<()> {
        // Mark as loading
        self.loading.write().insert(connection_id);

        // Load schema asynchronously via blocking
        let service = self.schema_service.clone();
        let result = self.runtime.block_on(async {
            service.get_full_schema(connection_id).await
        });

        // Clear loading state
        self.loading.write().remove(&connection_id);

        match result {
            Ok(schemas) => {
                // Store schemas
                self.schemas.write().insert(connection_id, schemas.clone());

                // Build tree
                let tree = TreeBuilder::build_tree(connection_id, &schemas);
                self.trees.write().insert(connection_id, tree);

                // Initialize expanded nodes with schemas folder
                let mut expanded = HashSet::new();
                expanded.insert(format!("{}:schemas", connection_id));
                self.expanded_nodes.write().insert(connection_id, expanded);

                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Refresh schema for a connection
    pub fn refresh_schema(&self, connection_id: Uuid) -> Result<()> {
        // Keep expanded state
        let expanded = self.get_expanded_nodes(connection_id);

        // Reload
        self.load_schema(connection_id)?;

        // Restore expanded state
        self.expanded_nodes.write().insert(connection_id, expanded);

        Ok(())
    }

    /// Toggle node expansion
    pub fn toggle_expanded(&self, connection_id: Uuid, node_id: &str) {
        let mut expanded = self.expanded_nodes.write();
        let conn_expanded = expanded.entry(connection_id).or_default();

        if conn_expanded.contains(node_id) {
            conn_expanded.remove(node_id);
        } else {
            conn_expanded.insert(node_id.to_string());
        }
    }

    /// Set node expansion
    pub fn set_expanded(&self, connection_id: Uuid, node_id: &str, expanded: bool) {
        let mut exp = self.expanded_nodes.write();
        let conn_expanded = exp.entry(connection_id).or_default();

        if expanded {
            conn_expanded.insert(node_id.to_string());
        } else {
            conn_expanded.remove(node_id);
        }
    }

    /// Is node expanded?
    pub fn is_expanded(&self, connection_id: Uuid, node_id: &str) -> bool {
        self.expanded_nodes.read()
            .get(&connection_id)
            .map(|e| e.contains(node_id))
            .unwrap_or(false)
    }

    /// Select a node
    pub fn select_node(&self, connection_id: Uuid, node_id: Option<String>) {
        self.selected_node.write().insert(connection_id, node_id);
    }

    /// Find node by ID in tree
    pub fn find_node(&self, connection_id: Uuid, node_id: &str) -> Option<SchemaTreeNode> {
        let trees = self.trees.read();
        let tree = trees.get(&connection_id)?;
        Self::find_node_recursive(tree, node_id)
    }

    fn find_node_recursive(node: &SchemaTreeNode, target_id: &str) -> Option<SchemaTreeNode> {
        if node.id == target_id {
            return Some(node.clone());
        }

        for child in &node.children {
            if let Some(found) = Self::find_node_recursive(child, target_id) {
                return Some(found);
            }
        }

        None
    }

    /// Search objects across all schemas
    pub fn search_objects(&self, connection_id: Uuid, query: &str) -> Vec<SearchResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        let schemas = self.schemas.read();
        let Some(schema_list) = schemas.get(&connection_id) else {
            return Vec::new();
        };

        let term = query.to_lowercase();
        let mut results = Vec::new();

        for schema in schema_list {
            // Search tables
            for table in &schema.tables {
                if Self::fuzzy_match(&table.name, &term) {
                    results.push(SearchResult {
                        result_type: SearchResultType::Table,
                        schema: schema.name.clone(),
                        name: table.name.clone(),
                        parent_name: None,
                        score: Self::match_score(&table.name, &term),
                    });
                }

                // Search columns
                for col in &table.columns {
                    if Self::fuzzy_match(&col.name, &term) {
                        results.push(SearchResult {
                            result_type: SearchResultType::Column,
                            schema: schema.name.clone(),
                            name: col.name.clone(),
                            parent_name: Some(table.name.clone()),
                            score: Self::match_score(&col.name, &term),
                        });
                    }
                }
            }

            // Search views
            for view in &schema.views {
                if Self::fuzzy_match(&view.name, &term) {
                    results.push(SearchResult {
                        result_type: SearchResultType::View,
                        schema: schema.name.clone(),
                        name: view.name.clone(),
                        parent_name: None,
                        score: Self::match_score(&view.name, &term),
                    });
                }
            }

            // Search materialized views
            for mv in &schema.materialized_views {
                if Self::fuzzy_match(&mv.name, &term) {
                    results.push(SearchResult {
                        result_type: SearchResultType::MaterializedView,
                        schema: schema.name.clone(),
                        name: mv.name.clone(),
                        parent_name: None,
                        score: Self::match_score(&mv.name, &term),
                    });
                }
            }

            // Search functions
            for func in &schema.functions {
                if Self::fuzzy_match(&func.name, &term) {
                    results.push(SearchResult {
                        result_type: SearchResultType::Function,
                        schema: schema.name.clone(),
                        name: func.name.clone(),
                        parent_name: None,
                        score: Self::match_score(&func.name, &term),
                    });
                }
            }
        }

        // Sort by score
        results.sort_by(|a, b| b.score.cmp(&a.score));

        // Limit results
        results.truncate(50);

        results
    }

    fn fuzzy_match(text: &str, pattern: &str) -> bool {
        let text_lower = text.to_lowercase();
        let mut pattern_idx = 0;
        let pattern_chars: Vec<char> = pattern.chars().collect();

        for c in text_lower.chars() {
            if pattern_idx < pattern_chars.len() && c == pattern_chars[pattern_idx] {
                pattern_idx += 1;
            }
        }

        pattern_idx == pattern_chars.len()
    }

    fn match_score(text: &str, pattern: &str) -> u32 {
        let text_lower = text.to_lowercase();

        // Exact match
        if text_lower == pattern {
            return 100;
        }

        // Prefix match
        if text_lower.starts_with(pattern) {
            return 90;
        }

        // Contains match
        if text_lower.contains(pattern) {
            return 80;
        }

        // Fuzzy match - count consecutive matches
        let mut score = 0u32;
        let mut consecutive = 0u32;
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let mut pattern_idx = 0;

        for c in text_lower.chars() {
            if pattern_idx < pattern_chars.len() && c == pattern_chars[pattern_idx] {
                consecutive += 1;
                score += consecutive;
                pattern_idx += 1;
            } else {
                consecutive = 0;
            }
        }

        score
    }
}

/// Search result
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub result_type: SearchResultType,
    pub schema: String,
    pub name: String,
    pub parent_name: Option<String>,
    pub score: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchResultType {
    Table,
    View,
    MaterializedView,
    Function,
    Column,
    Sequence,
    Type,
}

impl SearchResultType {
    pub fn label(&self) -> &'static str {
        match self {
            SearchResultType::Table => "table",
            SearchResultType::View => "view",
            SearchResultType::MaterializedView => "matview",
            SearchResultType::Function => "function",
            SearchResultType::Column => "column",
            SearchResultType::Sequence => "sequence",
            SearchResultType::Type => "type",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            SearchResultType::Table => "table",
            SearchResultType::View | SearchResultType::MaterializedView => "eye",
            SearchResultType::Function => "zap",
            SearchResultType::Column => "columns",
            SearchResultType::Sequence => "hash",
            SearchResultType::Type => "box",
        }
    }
}
```

### 16.3 Tree Builder

```rust
// src/schema_browser/tree_builder.rs

use uuid::Uuid;
use crate::schema::{Schema, Table, View, Function, Column, Index, ForeignKey};
use super::models::*;

pub struct TreeBuilder;

impl TreeBuilder {
    /// Build the complete schema tree for a connection
    pub fn build_tree(connection_id: Uuid, schemas: &[Schema]) -> SchemaTreeNode {
        let schema_nodes: Vec<SchemaTreeNode> = schemas.iter()
            .map(|schema| Self::build_schema_node(connection_id, schema))
            .collect();

        SchemaTreeNode::new(
            format!("{}:root", connection_id),
            "Database",
            NodeType::Connection,
        )
        .with_children(vec![
            SchemaTreeNode::new(
                format!("{}:schemas", connection_id),
                "Schemas",
                NodeType::SchemasFolder,
            )
            .with_badge(schemas.len().to_string())
            .with_children(schema_nodes),
        ])
    }

    fn build_schema_node(connection_id: Uuid, schema: &Schema) -> SchemaTreeNode {
        let children = vec![
            Self::build_tables_folder(connection_id, schema),
            Self::build_views_folder(connection_id, schema),
            Self::build_functions_folder(connection_id, schema),
            Self::build_sequences_folder(connection_id, schema),
            Self::build_types_folder(connection_id, schema),
        ]
        .into_iter()
        .filter(|n| n.badge.as_ref().map(|b| b != "0").unwrap_or(true))
        .collect();

        SchemaTreeNode::new(
            format!("{}:schema:{}", connection_id, schema.name),
            &schema.name,
            NodeType::Schema,
        )
        .with_schema(&schema.name)
        .with_children(children)
    }

    fn build_tables_folder(connection_id: Uuid, schema: &Schema) -> SchemaTreeNode {
        let children: Vec<SchemaTreeNode> = schema.tables.iter()
            .map(|table| Self::build_table_node(connection_id, &schema.name, table))
            .collect();

        SchemaTreeNode::new(
            format!("{}:{}:tables", connection_id, schema.name),
            "Tables",
            NodeType::TablesFolder,
        )
        .with_schema(&schema.name)
        .with_badge(schema.tables.len().to_string())
        .with_children(children)
    }

    fn build_table_node(connection_id: Uuid, schema_name: &str, table: &Table) -> SchemaTreeNode {
        let size_str = Self::format_size(table.size_bytes);
        let row_count = table.row_count_estimate
            .map(|r| format!("{} rows", Self::format_number(r)))
            .unwrap_or_else(|| "? rows".to_string());

        let tooltip = format!("{}, {}", row_count, size_str);

        let children = vec![
            // Columns folder
            SchemaTreeNode::new(
                format!("{}:{}:{}:columns", connection_id, schema_name, table.name),
                "Columns",
                NodeType::ColumnsFolder,
            )
            .with_badge(table.columns.len().to_string())
            .with_children(
                table.columns.iter()
                    .map(|col| Self::build_column_node(connection_id, schema_name, &table.name, col))
                    .collect()
            ),

            // Indexes folder
            SchemaTreeNode::new(
                format!("{}:{}:{}:indexes", connection_id, schema_name, table.name),
                "Indexes",
                NodeType::IndexesFolder,
            )
            .with_badge(table.indexes.len().to_string())
            .with_children(
                table.indexes.iter()
                    .map(|idx| Self::build_index_node(connection_id, schema_name, &table.name, idx))
                    .collect()
            ),

            // Foreign keys folder
            SchemaTreeNode::new(
                format!("{}:{}:{}:fks", connection_id, schema_name, table.name),
                "Foreign Keys",
                NodeType::ForeignKeysFolder,
            )
            .with_badge(table.foreign_keys.len().to_string())
            .with_children(
                table.foreign_keys.iter()
                    .map(|fk| Self::build_fk_node(connection_id, schema_name, &table.name, fk))
                    .collect()
            ),

            // Triggers folder
            SchemaTreeNode::new(
                format!("{}:{}:{}:triggers", connection_id, schema_name, table.name),
                "Triggers",
                NodeType::TriggersFolder,
            )
            .with_badge(table.triggers.as_ref().map(|t| t.len()).unwrap_or(0).to_string())
            .with_children(
                table.triggers.as_ref().map(|triggers| {
                    triggers.iter()
                        .map(|tr| SchemaTreeNode::new(
                            format!("{}:{}:{}:trigger:{}", connection_id, schema_name, table.name, tr.name),
                            &tr.name,
                            NodeType::Trigger,
                        ).with_schema(schema_name))
                        .collect()
                }).unwrap_or_default()
            ),

            // Policies folder
            SchemaTreeNode::new(
                format!("{}:{}:{}:policies", connection_id, schema_name, table.name),
                "Policies",
                NodeType::PoliciesFolder,
            )
            .with_badge(table.policies.as_ref().map(|p| p.len()).unwrap_or(0).to_string())
            .with_children(
                table.policies.as_ref().map(|policies| {
                    policies.iter()
                        .map(|pol| SchemaTreeNode::new(
                            format!("{}:{}:{}:policy:{}", connection_id, schema_name, table.name, pol.name),
                            &pol.name,
                            NodeType::Policy,
                        ).with_schema(schema_name))
                        .collect()
                }).unwrap_or_default()
            ),
        ]
        .into_iter()
        .filter(|n| n.badge.as_ref().map(|b| b != "0").unwrap_or(true))
        .collect();

        SchemaTreeNode::new(
            format!("{}:{}:table:{}", connection_id, schema_name, table.name),
            &table.name,
            NodeType::Table,
        )
        .with_schema(schema_name)
        .with_tooltip(tooltip)
        .with_extra(size_str)
        .with_data(NodeData::Table(table.clone()))
        .with_children(children)
    }

    fn build_column_node(
        connection_id: Uuid,
        schema_name: &str,
        table_name: &str,
        col: &Column,
    ) -> SchemaTreeNode {
        let label = if col.nullable {
            col.name.clone()
        } else {
            format!("{} *", col.name)
        };

        let tooltip = Self::build_column_tooltip(col);

        SchemaTreeNode::new(
            format!("{}:{}:{}:col:{}", connection_id, schema_name, table_name, col.name),
            &col.name,
            NodeType::Column,
        )
        .with_label(label)
        .with_schema(schema_name)
        .with_extra(&col.type_name)
        .with_tooltip(tooltip)
        .with_data(NodeData::Column(col.clone()))
    }

    fn build_column_tooltip(col: &Column) -> String {
        let mut parts = vec![col.type_name.clone()];

        if !col.nullable {
            parts.push("NOT NULL".to_string());
        }

        if let Some(default) = &col.default {
            parts.push(format!("DEFAULT {}", default));
        }

        if col.is_identity {
            let gen = col.identity_generation.as_deref().unwrap_or("BY DEFAULT");
            parts.push(format!("IDENTITY {}", gen));
        }

        if let Some(comment) = &col.comment {
            parts.push(format!("-- {}", comment));
        }

        parts.join(" ")
    }

    fn build_index_node(
        connection_id: Uuid,
        schema_name: &str,
        table_name: &str,
        index: &Index,
    ) -> SchemaTreeNode {
        let tooltip = format!(
            "{}{}on ({})",
            if index.is_unique { "UNIQUE " } else { "" },
            index.method.to_uppercase(),
            index.columns.join(", ")
        );

        SchemaTreeNode::new(
            format!("{}:{}:{}:idx:{}", connection_id, schema_name, table_name, index.name),
            &index.name,
            NodeType::Index,
        )
        .with_schema(schema_name)
        .with_extra(&index.method)
        .with_tooltip(tooltip)
        .with_data(NodeData::Index(index.clone()))
    }

    fn build_fk_node(
        connection_id: Uuid,
        schema_name: &str,
        table_name: &str,
        fk: &ForeignKey,
    ) -> SchemaTreeNode {
        let tooltip = format!(
            "({}) -> {}({})",
            fk.columns.join(", "),
            fk.referenced_table,
            fk.referenced_columns.join(", ")
        );

        SchemaTreeNode::new(
            format!("{}:{}:{}:fk:{}", connection_id, schema_name, table_name, fk.name),
            &fk.name,
            NodeType::ForeignKey,
        )
        .with_schema(schema_name)
        .with_tooltip(tooltip)
        .with_data(NodeData::ForeignKey(fk.clone()))
    }

    fn build_views_folder(connection_id: Uuid, schema: &Schema) -> SchemaTreeNode {
        let mut children: Vec<SchemaTreeNode> = schema.views.iter()
            .map(|view| SchemaTreeNode::new(
                format!("{}:{}:view:{}", connection_id, schema.name, view.name),
                &view.name,
                NodeType::View,
            )
            .with_schema(&schema.name)
            .with_data(NodeData::View(view.clone())))
            .collect();

        // Add materialized views
        for mv in &schema.materialized_views {
            children.push(
                SchemaTreeNode::new(
                    format!("{}:{}:matview:{}", connection_id, schema.name, mv.name),
                    &mv.name,
                    NodeType::MaterializedView,
                )
                .with_schema(&schema.name)
                .with_label(format!("{} (materialized)", mv.name))
            );
        }

        let total = schema.views.len() + schema.materialized_views.len();

        SchemaTreeNode::new(
            format!("{}:{}:views", connection_id, schema.name),
            "Views",
            NodeType::ViewsFolder,
        )
        .with_schema(&schema.name)
        .with_badge(total.to_string())
        .with_children(children)
    }

    fn build_functions_folder(connection_id: Uuid, schema: &Schema) -> SchemaTreeNode {
        let children: Vec<SchemaTreeNode> = schema.functions.iter()
            .map(|func| {
                let tooltip = format!("{}({}) -> {}", func.name, func.arguments, func.return_type);
                SchemaTreeNode::new(
                    format!("{}:{}:fn:{}:{}", connection_id, schema.name, func.name, func.oid),
                    &func.name,
                    NodeType::Function,
                )
                .with_schema(&schema.name)
                .with_extra(&func.return_type)
                .with_tooltip(tooltip)
                .with_data(NodeData::Function(func.clone()))
            })
            .collect();

        SchemaTreeNode::new(
            format!("{}:{}:functions", connection_id, schema.name),
            "Functions",
            NodeType::FunctionsFolder,
        )
        .with_schema(&schema.name)
        .with_badge(schema.functions.len().to_string())
        .with_children(children)
    }

    fn build_sequences_folder(connection_id: Uuid, schema: &Schema) -> SchemaTreeNode {
        let children: Vec<SchemaTreeNode> = schema.sequences.iter()
            .map(|seq| SchemaTreeNode::new(
                format!("{}:{}:seq:{}", connection_id, schema.name, seq.name),
                &seq.name,
                NodeType::Sequence,
            )
            .with_schema(&schema.name)
            .with_data(NodeData::Sequence(seq.clone())))
            .collect();

        SchemaTreeNode::new(
            format!("{}:{}:sequences", connection_id, schema.name),
            "Sequences",
            NodeType::SequencesFolder,
        )
        .with_schema(&schema.name)
        .with_badge(schema.sequences.len().to_string())
        .with_children(children)
    }

    fn build_types_folder(connection_id: Uuid, schema: &Schema) -> SchemaTreeNode {
        let children: Vec<SchemaTreeNode> = schema.types.iter()
            .map(|t| SchemaTreeNode::new(
                format!("{}:{}:type:{}", connection_id, schema.name, t.name),
                &t.name,
                NodeType::Type,
            )
            .with_schema(&schema.name))
            .collect();

        SchemaTreeNode::new(
            format!("{}:{}:types", connection_id, schema.name),
            "Types",
            NodeType::TypesFolder,
        )
        .with_schema(&schema.name)
        .with_badge(schema.types.len().to_string())
        .with_children(children)
    }

    fn format_size(bytes: u64) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    fn format_number(n: i64) -> String {
        if n < 1000 {
            n.to_string()
        } else if n < 1_000_000 {
            format!("{:.1}K", n as f64 / 1000.0)
        } else if n < 1_000_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else {
            format!("{:.1}B", n as f64 / 1_000_000_000.0)
        }
    }
}
```

### 16.4 Schema Tree GPUI Component

```rust
// src/ui/schema_tree.rs

use gpui::*;
use uuid::Uuid;
use std::sync::Arc;

use crate::schema_browser::state::SchemaBrowserState;
use crate::schema_browser::models::*;
use crate::tabs::state::TabState;
use crate::theme::Theme;

/// Schema tree component
pub struct SchemaTree {
    connection_id: Uuid,
    search_query: String,
    context_menu: Option<ContextMenuState>,
}

struct ContextMenuState {
    position: Point<Pixels>,
    node: SchemaTreeNode,
}

pub enum SchemaTreeEvent {
    NodeSelected(SchemaTreeNode),
    NodeActivated(SchemaTreeNode),
    Refresh,
}

impl EventEmitter<SchemaTreeEvent> for SchemaTree {}

impl SchemaTree {
    pub fn new(connection_id: Uuid) -> Self {
        Self {
            connection_id,
            search_query: String::new(),
            context_menu: None,
        }
    }

    fn load_if_needed(&self, cx: &mut Context<Self>) {
        let state = cx.global::<SchemaBrowserState>();
        if !state.has_schema(self.connection_id) && !state.is_loading(self.connection_id) {
            let _ = state.load_schema(self.connection_id);
            cx.notify();
        }
    }

    fn handle_node_click(&mut self, node: &SchemaTreeNode, cx: &mut Context<Self>) {
        let state = cx.global::<SchemaBrowserState>();
        state.select_node(self.connection_id, Some(node.id.clone()));

        if node.has_children() {
            state.toggle_expanded(self.connection_id, &node.id);
        }

        cx.emit(SchemaTreeEvent::NodeSelected(node.clone()));
        cx.notify();
    }

    fn handle_node_double_click(&mut self, node: &SchemaTreeNode, cx: &mut Context<Self>) {
        if node.node_type.is_openable() {
            cx.emit(SchemaTreeEvent::NodeActivated(node.clone()));
        }
    }

    fn handle_context_menu(&mut self, node: &SchemaTreeNode, position: Point<Pixels>, cx: &mut Context<Self>) {
        self.context_menu = Some(ContextMenuState {
            position,
            node: node.clone(),
        });
        cx.notify();
    }

    fn dismiss_context_menu(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        cx.notify();
    }

    fn get_context_menu_items(&self, node: &SchemaTreeNode) -> Vec<ContextMenuItem> {
        match node.node_type {
            NodeType::Table => vec![
                ContextMenuItem::Action {
                    label: "View Data".to_string(),
                    action: ContextMenuAction::ViewData,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "New Query".to_string(),
                    action: ContextMenuAction::NewQuery,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Separator,
                ContextMenuItem::Action {
                    label: "Edit Table".to_string(),
                    action: ContextMenuAction::EditObject,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "Create Similar".to_string(),
                    action: ContextMenuAction::CreateSimilar,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Separator,
                ContextMenuItem::Action {
                    label: "Copy Name".to_string(),
                    action: ContextMenuAction::CopyName,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "Copy Qualified Name".to_string(),
                    action: ContextMenuAction::CopyQualifiedName,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Separator,
                ContextMenuItem::Action {
                    label: "View DDL".to_string(),
                    action: ContextMenuAction::ViewDdl,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Separator,
                ContextMenuItem::Action {
                    label: "Truncate...".to_string(),
                    action: ContextMenuAction::Truncate,
                    shortcut: None,
                    danger: true,
                },
                ContextMenuItem::Action {
                    label: "Drop...".to_string(),
                    action: ContextMenuAction::Drop,
                    shortcut: None,
                    danger: true,
                },
                ContextMenuItem::Separator,
                ContextMenuItem::Action {
                    label: "Refresh".to_string(),
                    action: ContextMenuAction::Refresh,
                    shortcut: None,
                    danger: false,
                },
            ],
            NodeType::View | NodeType::MaterializedView => vec![
                ContextMenuItem::Action {
                    label: "View Data".to_string(),
                    action: ContextMenuAction::ViewData,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "New Query".to_string(),
                    action: ContextMenuAction::NewQuery,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Separator,
                ContextMenuItem::Action {
                    label: "Edit View".to_string(),
                    action: ContextMenuAction::EditObject,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "View DDL".to_string(),
                    action: ContextMenuAction::ViewDdl,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Separator,
                ContextMenuItem::Action {
                    label: "Copy Name".to_string(),
                    action: ContextMenuAction::CopyName,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Separator,
                ContextMenuItem::Action {
                    label: "Drop...".to_string(),
                    action: ContextMenuAction::Drop,
                    shortcut: None,
                    danger: true,
                },
            ],
            NodeType::Function => vec![
                ContextMenuItem::Action {
                    label: "Open".to_string(),
                    action: ContextMenuAction::ViewData,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "Execute...".to_string(),
                    action: ContextMenuAction::ExecuteFunction,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Separator,
                ContextMenuItem::Action {
                    label: "View DDL".to_string(),
                    action: ContextMenuAction::ViewDdl,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "Drop...".to_string(),
                    action: ContextMenuAction::Drop,
                    shortcut: None,
                    danger: true,
                },
            ],
            NodeType::Index => vec![
                ContextMenuItem::Action {
                    label: "View DDL".to_string(),
                    action: ContextMenuAction::ViewDdl,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "Reindex".to_string(),
                    action: ContextMenuAction::Reindex,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "Drop...".to_string(),
                    action: ContextMenuAction::Drop,
                    shortcut: None,
                    danger: true,
                },
            ],
            NodeType::Column => vec![
                ContextMenuItem::Action {
                    label: "Add to Query".to_string(),
                    action: ContextMenuAction::AddToQuery,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "Filter by Value...".to_string(),
                    action: ContextMenuAction::FilterByColumn,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "Copy Name".to_string(),
                    action: ContextMenuAction::CopyName,
                    shortcut: None,
                    danger: false,
                },
            ],
            NodeType::Schema => vec![
                ContextMenuItem::Action {
                    label: "New Table...".to_string(),
                    action: ContextMenuAction::CreateTable,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "New View...".to_string(),
                    action: ContextMenuAction::CreateView,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Action {
                    label: "New Function...".to_string(),
                    action: ContextMenuAction::CreateFunction,
                    shortcut: None,
                    danger: false,
                },
                ContextMenuItem::Separator,
                ContextMenuItem::Action {
                    label: "Drop Schema...".to_string(),
                    action: ContextMenuAction::Drop,
                    shortcut: None,
                    danger: true,
                },
            ],
            _ => vec![
                ContextMenuItem::Action {
                    label: "Refresh".to_string(),
                    action: ContextMenuAction::Refresh,
                    shortcut: None,
                    danger: false,
                },
            ],
        }
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .gap_1()
            .p_2()
            .border_b_1()
            .border_color(theme.border)
            // Search box
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .child(
                        svg()
                            .path("icons/search.svg")
                            .size_3p5()
                            .text_color(theme.text_muted)
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child("Search objects...")
                            // Would be replaced with actual TextInput
                    )
            )
            // Refresh button
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_7()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .cursor_pointer()
                    .hover(|div| div.bg(theme.hover))
                    .on_click(cx.listener(|this, _, cx| {
                        cx.emit(SchemaTreeEvent::Refresh);
                    }))
                    .child(
                        svg()
                            .path("icons/refresh-cw.svg")
                            .size_3p5()
                            .text_color(theme.text_muted)
                    )
            )
    }

    fn render_tree_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<SchemaBrowserState>();

        if state.is_loading(self.connection_id) {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_muted)
                .text_sm()
                .child("Loading schema...");
        }

        match state.get_tree(self.connection_id) {
            Some(tree) => {
                div()
                    .flex_1()
                    .overflow_y_auto()
                    .py_1()
                    .children(
                        tree.children.iter().map(|node| {
                            self.render_tree_node(node, 0, cx)
                        })
                    )
            }
            None => {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .text_sm()
                    .child("No schema loaded")
            }
        }
    }

    fn render_tree_node(&self, node: &SchemaTreeNode, depth: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<SchemaBrowserState>();

        let is_expanded = state.is_expanded(self.connection_id, &node.id);
        let is_selected = state.get_selected_node(self.connection_id)
            .as_ref()
            .map(|id| id == &node.id)
            .unwrap_or(false);
        let has_children = node.has_children();

        let node_clone = node.clone();
        let node_clone2 = node.clone();
        let node_id = node.id.clone();

        div()
            .flex()
            .flex_col()
            // Node content row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_0p5()
                    .mx_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .pl(px(depth as f32 * 16.0 + 4.0))
                    .when(is_selected, |div| div.bg(theme.selected))
                    .when(!is_selected, |div| div.hover(|d| d.bg(theme.hover)))
                    .on_click(cx.listener(move |this, _, cx| {
                        this.handle_node_click(&node_clone, cx);
                    }))
                    .on_double_click(cx.listener(move |this, _, cx| {
                        this.handle_node_double_click(&node_clone2, cx);
                    }))
                    // Expand icon
                    .child(
                        div()
                            .size_3p5()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(theme.text_muted)
                            .when(has_children, |div| {
                                if is_expanded {
                                    div.child(svg().path("icons/chevron-down.svg").size_3p5())
                                } else {
                                    div.child(svg().path("icons/chevron-right.svg").size_3p5())
                                }
                            })
                    )
                    // Node icon
                    .child(
                        div()
                            .flex()
                            .text_color(theme.text_muted)
                            .child(
                                svg()
                                    .path(format!("icons/{}.svg", node.node_type.icon_name()))
                                    .size_3p5()
                            )
                    )
                    // Node label
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .truncate()
                            .child(node.display_name().to_string())
                            .when_some(node.badge.as_ref(), |div, badge| {
                                div.child(
                                    span()
                                        .text_xs()
                                        .text_color(theme.text_muted)
                                        .ml_1()
                                        .child(format!("({})", badge))
                                )
                            })
                    )
                    // Extra info (type, size)
                    .when_some(node.extra.as_ref(), |div, extra| {
                        div.child(
                            div()
                                .text_xs()
                                .font_family("monospace")
                                .text_color(theme.text_muted)
                                .child(extra.clone())
                        )
                    })
            )
            // Children (if expanded)
            .when(is_expanded && has_children, |div| {
                div.children(
                    node.children.iter().map(|child| {
                        self.render_tree_node(child, depth + 1, cx)
                    })
                )
            })
    }

    fn render_context_menu(&self, state: &ContextMenuState, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let items = self.get_context_menu_items(&state.node);

        div()
            .absolute()
            .top(state.position.y)
            .left(state.position.x)
            .min_w_48()
            .bg(theme.surface_elevated)
            .rounded_md()
            .shadow_lg()
            .border_1()
            .border_color(theme.border)
            .py_1()
            .children(items.into_iter().map(|item| {
                match item {
                    ContextMenuItem::Action { label, action, shortcut, danger } => {
                        div()
                            .px_3()
                            .py_1p5()
                            .flex()
                            .items_center()
                            .justify_between()
                            .cursor_pointer()
                            .hover(|div| div.bg(theme.hover))
                            .when(danger, |div| div.text_color(theme.error))
                            .when(!danger, |div| div.text_color(theme.text))
                            .on_click(cx.listener(move |this, _, cx| {
                                this.dismiss_context_menu(cx);
                                // Handle action
                            }))
                            .child(
                                div()
                                    .text_sm()
                                    .child(label)
                            )
                            .when_some(shortcut, |div, shortcut| {
                                div.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.text_muted)
                                        .child(shortcut)
                                )
                            })
                            .into_any_element()
                    }
                    ContextMenuItem::Separator => {
                        div()
                            .my_1()
                            .h_px()
                            .bg(theme.border)
                            .into_any_element()
                    }
                }
            }))
    }
}

impl Render for SchemaTree {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        // Load schema if needed
        self.load_if_needed(cx);

        div()
            .flex()
            .flex_col()
            .h_full()
            .bg(theme.surface)
            .child(self.render_header(cx))
            .child(self.render_tree_content(cx))
            .when_some(self.context_menu.as_ref(), |div, state| {
                div.child(self.render_context_menu(state, cx))
            })
    }
}
```

### 16.5 Object Search Dialog (Command Palette)

```rust
// src/ui/object_search.rs

use gpui::*;
use uuid::Uuid;

use crate::schema_browser::state::{SchemaBrowserState, SearchResult, SearchResultType};
use crate::tabs::state::TabState;
use crate::theme::Theme;

/// Object search dialog (command palette)
pub struct ObjectSearch {
    connection_id: Uuid,
    query: String,
    selected_index: usize,
    results: Vec<SearchResult>,
}

pub enum ObjectSearchEvent {
    Selected(SearchResult),
    Dismissed,
}

impl EventEmitter<ObjectSearchEvent> for ObjectSearch {}

impl ObjectSearch {
    pub fn new(connection_id: Uuid) -> Self {
        Self {
            connection_id,
            query: String::new(),
            selected_index: 0,
            results: Vec::new(),
        }
    }

    fn update_search(&mut self, cx: &mut Context<Self>) {
        let state = cx.global::<SchemaBrowserState>();
        self.results = state.search_objects(self.connection_id, &self.query);
        self.selected_index = 0;
        cx.notify();
    }

    fn select_current(&mut self, cx: &mut Context<Self>) {
        if let Some(result) = self.results.get(self.selected_index) {
            cx.emit(ObjectSearchEvent::Selected(result.clone()));
        }
    }

    fn move_selection(&mut self, delta: i32, cx: &mut Context<Self>) {
        if self.results.is_empty() {
            return;
        }

        let len = self.results.len() as i32;
        let new_index = (self.selected_index as i32 + delta).rem_euclid(len);
        self.selected_index = new_index as usize;
        cx.notify();
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "down" => {
                self.move_selection(1, cx);
            }
            "up" => {
                self.move_selection(-1, cx);
            }
            "enter" => {
                self.select_current(cx);
            }
            "escape" => {
                cx.emit(ObjectSearchEvent::Dismissed);
            }
            _ => {}
        }
    }

    fn render_result(&self, result: &SearchResult, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let is_selected = index == self.selected_index;

        let result_clone = result.clone();

        div()
            .flex()
            .items_center()
            .gap_3()
            .w_full()
            .px_4()
            .py_2()
            .cursor_pointer()
            .when(is_selected, |div| div.bg(theme.hover))
            .on_click(cx.listener(move |this, _, cx| {
                cx.emit(ObjectSearchEvent::Selected(result_clone.clone()));
            }))
            .on_mouse_enter(cx.listener(move |this, _, cx| {
                this.selected_index = index;
                cx.notify();
            }))
            // Icon
            .child(
                svg()
                    .path(format!("icons/{}.svg", result.result_type.icon_name()))
                    .size_4()
                    .text_color(theme.text_muted)
            )
            // Name and parent
            .child(
                div()
                    .flex_1()
                    .font_weight(FontWeight::MEDIUM)
                    .child(result.name.clone())
                    .when_some(result.parent_name.as_ref(), |div, parent| {
                        div.child(
                            span()
                                .font_weight(FontWeight::NORMAL)
                                .text_color(theme.text_muted)
                                .text_sm()
                                .ml_2()
                                .child(format!("in {}", parent))
                        )
                    })
            )
            // Schema
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child(result.schema.clone())
            )
            // Type badge
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .px_1p5()
                    .py_0p5()
                    .rounded_sm()
                    .bg(theme.surface_variant)
                    .uppercase()
                    .child(result.result_type.label())
            )
    }
}

impl Render for ObjectSearch {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        // Overlay
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000080))
            .flex()
            .justify_center()
            .pt_24()
            .on_click(cx.listener(|_, _, cx| {
                cx.emit(ObjectSearchEvent::Dismissed);
            }))
            .child(
                // Dialog
                div()
                    .w_150()
                    .max_h_125()
                    .bg(theme.surface)
                    .rounded_lg()
                    .shadow_xl()
                    .overflow_hidden()
                    .on_click(|_, _| {}) // Prevent click-through
                    .on_key_down(cx.listener(|this, event, cx| {
                        this.handle_key_down(event, cx);
                    }))
                    // Search input
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(theme.border)
                            .child(
                                svg()
                                    .path("icons/search.svg")
                                    .size_5()
                                    .text_color(theme.text_muted)
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_lg()
                                    .child(&self.query)
                                    // In real implementation: TextInput component
                            )
                    )
                    // Results
                    .child(
                        div()
                            .max_h_100()
                            .overflow_y_auto()
                            .when(self.results.is_empty() && !self.query.is_empty(), |div| {
                                div.child(
                                    div()
                                        .p_8()
                                        .text_center()
                                        .text_color(theme.text_muted)
                                        .child("No objects found")
                                )
                            })
                            .when(self.results.is_empty() && self.query.is_empty(), |div| {
                                div.child(
                                    div()
                                        .p_8()
                                        .text_center()
                                        .text_color(theme.text_muted)
                                        .child("Type to search across all schemas")
                                )
                            })
                            .when(!self.results.is_empty(), |div| {
                                div.children(
                                    self.results.iter().enumerate().map(|(i, result)| {
                                        self.render_result(result, i, cx)
                                    })
                                )
                            })
                    )
            )
    }
}
```

### 16.6 DDL Generator

```rust
// src/schema_browser/ddl.rs

use crate::schema::{Table, View, Function, Index, ForeignKey, Column};

pub struct DdlGenerator;

impl DdlGenerator {
    pub fn generate_create_table(table: &Table) -> String {
        let mut ddl = format!(
            "CREATE TABLE \"{}\".\"{}\" (\n",
            table.schema, table.name
        );

        // Columns
        let column_defs: Vec<String> = table.columns.iter()
            .map(Self::generate_column_def)
            .collect();
        ddl.push_str(&column_defs.join(",\n"));

        // Primary key
        if let Some(pk) = &table.primary_key {
            ddl.push_str(&format!(
                ",\n  CONSTRAINT \"{}\" PRIMARY KEY ({})",
                pk.name,
                pk.columns.iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Unique constraints
        for uc in &table.unique_constraints {
            ddl.push_str(&format!(
                ",\n  CONSTRAINT \"{}\" UNIQUE ({})",
                uc.name,
                uc.columns.iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Check constraints
        for cc in &table.check_constraints {
            ddl.push_str(&format!(
                ",\n  CONSTRAINT \"{}\" CHECK ({})",
                cc.name, cc.expression
            ));
        }

        // Foreign keys
        for fk in &table.foreign_keys {
            ddl.push_str(&Self::generate_fk_constraint(fk));
        }

        ddl.push_str("\n);\n\n");

        // Indexes (non-primary, non-unique constraint)
        for index in &table.indexes {
            if !index.is_primary && !index.is_unique {
                ddl.push_str(&Self::generate_create_index(index, &table.schema, &table.name));
                ddl.push('\n');
            }
        }

        // Table comment
        if let Some(comment) = &table.comment {
            ddl.push_str(&format!(
                "\nCOMMENT ON TABLE \"{}\".\"{}\" IS {};\n",
                table.schema, table.name, Self::quote_string(comment)
            ));
        }

        // Column comments
        for col in &table.columns {
            if let Some(comment) = &col.comment {
                ddl.push_str(&format!(
                    "COMMENT ON COLUMN \"{}\".\"{}\".\"{}\" IS {};\n",
                    table.schema, table.name, col.name, Self::quote_string(comment)
                ));
            }
        }

        ddl
    }

    fn generate_column_def(col: &Column) -> String {
        let mut def = format!("  \"{}\" {}", col.name, col.type_name);

        if col.is_identity {
            let gen = col.identity_generation.as_deref().unwrap_or("BY DEFAULT");
            def.push_str(&format!(" GENERATED {} AS IDENTITY", gen));
        } else if col.is_generated {
            if let Some(expr) = &col.generation_expression {
                def.push_str(&format!(" GENERATED ALWAYS AS ({}) STORED", expr));
            }
        } else if let Some(default) = &col.default {
            def.push_str(&format!(" DEFAULT {}", default));
        }

        if !col.nullable {
            def.push_str(" NOT NULL");
        }

        def
    }

    fn generate_fk_constraint(fk: &ForeignKey) -> String {
        format!(
            ",\n  CONSTRAINT \"{}\" FOREIGN KEY ({}) REFERENCES \"{}\".\"{}\"({}) ON DELETE {} ON UPDATE {}{}",
            fk.name,
            fk.columns.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", "),
            fk.referenced_schema,
            fk.referenced_table,
            fk.referenced_columns.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", "),
            fk.on_delete,
            fk.on_update,
            if fk.deferrable {
                format!(" DEFERRABLE{}", if fk.initially_deferred { " INITIALLY DEFERRED" } else { "" })
            } else {
                String::new()
            }
        )
    }

    pub fn generate_create_index(index: &Index, schema: &str, table: &str) -> String {
        let mut ddl = String::from("CREATE ");

        if index.is_unique {
            ddl.push_str("UNIQUE ");
        }

        ddl.push_str(&format!(
            "INDEX \"{}\" ON \"{}\".\"{}\" USING {} ({})",
            index.name,
            schema,
            table,
            index.method,
            index.columns.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ")
        ));

        if !index.include_columns.is_empty() {
            ddl.push_str(&format!(
                " INCLUDE ({})",
                index.include_columns.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ")
            ));
        }

        if let Some(pred) = &index.predicate {
            ddl.push_str(&format!(" WHERE {}", pred));
        }

        ddl.push(';');
        ddl
    }

    pub fn generate_create_view(view: &View) -> String {
        format!(
            "CREATE OR REPLACE VIEW \"{}\".\"{}\" AS\n{};\n",
            view.schema, view.name, view.definition
        )
    }

    pub fn generate_create_materialized_view(view: &View) -> String {
        format!(
            "CREATE MATERIALIZED VIEW \"{}\".\"{}\" AS\n{};\n",
            view.schema, view.name, view.definition
        )
    }

    pub fn generate_create_function(func: &Function) -> String {
        let mut ddl = format!(
            "CREATE OR REPLACE FUNCTION \"{}\".\"{}\"({})\n",
            func.schema, func.name, func.arguments
        );

        ddl.push_str(&format!("RETURNS {}\n", func.return_type));
        ddl.push_str(&format!("LANGUAGE {}\n", func.language));

        match func.volatility.as_str() {
            "i" => ddl.push_str("IMMUTABLE\n"),
            "s" => ddl.push_str("STABLE\n"),
            "v" => ddl.push_str("VOLATILE\n"),
            _ => {}
        }

        if func.is_strict {
            ddl.push_str("STRICT\n");
        }

        if func.is_security_definer {
            ddl.push_str("SECURITY DEFINER\n");
        }

        ddl.push_str("AS $function$\n");
        ddl.push_str(&func.source);
        ddl.push_str("\n$function$;\n");

        if let Some(comment) = &func.comment {
            ddl.push_str(&format!(
                "\nCOMMENT ON FUNCTION \"{}\".\"{}\"({}) IS {};\n",
                func.schema, func.name, func.arguments, Self::quote_string(comment)
            ));
        }

        ddl
    }

    pub fn generate_drop_statement(object_type: &str, schema: &str, name: &str, cascade: bool) -> String {
        let cascade_str = if cascade { " CASCADE" } else { "" };
        format!(
            "DROP {} IF EXISTS \"{}\".\"{}\"{};",
            object_type.to_uppercase(),
            schema,
            name,
            cascade_str
        )
    }

    pub fn generate_truncate(schema: &str, table: &str, cascade: bool) -> String {
        let cascade_str = if cascade { " CASCADE" } else { "" };
        format!("TRUNCATE TABLE \"{}\".\"{}\"{};", schema, table, cascade_str)
    }

    pub fn generate_refresh_materialized_view(schema: &str, name: &str, concurrently: bool) -> String {
        let concurrent = if concurrently { "CONCURRENTLY " } else { "" };
        format!("REFRESH MATERIALIZED VIEW {}\"{}\".\"{}\";\n", concurrent, schema, name)
    }

    pub fn generate_reindex(schema: &str, index_name: &str) -> String {
        format!("REINDEX INDEX \"{}\".\"{}\";", schema, index_name)
    }

    fn quote_string(s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }
}
```

### 16.7 Keyboard Shortcuts

```rust
// src/schema_browser/shortcuts.rs

use gpui::*;

/// Register schema browser keyboard shortcuts
pub fn register_schema_browser_shortcuts(cx: &mut AppContext) {
    // Open object search (command palette)
    cx.bind_keys([
        KeyBinding::new("ctrl-p", OpenObjectSearch, Some("Workspace")),
        KeyBinding::new("cmd-p", OpenObjectSearch, Some("Workspace")),
        KeyBinding::new("ctrl-shift-p", OpenObjectSearch, Some("Workspace")),
        KeyBinding::new("cmd-shift-p", OpenObjectSearch, Some("Workspace")),
    ]);

    // Refresh schema
    cx.bind_keys([
        KeyBinding::new("ctrl-shift-r", RefreshSchema, Some("SchemaTree")),
        KeyBinding::new("cmd-shift-r", RefreshSchema, Some("SchemaTree")),
        KeyBinding::new("f5", RefreshSchema, Some("SchemaTree")),
    ]);

    // Navigate tree
    cx.bind_keys([
        KeyBinding::new("up", TreeMoveUp, Some("SchemaTree")),
        KeyBinding::new("down", TreeMoveDown, Some("SchemaTree")),
        KeyBinding::new("left", TreeCollapse, Some("SchemaTree")),
        KeyBinding::new("right", TreeExpand, Some("SchemaTree")),
        KeyBinding::new("enter", TreeActivate, Some("SchemaTree")),
        KeyBinding::new("space", TreeToggle, Some("SchemaTree")),
    ]);
}

#[derive(Clone, PartialEq)]
pub struct OpenObjectSearch;

impl_actions!(schema_browser, [OpenObjectSearch]);

#[derive(Clone, PartialEq)]
pub struct RefreshSchema;

impl_actions!(schema_browser, [RefreshSchema]);

#[derive(Clone, PartialEq)]
pub struct TreeMoveUp;

#[derive(Clone, PartialEq)]
pub struct TreeMoveDown;

#[derive(Clone, PartialEq)]
pub struct TreeCollapse;

#[derive(Clone, PartialEq)]
pub struct TreeExpand;

#[derive(Clone, PartialEq)]
pub struct TreeActivate;

#[derive(Clone, PartialEq)]
pub struct TreeToggle;

impl_actions!(schema_browser, [TreeMoveUp, TreeMoveDown, TreeCollapse, TreeExpand, TreeActivate, TreeToggle]);
```

## Acceptance Criteria

1. **Tree Display**
   - Show all database objects in hierarchical tree
   - Support lazy loading for large schemas
   - Display counts for folders (e.g., "Tables (42)")
   - Show size/row count for tables

2. **Tree Navigation**
   - Expand/collapse nodes
   - Single-click to select
   - Double-click to open
   - Keyboard navigation (arrows, enter, space)

3. **Context Menus**
   - Table: View Data, New Query, Edit, DDL, Drop, Truncate
   - View: View Data, Edit, DDL, Drop
   - Function: Open, Execute, DDL, Drop
   - Index: DDL, Reindex, Drop
   - Column: Add to Query, Copy Name

4. **DDL Generation**
   - Generate complete CREATE statements
   - Include comments, constraints, indexes
   - Support DROP with CASCADE option
   - TRUNCATE with CASCADE
   - REFRESH MATERIALIZED VIEW

5. **Object Search**
   - Fuzzy search across all objects
   - Keyboard navigation (up/down/enter/escape)
   - Show object type, schema, and parent
   - Open selected object in tab

6. **Refresh**
   - Manual refresh button
   - F5 keyboard shortcut
   - Preserve expanded state after refresh

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_builder() {
        let schemas = vec![
            Schema {
                name: "public".to_string(),
                tables: vec![
                    Table {
                        name: "users".to_string(),
                        schema: "public".to_string(),
                        columns: vec![
                            Column {
                                name: "id".to_string(),
                                type_name: "int4".to_string(),
                                nullable: false,
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        ];

        let tree = TreeBuilder::build_tree(Uuid::new_v4(), &schemas);

        assert_eq!(tree.node_type, NodeType::Connection);
        assert!(!tree.children.is_empty());

        let schemas_folder = &tree.children[0];
        assert_eq!(schemas_folder.node_type, NodeType::SchemasFolder);
        assert_eq!(schemas_folder.badge, Some("1".to_string()));
    }

    #[test]
    fn test_fuzzy_search() {
        assert!(SchemaBrowserState::fuzzy_match("users", "usr"));
        assert!(SchemaBrowserState::fuzzy_match("user_accounts", "usac"));
        assert!(!SchemaBrowserState::fuzzy_match("products", "usr"));
    }

    #[test]
    fn test_match_score() {
        // Exact match
        assert_eq!(SchemaBrowserState::match_score("users", "users"), 100);

        // Prefix match
        assert_eq!(SchemaBrowserState::match_score("users", "user"), 90);

        // Contains match
        assert!(SchemaBrowserState::match_score("all_users", "user") > 70);
    }

    #[test]
    fn test_ddl_generation() {
        let table = Table {
            name: "users".to_string(),
            schema: "public".to_string(),
            columns: vec![
                Column {
                    name: "id".to_string(),
                    type_name: "integer".to_string(),
                    nullable: false,
                    is_identity: true,
                    identity_generation: Some("BY DEFAULT".to_string()),
                    ..Default::default()
                },
                Column {
                    name: "name".to_string(),
                    type_name: "text".to_string(),
                    nullable: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let ddl = DdlGenerator::generate_create_table(&table);

        assert!(ddl.contains("CREATE TABLE \"public\".\"users\""));
        assert!(ddl.contains("\"id\" integer GENERATED BY DEFAULT AS IDENTITY"));
        assert!(ddl.contains("\"name\" text NOT NULL"));
    }
}
```

## Dependencies

### Rust Crates

```toml
[dependencies]
# Clipboard for copy operations
arboard = "3.4"

# UUID
uuid = { version = "1.0", features = ["v4"] }
```

## Module Structure

```
src/
├── schema_browser/
│   ├── mod.rs
│   ├── models.rs        # Tree node types and context menu
│   ├── state.rs         # Global state with search
│   ├── tree_builder.rs  # Build tree from schema data
│   ├── ddl.rs           # DDL generation
│   └── shortcuts.rs     # Keyboard shortcuts
├── ui/
│   ├── schema_tree.rs   # GPUI tree component
│   └── object_search.rs # Command palette dialog
```
