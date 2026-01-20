# Feature 26: ER Diagram Visualization

## Overview

The ER Diagram feature provides visual entity-relationship diagrams for PostgreSQL schemas. Using @xyflow/svelte, tables are rendered as interactive nodes with columns, and foreign keys are displayed as directed edges connecting related tables. The diagram supports multiple layout algorithms, manual positioning, zoom/pan navigation, and export to PNG/SVG formats.

## Goals

1. Generate visual ER diagrams from schema metadata
2. Display tables as nodes with configurable column visibility
3. Render foreign key relationships as directed edges
4. Support automatic layout algorithms (hierarchical, force-directed, circular)
5. Enable manual node positioning with persistence
6. Provide zoom, pan, and minimap navigation
7. Export diagrams to PNG and SVG formats
8. Color-code tables by schema
9. Save and restore diagram configurations

## Dependencies

- Feature 01: Project Setup (Tauri + Svelte)
- Feature 02: Local Storage (SQLite for diagram configs)
- Feature 10: Schema Introspection Service (table/FK metadata)

## Technical Specification

### 26.1 Rust Backend

The backend provides schema data for diagram generation and persists diagram configurations.

#### Diagram Configuration Storage

**File: `src-tauri/src/models/diagram.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ER Diagram configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramConfig {
    pub id: String,
    pub connection_id: String,
    pub name: String,
    pub schemas: Vec<String>,
    pub tables: Vec<DiagramTable>,
    pub options: DiagramOptions,
    pub layout: DiagramLayout,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Table inclusion in diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramTable {
    pub schema: String,
    pub name: String,
    pub included: bool,
    pub position: Option<NodePosition>,
}

/// Node position for manual layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

/// Diagram display options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramOptions {
    pub column_display: ColumnDisplay,
    pub show_data_types: bool,
    pub show_nullable: bool,
    pub show_indexes: bool,
    pub show_constraints: bool,
    pub color_by_schema: bool,
    pub snap_to_grid: bool,
    pub grid_size: u32,
}

impl Default for DiagramOptions {
    fn default() -> Self {
        Self {
            column_display: ColumnDisplay::All,
            show_data_types: true,
            show_nullable: true,
            show_indexes: false,
            show_constraints: true,
            color_by_schema: true,
            snap_to_grid: false,
            grid_size: 20,
        }
    }
}

/// Column visibility modes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ColumnDisplay {
    All,
    PkFkOnly,
    None,
}

/// Layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramLayout {
    pub algorithm: LayoutAlgorithm,
    pub node_positions: HashMap<String, NodePosition>,
    pub viewport: Viewport,
}

impl Default for DiagramLayout {
    fn default() -> Self {
        Self {
            algorithm: LayoutAlgorithm::Hierarchical,
            node_positions: HashMap::new(),
            viewport: Viewport::default(),
        }
    }
}

/// Supported layout algorithms
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LayoutAlgorithm {
    Hierarchical,
    ForceDirected,
    Circular,
}

/// Viewport state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

/// Diagram data for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramData {
    pub nodes: Vec<TableNode>,
    pub edges: Vec<RelationshipEdge>,
}

/// Table node for xyflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableNode {
    pub id: String,
    pub schema: String,
    pub name: String,
    pub columns: Vec<DiagramColumn>,
    pub indexes: Vec<DiagramIndex>,
    pub position: NodePosition,
    pub color: String,
}

/// Column info for diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub fk_reference: Option<String>,
}

/// Index info for diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramIndex {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
}

/// Relationship edge for xyflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEdge {
    pub id: String,
    pub source: String,
    pub source_handle: String,
    pub target: String,
    pub target_handle: String,
    pub label: String,
    pub relationship_type: RelationshipType,
}

/// FK relationship type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    OneToOne,
    OneToMany,
    ManyToMany,
}
```

#### Diagram Service

**File: `src-tauri/src/services/diagram.rs`**

```rust
use crate::models::diagram::*;
use crate::models::schema::{Table, Column, ForeignKey, Index};
use crate::services::schema::SchemaService;
use crate::services::storage::StorageService;
use crate::error::{Result, TuskError};
use std::collections::{HashMap, HashSet};

/// Schema colors for visual distinction
const SCHEMA_COLORS: &[&str] = &[
    "#3b82f6", // blue
    "#10b981", // emerald
    "#8b5cf6", // violet
    "#f59e0b", // amber
    "#ef4444", // red
    "#ec4899", // pink
    "#06b6d4", // cyan
    "#84cc16", // lime
];

pub struct DiagramService {
    schema_service: SchemaService,
    storage: StorageService,
}

impl DiagramService {
    pub fn new(schema_service: SchemaService, storage: StorageService) -> Self {
        Self { schema_service, storage }
    }

    /// Generate diagram data from schema
    pub async fn generate_diagram(
        &self,
        connection_id: &str,
        schemas: &[String],
        tables: Option<&[String]>,
        options: &DiagramOptions,
    ) -> Result<DiagramData> {
        // Fetch tables from specified schemas
        let mut all_tables: Vec<Table> = Vec::new();
        let mut schema_color_map: HashMap<String, String> = HashMap::new();

        for (idx, schema) in schemas.iter().enumerate() {
            schema_color_map.insert(
                schema.clone(),
                SCHEMA_COLORS[idx % SCHEMA_COLORS.len()].to_string()
            );

            let schema_tables = self.schema_service
                .list_tables(connection_id, schema)
                .await?;
            all_tables.extend(schema_tables);
        }

        // Filter to requested tables if specified
        if let Some(table_filter) = tables {
            let filter_set: HashSet<String> = table_filter.iter().cloned().collect();
            all_tables.retain(|t| {
                let full_name = format!("{}.{}", t.schema, t.name);
                filter_set.contains(&full_name)
            });
        }

        // Build nodes
        let nodes = self.build_nodes(&all_tables, options, &schema_color_map).await?;

        // Build edges from foreign keys
        let edges = self.build_edges(&all_tables, options).await?;

        Ok(DiagramData { nodes, edges })
    }

    /// Build table nodes
    async fn build_nodes(
        &self,
        tables: &[Table],
        options: &DiagramOptions,
        schema_colors: &HashMap<String, String>,
    ) -> Result<Vec<TableNode>> {
        let mut nodes = Vec::new();

        for (idx, table) in tables.iter().enumerate() {
            // Get columns for this table
            let columns = self.schema_service
                .list_columns(&table.connection_id, &table.schema, &table.name)
                .await?;

            // Get indexes if requested
            let indexes = if options.show_indexes {
                self.schema_service
                    .list_indexes(&table.connection_id, &table.schema, &table.name)
                    .await?
            } else {
                vec![]
            };

            // Get foreign keys to determine FK columns
            let fks = self.schema_service
                .list_foreign_keys(&table.connection_id, &table.schema, &table.name)
                .await?;

            let fk_columns: HashSet<String> = fks.iter()
                .flat_map(|fk| fk.columns.clone())
                .collect();

            // Get primary key columns
            let pk_columns: HashSet<String> = indexes.iter()
                .filter(|i| i.is_primary)
                .flat_map(|i| i.columns.clone())
                .collect();

            // Filter columns based on display option
            let diagram_columns: Vec<DiagramColumn> = columns.iter()
                .filter(|c| match options.column_display {
                    ColumnDisplay::All => true,
                    ColumnDisplay::PkFkOnly => pk_columns.contains(&c.name) || fk_columns.contains(&c.name),
                    ColumnDisplay::None => false,
                })
                .map(|c| {
                    // Find FK reference if exists
                    let fk_ref = fks.iter()
                        .find(|fk| fk.columns.contains(&c.name))
                        .map(|fk| format!("{}.{}", fk.referenced_table, fk.referenced_columns[0]));

                    DiagramColumn {
                        name: c.name.clone(),
                        data_type: if options.show_data_types { c.data_type.clone() } else { String::new() },
                        nullable: c.nullable && options.show_nullable,
                        is_primary_key: pk_columns.contains(&c.name),
                        is_foreign_key: fk_columns.contains(&c.name),
                        fk_reference: fk_ref,
                    }
                })
                .collect();

            // Convert indexes
            let diagram_indexes: Vec<DiagramIndex> = indexes.iter()
                .map(|i| DiagramIndex {
                    name: i.name.clone(),
                    columns: i.columns.clone(),
                    is_unique: i.is_unique,
                    is_primary: i.is_primary,
                })
                .collect();

            // Initial position (will be adjusted by layout algorithm)
            let position = NodePosition {
                x: (idx % 5) as f64 * 300.0,
                y: (idx / 5) as f64 * 400.0,
            };

            let color = if options.color_by_schema {
                schema_colors.get(&table.schema)
                    .cloned()
                    .unwrap_or_else(|| "#6b7280".to_string())
            } else {
                "#6b7280".to_string()
            };

            nodes.push(TableNode {
                id: format!("{}.{}", table.schema, table.name),
                schema: table.schema.clone(),
                name: table.name.clone(),
                columns: diagram_columns,
                indexes: diagram_indexes,
                position,
                color,
            });
        }

        Ok(nodes)
    }

    /// Build relationship edges from foreign keys
    async fn build_edges(
        &self,
        tables: &[Table],
        options: &DiagramOptions,
    ) -> Result<Vec<RelationshipEdge>> {
        let mut edges = Vec::new();
        let table_set: HashSet<String> = tables.iter()
            .map(|t| format!("{}.{}", t.schema, t.name))
            .collect();

        for table in tables {
            let fks = self.schema_service
                .list_foreign_keys(&table.connection_id, &table.schema, &table.name)
                .await?;

            for fk in fks {
                let target_id = format!("{}.{}", fk.referenced_schema, fk.referenced_table);

                // Only include edge if target table is in diagram
                if !table_set.contains(&target_id) {
                    continue;
                }

                let source_id = format!("{}.{}", table.schema, table.name);

                // Determine relationship type based on unique constraints
                let rel_type = self.determine_relationship_type(&fk, table).await?;

                edges.push(RelationshipEdge {
                    id: format!("{}_{}", source_id, fk.name),
                    source: source_id,
                    source_handle: fk.columns.join(","),
                    target: target_id,
                    target_handle: fk.referenced_columns.join(","),
                    label: fk.name.clone(),
                    relationship_type: rel_type,
                });
            }
        }

        Ok(edges)
    }

    /// Determine relationship cardinality
    async fn determine_relationship_type(
        &self,
        fk: &ForeignKey,
        table: &Table,
    ) -> Result<RelationshipType> {
        // Get indexes to check if FK columns have unique constraint
        let indexes = self.schema_service
            .list_indexes(&table.connection_id, &table.schema, &table.name)
            .await?;

        // Check if FK columns are unique (one-to-one)
        let fk_columns: HashSet<&str> = fk.columns.iter().map(|s| s.as_str()).collect();

        let is_unique = indexes.iter().any(|idx| {
            if !idx.is_unique { return false; }
            let idx_cols: HashSet<&str> = idx.columns.iter().map(|s| s.as_str()).collect();
            idx_cols == fk_columns
        });

        if is_unique {
            Ok(RelationshipType::OneToOne)
        } else {
            Ok(RelationshipType::OneToMany)
        }
    }

    /// Apply layout algorithm to nodes
    pub fn apply_layout(
        &self,
        nodes: &mut [TableNode],
        edges: &[RelationshipEdge],
        algorithm: LayoutAlgorithm,
    ) {
        match algorithm {
            LayoutAlgorithm::Hierarchical => self.hierarchical_layout(nodes, edges),
            LayoutAlgorithm::ForceDirected => self.force_directed_layout(nodes, edges),
            LayoutAlgorithm::Circular => self.circular_layout(nodes),
        }
    }

    /// Hierarchical layout (Sugiyama-style)
    fn hierarchical_layout(&self, nodes: &mut [TableNode], edges: &[RelationshipEdge]) {
        // Build adjacency map
        let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
        let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();

        for edge in edges {
            outgoing.entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
            incoming.entry(edge.target.clone())
                .or_default()
                .push(edge.source.clone());
        }

        // Assign layers (topological sort)
        let mut layers: HashMap<String, usize> = HashMap::new();
        let mut remaining: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();

        // Start with nodes that have no incoming edges (root tables)
        let mut current_layer = 0;
        while !remaining.is_empty() {
            let roots: Vec<String> = remaining.iter()
                .filter(|n| {
                    incoming.get(*n)
                        .map(|i| i.iter().all(|s| layers.contains_key(s)))
                        .unwrap_or(true)
                })
                .cloned()
                .collect();

            if roots.is_empty() {
                // Cycle detected, break and assign remaining to current layer
                for n in &remaining {
                    layers.insert(n.clone(), current_layer);
                }
                break;
            }

            for node_id in &roots {
                layers.insert(node_id.clone(), current_layer);
                remaining.remove(node_id);
            }
            current_layer += 1;
        }

        // Group nodes by layer
        let mut layer_groups: HashMap<usize, Vec<&mut TableNode>> = HashMap::new();
        for node in nodes.iter_mut() {
            let layer = *layers.get(&node.id).unwrap_or(&0);
            layer_groups.entry(layer).or_default().push(node);
        }

        // Position nodes
        let layer_height = 300.0;
        let node_width = 250.0;
        let padding = 50.0;

        for (layer, layer_nodes) in layer_groups.iter_mut() {
            let count = layer_nodes.len();
            let total_width = count as f64 * (node_width + padding) - padding;
            let start_x = -total_width / 2.0;

            for (idx, node) in layer_nodes.iter_mut().enumerate() {
                node.position.x = start_x + idx as f64 * (node_width + padding);
                node.position.y = *layer as f64 * layer_height;
            }
        }
    }

    /// Force-directed layout (simplified Fruchterman-Reingold)
    fn force_directed_layout(&self, nodes: &mut [TableNode], edges: &[RelationshipEdge]) {
        let iterations = 100;
        let k = 200.0; // Ideal edge length
        let temp_start = 100.0;

        // Build edge map
        let edge_pairs: Vec<(String, String)> = edges.iter()
            .map(|e| (e.source.clone(), e.target.clone()))
            .collect();

        for iter in 0..iterations {
            let temp = temp_start * (1.0 - iter as f64 / iterations as f64);

            // Calculate repulsive forces
            let mut forces: HashMap<String, (f64, f64)> = HashMap::new();

            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    let dx = nodes[j].position.x - nodes[i].position.x;
                    let dy = nodes[j].position.y - nodes[i].position.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);

                    // Repulsive force
                    let force = k * k / dist;
                    let fx = dx / dist * force;
                    let fy = dy / dist * force;

                    let entry_i = forces.entry(nodes[i].id.clone()).or_insert((0.0, 0.0));
                    entry_i.0 -= fx;
                    entry_i.1 -= fy;

                    let entry_j = forces.entry(nodes[j].id.clone()).or_insert((0.0, 0.0));
                    entry_j.0 += fx;
                    entry_j.1 += fy;
                }
            }

            // Calculate attractive forces
            for (source, target) in &edge_pairs {
                let source_node = nodes.iter().find(|n| &n.id == source);
                let target_node = nodes.iter().find(|n| &n.id == target);

                if let (Some(sn), Some(tn)) = (source_node, target_node) {
                    let dx = tn.position.x - sn.position.x;
                    let dy = tn.position.y - sn.position.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);

                    let force = dist * dist / k;
                    let fx = dx / dist * force;
                    let fy = dy / dist * force;

                    let entry_s = forces.entry(source.clone()).or_insert((0.0, 0.0));
                    entry_s.0 += fx;
                    entry_s.1 += fy;

                    let entry_t = forces.entry(target.clone()).or_insert((0.0, 0.0));
                    entry_t.0 -= fx;
                    entry_t.1 -= fy;
                }
            }

            // Apply forces with temperature limit
            for node in nodes.iter_mut() {
                if let Some((fx, fy)) = forces.get(&node.id) {
                    let mag = (fx * fx + fy * fy).sqrt().max(1.0);
                    let limited_mag = mag.min(temp);

                    node.position.x += fx / mag * limited_mag;
                    node.position.y += fy / mag * limited_mag;
                }
            }
        }
    }

    /// Circular layout
    fn circular_layout(&self, nodes: &mut [TableNode]) {
        let count = nodes.len();
        if count == 0 { return; }

        let radius = (count as f64 * 100.0).max(300.0);
        let angle_step = 2.0 * std::f64::consts::PI / count as f64;

        for (idx, node) in nodes.iter_mut().enumerate() {
            let angle = idx as f64 * angle_step - std::f64::consts::PI / 2.0;
            node.position.x = radius * angle.cos();
            node.position.y = radius * angle.sin();
        }
    }

    /// Save diagram configuration
    pub async fn save_diagram(&self, config: &DiagramConfig) -> Result<()> {
        let json = serde_json::to_string(config)?;
        self.storage.set(&format!("diagram:{}", config.id), &json).await
    }

    /// Load diagram configuration
    pub async fn load_diagram(&self, diagram_id: &str) -> Result<Option<DiagramConfig>> {
        let key = format!("diagram:{}", diagram_id);
        match self.storage.get(&key).await? {
            Some(json) => {
                let config: DiagramConfig = serde_json::from_str(&json)?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// List diagrams for connection
    pub async fn list_diagrams(&self, connection_id: &str) -> Result<Vec<DiagramConfig>> {
        let prefix = "diagram:";
        let all = self.storage.list_by_prefix(prefix).await?;

        let diagrams: Vec<DiagramConfig> = all.into_iter()
            .filter_map(|(_, json)| serde_json::from_str(&json).ok())
            .filter(|d: &DiagramConfig| d.connection_id == connection_id)
            .collect();

        Ok(diagrams)
    }

    /// Delete diagram
    pub async fn delete_diagram(&self, diagram_id: &str) -> Result<()> {
        self.storage.delete(&format!("diagram:{}", diagram_id)).await
    }
}
```

#### Tauri Commands

**File: `src-tauri/src/commands/diagram.rs`**

```rust
use crate::models::diagram::*;
use crate::services::diagram::DiagramService;
use crate::state::AppState;
use crate::error::Result;
use tauri::State;

/// Generate diagram data from schema
#[tauri::command]
pub async fn diagram_generate(
    state: State<'_, AppState>,
    connection_id: String,
    schemas: Vec<String>,
    tables: Option<Vec<String>>,
    options: DiagramOptions,
) -> Result<DiagramData> {
    let service = state.diagram_service.lock().await;
    service.generate_diagram(
        &connection_id,
        &schemas,
        tables.as_deref(),
        &options,
    ).await
}

/// Apply layout algorithm
#[tauri::command]
pub async fn diagram_apply_layout(
    state: State<'_, AppState>,
    mut diagram_data: DiagramData,
    algorithm: LayoutAlgorithm,
) -> Result<DiagramData> {
    let service = state.diagram_service.lock().await;
    service.apply_layout(&mut diagram_data.nodes, &diagram_data.edges, algorithm);
    Ok(diagram_data)
}

/// Save diagram configuration
#[tauri::command]
pub async fn diagram_save(
    state: State<'_, AppState>,
    config: DiagramConfig,
) -> Result<()> {
    let service = state.diagram_service.lock().await;
    service.save_diagram(&config).await
}

/// Load diagram configuration
#[tauri::command]
pub async fn diagram_load(
    state: State<'_, AppState>,
    diagram_id: String,
) -> Result<Option<DiagramConfig>> {
    let service = state.diagram_service.lock().await;
    service.load_diagram(&diagram_id).await
}

/// List diagrams for connection
#[tauri::command]
pub async fn diagram_list(
    state: State<'_, AppState>,
    connection_id: String,
) -> Result<Vec<DiagramConfig>> {
    let service = state.diagram_service.lock().await;
    service.list_diagrams(&connection_id).await
}

/// Delete diagram
#[tauri::command]
pub async fn diagram_delete(
    state: State<'_, AppState>,
    diagram_id: String,
) -> Result<()> {
    let service = state.diagram_service.lock().await;
    service.delete_diagram(&diagram_id).await
}
```

### 26.2 Svelte Frontend

#### Diagram Store

**File: `src/lib/stores/diagram.ts`**

```typescript
import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export interface NodePosition {
	x: number;
	y: number;
}

export interface DiagramColumn {
	name: string;
	data_type: string;
	nullable: boolean;
	is_primary_key: boolean;
	is_foreign_key: boolean;
	fk_reference: string | null;
}

export interface DiagramIndex {
	name: string;
	columns: string[];
	is_unique: boolean;
	is_primary: boolean;
}

export interface TableNode {
	id: string;
	schema: string;
	name: string;
	columns: DiagramColumn[];
	indexes: DiagramIndex[];
	position: NodePosition;
	color: string;
}

export interface RelationshipEdge {
	id: string;
	source: string;
	source_handle: string;
	target: string;
	target_handle: string;
	label: string;
	relationship_type: 'one_to_one' | 'one_to_many' | 'many_to_many';
}

export interface DiagramData {
	nodes: TableNode[];
	edges: RelationshipEdge[];
}

export type ColumnDisplay = 'all' | 'pk_fk_only' | 'none';
export type LayoutAlgorithm = 'hierarchical' | 'force_directed' | 'circular';

export interface DiagramOptions {
	column_display: ColumnDisplay;
	show_data_types: boolean;
	show_nullable: boolean;
	show_indexes: boolean;
	show_constraints: boolean;
	color_by_schema: boolean;
	snap_to_grid: boolean;
	grid_size: number;
}

export interface Viewport {
	x: number;
	y: number;
	zoom: number;
}

export interface DiagramConfig {
	id: string;
	connection_id: string;
	name: string;
	schemas: string[];
	tables: { schema: string; name: string; included: boolean; position: NodePosition | null }[];
	options: DiagramOptions;
	layout: {
		algorithm: LayoutAlgorithm;
		node_positions: Record<string, NodePosition>;
		viewport: Viewport;
	};
	created_at: number;
	updated_at: number;
}

function createDiagramStore() {
	const { subscribe, set, update } = writable<{
		data: DiagramData | null;
		config: DiagramConfig | null;
		loading: boolean;
		error: string | null;
		selectedNodes: Set<string>;
		viewport: Viewport;
	}>({
		data: null,
		config: null,
		loading: false,
		error: null,
		selectedNodes: new Set(),
		viewport: { x: 0, y: 0, zoom: 1 }
	});

	return {
		subscribe,

		async generate(
			connectionId: string,
			schemas: string[],
			tables: string[] | null,
			options: DiagramOptions
		) {
			update((s) => ({ ...s, loading: true, error: null }));

			try {
				const data = await invoke<DiagramData>('diagram_generate', {
					connectionId,
					schemas,
					tables,
					options
				});

				update((s) => ({ ...s, data, loading: false }));
				return data;
			} catch (e) {
				const error = e instanceof Error ? e.message : String(e);
				update((s) => ({ ...s, error, loading: false }));
				throw e;
			}
		},

		async applyLayout(algorithm: LayoutAlgorithm) {
			update((s) => {
				if (!s.data) return s;
				return { ...s, loading: true };
			});

			try {
				const current = await new Promise<DiagramData | null>((resolve) => {
					const unsub = subscribe((s) => {
						resolve(s.data);
						unsub();
					});
				});

				if (!current) return;

				const data = await invoke<DiagramData>('diagram_apply_layout', {
					diagramData: current,
					algorithm
				});

				update((s) => ({ ...s, data, loading: false }));
			} catch (e) {
				update((s) => ({ ...s, loading: false }));
			}
		},

		updateNodePosition(nodeId: string, position: NodePosition) {
			update((s) => {
				if (!s.data) return s;

				const nodes = s.data.nodes.map((n) => (n.id === nodeId ? { ...n, position } : n));

				return {
					...s,
					data: { ...s.data, nodes }
				};
			});
		},

		setViewport(viewport: Viewport) {
			update((s) => ({ ...s, viewport }));
		},

		selectNode(nodeId: string, additive = false) {
			update((s) => {
				const selectedNodes = additive ? new Set(s.selectedNodes) : new Set<string>();

				if (s.selectedNodes.has(nodeId) && additive) {
					selectedNodes.delete(nodeId);
				} else {
					selectedNodes.add(nodeId);
				}

				return { ...s, selectedNodes };
			});
		},

		clearSelection() {
			update((s) => ({ ...s, selectedNodes: new Set() }));
		},

		async save(config: DiagramConfig) {
			await invoke('diagram_save', { config });
			update((s) => ({ ...s, config }));
		},

		async load(diagramId: string) {
			const config = await invoke<DiagramConfig | null>('diagram_load', { diagramId });
			if (config) {
				update((s) => ({ ...s, config }));
			}
			return config;
		},

		reset() {
			set({
				data: null,
				config: null,
				loading: false,
				error: null,
				selectedNodes: new Set(),
				viewport: { x: 0, y: 0, zoom: 1 }
			});
		}
	};
}

export const diagramStore = createDiagramStore();

// Derived stores
export const diagramNodes = derived(diagramStore, ($s) => $s.data?.nodes ?? []);
export const diagramEdges = derived(diagramStore, ($s) => $s.data?.edges ?? []);
export const diagramLoading = derived(diagramStore, ($s) => $s.loading);
```

#### Table Node Component

**File: `src/lib/components/diagram/TableNode.svelte`**

```svelte
<script lang="ts">
	import type { DiagramColumn, DiagramIndex } from '$lib/stores/diagram';
	import { KeyRound, Link, Hash } from 'lucide-svelte';

	export let id: string;
	export let schema: string;
	export let name: string;
	export let columns: DiagramColumn[];
	export let indexes: DiagramIndex[];
	export let color: string;
	export let selected: boolean = false;
	export let showDataTypes: boolean = true;
	export let showNullable: boolean = true;
	export let showIndexes: boolean = false;
</script>

<div class="table-node" class:selected style="--node-color: {color}">
	<!-- Header -->
	<div class="node-header">
		<span class="schema">{schema}.</span>
		<span class="name">{name}</span>
	</div>

	<!-- Columns -->
	{#if columns.length > 0}
		<div class="node-columns">
			{#each columns as column}
				<div class="column" class:pk={column.is_primary_key} class:fk={column.is_foreign_key}>
					<div class="column-icons">
						{#if column.is_primary_key}
							<KeyRound size={12} class="pk-icon" />
						{/if}
						{#if column.is_foreign_key}
							<Link size={12} class="fk-icon" />
						{/if}
					</div>

					<span class="column-name">{column.name}</span>

					{#if showDataTypes && column.data_type}
						<span class="column-type">{column.data_type}</span>
					{/if}

					{#if showNullable && column.nullable}
						<span class="nullable">?</span>
					{/if}
				</div>
			{/each}
		</div>
	{/if}

	<!-- Indexes -->
	{#if showIndexes && indexes.length > 0}
		<div class="node-indexes">
			{#each indexes as index}
				<div class="index" class:unique={index.is_unique}>
					<Hash size={12} />
					<span class="index-name">{index.name}</span>
					<span class="index-cols">({index.columns.join(', ')})</span>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.table-node {
		background: var(--bg-primary);
		border: 2px solid var(--node-color);
		border-radius: 8px;
		min-width: 200px;
		max-width: 300px;
		font-size: 12px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
		overflow: hidden;
	}

	.table-node.selected {
		box-shadow:
			0 0 0 3px var(--node-color),
			0 4px 12px rgba(0, 0, 0, 0.2);
	}

	.node-header {
		background: var(--node-color);
		color: white;
		padding: 8px 12px;
		font-weight: 600;
	}

	.schema {
		opacity: 0.8;
		font-weight: 400;
	}

	.node-columns {
		padding: 8px 0;
		border-bottom: 1px solid var(--border-color);
	}

	.column {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 12px;
	}

	.column:hover {
		background: var(--bg-hover);
	}

	.column.pk .column-name,
	.column.fk .column-name {
		font-weight: 500;
	}

	.column-icons {
		display: flex;
		gap: 2px;
		width: 28px;
		flex-shrink: 0;
	}

	.pk-icon {
		color: var(--warning-color);
	}

	.fk-icon {
		color: var(--info-color);
	}

	.column-name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.column-type {
		color: var(--text-secondary);
		font-family: var(--font-mono);
		font-size: 11px;
	}

	.nullable {
		color: var(--text-tertiary);
		font-style: italic;
	}

	.node-indexes {
		padding: 8px 0;
	}

	.index {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 2px 12px;
		color: var(--text-secondary);
		font-size: 11px;
	}

	.index.unique {
		color: var(--success-color);
	}

	.index-cols {
		opacity: 0.7;
	}
</style>
```

#### Relationship Edge Component

**File: `src/lib/components/diagram/RelationshipEdge.svelte`**

```svelte
<script lang="ts">
	import type { RelationshipEdge } from '$lib/stores/diagram';
	import { BaseEdge, EdgeLabelRenderer, getBezierPath, type EdgeProps } from '@xyflow/svelte';

	type $$Props = EdgeProps<RelationshipEdge>;

	export let id: $$Props['id'];
	export let sourceX: $$Props['sourceX'];
	export let sourceY: $$Props['sourceY'];
	export let targetX: $$Props['targetX'];
	export let targetY: $$Props['targetY'];
	export let sourcePosition: $$Props['sourcePosition'];
	export let targetPosition: $$Props['targetPosition'];
	export let data: RelationshipEdge;
	export let selected: boolean = false;

	$: [edgePath, labelX, labelY] = getBezierPath({
		sourceX,
		sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition
	});

	// Cardinality symbols
	const getSourceSymbol = (type: RelationshipEdge['relationship_type']) => {
		return '|'; // One side
	};

	const getTargetSymbol = (type: RelationshipEdge['relationship_type']) => {
		switch (type) {
			case 'one_to_one':
				return '|';
			case 'one_to_many':
				return '∞';
			case 'many_to_many':
				return '∞';
		}
	};
</script>

<BaseEdge {id} path={edgePath} class:selected />

<!-- Cardinality markers -->
<EdgeLabelRenderer>
	<div
		class="edge-label source-label"
		style="transform: translate(-50%, -50%) translate({sourceX}px, {sourceY}px)"
	>
		{getSourceSymbol(data.relationship_type)}
	</div>

	<div
		class="edge-label target-label"
		style="transform: translate(-50%, -50%) translate({targetX}px, {targetY}px)"
	>
		{getTargetSymbol(data.relationship_type)}
	</div>

	{#if data.label && selected}
		<div
			class="edge-name"
			style="transform: translate(-50%, -50%) translate({labelX}px, {labelY}px)"
		>
			{data.label}
		</div>
	{/if}
</EdgeLabelRenderer>

<style>
	:global(.svelte-flow__edge-path) {
		stroke: var(--text-tertiary);
		stroke-width: 2;
	}

	:global(.svelte-flow__edge-path.selected) {
		stroke: var(--primary-color);
		stroke-width: 3;
	}

	.edge-label {
		position: absolute;
		font-size: 14px;
		font-weight: 600;
		color: var(--text-secondary);
		pointer-events: none;
		background: var(--bg-primary);
		padding: 2px 4px;
		border-radius: 4px;
	}

	.edge-name {
		position: absolute;
		font-size: 10px;
		color: var(--text-tertiary);
		background: var(--bg-secondary);
		padding: 2px 6px;
		border-radius: 4px;
		pointer-events: none;
	}
</style>
```

#### ER Diagram Container

**File: `src/lib/components/diagram/ERDiagram.svelte`**

```svelte
<script lang="ts">
	import { SvelteFlow, Controls, MiniMap, Background, type Node, type Edge } from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';

	import {
		diagramStore,
		diagramNodes,
		diagramEdges,
		type LayoutAlgorithm
	} from '$lib/stores/diagram';
	import TableNode from './TableNode.svelte';
	import RelationshipEdge from './RelationshipEdge.svelte';
	import DiagramToolbar from './DiagramToolbar.svelte';
	import DiagramOptionsPanel from './DiagramOptionsPanel.svelte';

	export let connectionId: string;

	const nodeTypes = {
		tableNode: TableNode
	};

	const edgeTypes = {
		relationship: RelationshipEdge
	};

	// Convert to xyflow format
	$: nodes = $diagramNodes.map((n) => ({
		id: n.id,
		type: 'tableNode',
		position: n.position,
		data: n,
		selected: $diagramStore.selectedNodes.has(n.id)
	})) as Node[];

	$: edges = $diagramEdges.map((e) => ({
		id: e.id,
		source: e.source,
		target: e.target,
		sourceHandle: e.source_handle,
		targetHandle: e.target_handle,
		type: 'relationship',
		data: e,
		animated: false
	})) as Edge[];

	function handleNodesChange(changes: any[]) {
		for (const change of changes) {
			if (change.type === 'position' && change.position) {
				diagramStore.updateNodePosition(change.id, change.position);
			}
		}
	}

	function handleNodeClick(event: CustomEvent<{ node: Node }>) {
		const additive = event.detail.node && (event as any).shiftKey;
		diagramStore.selectNode(event.detail.node.id, additive);
	}

	function handlePaneClick() {
		diagramStore.clearSelection();
	}

	function handleViewportChange(viewport: any) {
		diagramStore.setViewport({
			x: viewport.x,
			y: viewport.y,
			zoom: viewport.zoom
		});
	}

	async function handleLayoutChange(algorithm: LayoutAlgorithm) {
		await diagramStore.applyLayout(algorithm);
	}
</script>

<div class="diagram-container">
	<DiagramToolbar {connectionId} onLayoutChange={handleLayoutChange} />

	<div class="diagram-canvas">
		<SvelteFlow
			{nodes}
			{edges}
			{nodeTypes}
			{edgeTypes}
			fitView
			minZoom={0.1}
			maxZoom={2}
			onNodesChange={handleNodesChange}
			on:nodeclick={handleNodeClick}
			on:paneclick={handlePaneClick}
			on:viewportchange={(e) => handleViewportChange(e.detail)}
		>
			<Controls position="bottom-left" />
			<MiniMap position="bottom-right" nodeColor={(node) => node.data?.color ?? '#6b7280'} />
			<Background gap={20} />
		</SvelteFlow>
	</div>

	<DiagramOptionsPanel />
</div>

<style>
	.diagram-container {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--bg-secondary);
	}

	.diagram-canvas {
		flex: 1;
		position: relative;
	}

	:global(.svelte-flow) {
		background: var(--bg-primary);
	}

	:global(.svelte-flow__minimap) {
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
	}

	:global(.svelte-flow__controls) {
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
	}

	:global(.svelte-flow__controls-button) {
		background: var(--bg-primary);
		border-color: var(--border-color);
	}

	:global(.svelte-flow__controls-button:hover) {
		background: var(--bg-hover);
	}
</style>
```

#### Diagram Toolbar

**File: `src/lib/components/diagram/DiagramToolbar.svelte`**

```svelte
<script lang="ts">
	import { diagramStore, type LayoutAlgorithm, type DiagramOptions } from '$lib/stores/diagram';
	import {
		Download,
		ZoomIn,
		ZoomOut,
		Maximize,
		LayoutGrid,
		Share2,
		Circle,
		Save
	} from 'lucide-svelte';
	import Button from '$lib/components/common/Button.svelte';
	import Select from '$lib/components/common/Select.svelte';

	export let connectionId: string;
	export let onLayoutChange: (algorithm: LayoutAlgorithm) => void;

	let selectedLayout: LayoutAlgorithm = 'hierarchical';

	const layoutOptions = [
		{ value: 'hierarchical', label: 'Hierarchical' },
		{ value: 'force_directed', label: 'Force Directed' },
		{ value: 'circular', label: 'Circular' }
	];

	function handleLayoutChange() {
		onLayoutChange(selectedLayout);
	}

	async function handleExportPng() {
		// Get the SVG element from the flow
		const svg = document.querySelector('.svelte-flow__viewport');
		if (!svg) return;

		// Create canvas and render
		const canvas = document.createElement('canvas');
		const bbox = svg.getBoundingClientRect();
		const scale = 2; // 2x DPI

		canvas.width = bbox.width * scale;
		canvas.height = bbox.height * scale;

		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		ctx.scale(scale, scale);
		ctx.fillStyle = 'white';
		ctx.fillRect(0, 0, canvas.width, canvas.height);

		// Convert SVG to data URL and draw
		const svgData = new XMLSerializer().serializeToString(svg);
		const svgBlob = new Blob([svgData], { type: 'image/svg+xml;charset=utf-8' });
		const url = URL.createObjectURL(svgBlob);

		const img = new Image();
		img.onload = () => {
			ctx.drawImage(img, 0, 0);
			URL.revokeObjectURL(url);

			// Download
			const link = document.createElement('a');
			link.download = 'er-diagram.png';
			link.href = canvas.toDataURL('image/png');
			link.click();
		};
		img.src = url;
	}

	async function handleExportSvg() {
		const svg = document.querySelector('.svelte-flow__viewport');
		if (!svg) return;

		const svgData = new XMLSerializer().serializeToString(svg);
		const blob = new Blob([svgData], { type: 'image/svg+xml;charset=utf-8' });

		const link = document.createElement('a');
		link.download = 'er-diagram.svg';
		link.href = URL.createObjectURL(blob);
		link.click();
	}

	async function handleSave() {
		// Save current diagram configuration
		const state = $diagramStore;
		if (!state.data || !state.config) return;

		const config = {
			...state.config,
			layout: {
				...state.config.layout,
				node_positions: Object.fromEntries(state.data.nodes.map((n) => [n.id, n.position])),
				viewport: state.viewport
			},
			updated_at: Date.now()
		};

		await diagramStore.save(config);
	}
</script>

<div class="toolbar">
	<div class="toolbar-section">
		<span class="section-label">Layout:</span>
		<Select bind:value={selectedLayout} options={layoutOptions} on:change={handleLayoutChange} />
		<Button variant="ghost" size="sm" on:click={handleLayoutChange} title="Apply Layout">
			<LayoutGrid size={16} />
		</Button>
	</div>

	<div class="toolbar-divider" />

	<div class="toolbar-section">
		<Button variant="ghost" size="sm" title="Fit View">
			<Maximize size={16} />
		</Button>
		<Button variant="ghost" size="sm" title="Zoom In">
			<ZoomIn size={16} />
		</Button>
		<Button variant="ghost" size="sm" title="Zoom Out">
			<ZoomOut size={16} />
		</Button>
	</div>

	<div class="toolbar-divider" />

	<div class="toolbar-section">
		<Button variant="ghost" size="sm" on:click={handleExportPng} title="Export PNG">
			<Download size={16} />
			PNG
		</Button>
		<Button variant="ghost" size="sm" on:click={handleExportSvg} title="Export SVG">
			<Download size={16} />
			SVG
		</Button>
	</div>

	<div class="toolbar-spacer" />

	<div class="toolbar-section">
		<Button variant="primary" size="sm" on:click={handleSave}>
			<Save size={16} />
			Save Diagram
		</Button>
	</div>
</div>

<style>
	.toolbar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 16px;
		background: var(--bg-primary);
		border-bottom: 1px solid var(--border-color);
	}

	.toolbar-section {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.section-label {
		font-size: 12px;
		color: var(--text-secondary);
		margin-right: 4px;
	}

	.toolbar-divider {
		width: 1px;
		height: 24px;
		background: var(--border-color);
		margin: 0 8px;
	}

	.toolbar-spacer {
		flex: 1;
	}
</style>
```

#### Diagram Options Panel

**File: `src/lib/components/diagram/DiagramOptionsPanel.svelte`**

```svelte
<script lang="ts">
	import { diagramStore, type DiagramOptions, type ColumnDisplay } from '$lib/stores/diagram';
	import { Settings, ChevronDown, ChevronRight } from 'lucide-svelte';
	import Checkbox from '$lib/components/common/Checkbox.svelte';
	import Select from '$lib/components/common/Select.svelte';

	let expanded = false;

	let options: DiagramOptions = {
		column_display: 'all',
		show_data_types: true,
		show_nullable: true,
		show_indexes: false,
		show_constraints: true,
		color_by_schema: true,
		snap_to_grid: false,
		grid_size: 20
	};

	const columnDisplayOptions = [
		{ value: 'all', label: 'All Columns' },
		{ value: 'pk_fk_only', label: 'PK/FK Only' },
		{ value: 'none', label: 'No Columns' }
	];

	function togglePanel() {
		expanded = !expanded;
	}
</script>

<div class="options-panel" class:expanded>
	<button class="panel-toggle" on:click={togglePanel}>
		<Settings size={16} />
		<span>Display Options</span>
		{#if expanded}
			<ChevronDown size={16} />
		{:else}
			<ChevronRight size={16} />
		{/if}
	</button>

	{#if expanded}
		<div class="panel-content">
			<div class="option-group">
				<label class="option-label">Show Columns</label>
				<Select bind:value={options.column_display} options={columnDisplayOptions} />
			</div>

			<div class="option-group">
				<Checkbox bind:checked={options.show_data_types}>Show data types</Checkbox>
			</div>

			<div class="option-group">
				<Checkbox bind:checked={options.show_nullable}>Show nullable indicators</Checkbox>
			</div>

			<div class="option-group">
				<Checkbox bind:checked={options.show_indexes}>Show indexes</Checkbox>
			</div>

			<div class="option-group">
				<Checkbox bind:checked={options.show_constraints}>Show constraints</Checkbox>
			</div>

			<div class="option-group">
				<Checkbox bind:checked={options.color_by_schema}>Color by schema</Checkbox>
			</div>

			<div class="option-group">
				<Checkbox bind:checked={options.snap_to_grid}>Snap to grid</Checkbox>
			</div>
		</div>
	{/if}
</div>

<style>
	.options-panel {
		position: absolute;
		top: 8px;
		right: 8px;
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
		z-index: 10;
		min-width: 200px;
	}

	.panel-toggle {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 10px 12px;
		background: none;
		border: none;
		cursor: pointer;
		font-size: 13px;
		color: var(--text-primary);
	}

	.panel-toggle:hover {
		background: var(--bg-hover);
	}

	.panel-toggle span {
		flex: 1;
		text-align: left;
	}

	.panel-content {
		padding: 12px;
		border-top: 1px solid var(--border-color);
	}

	.option-group {
		margin-bottom: 12px;
	}

	.option-group:last-child {
		margin-bottom: 0;
	}

	.option-label {
		display: block;
		font-size: 12px;
		color: var(--text-secondary);
		margin-bottom: 4px;
	}
</style>
```

#### Diagram Generation Dialog

**File: `src/lib/components/diagram/DiagramGeneratorDialog.svelte`**

```svelte
<script lang="ts">
	import { diagramStore, type DiagramOptions } from '$lib/stores/diagram';
	import { schemaStore } from '$lib/stores/schema';
	import Dialog from '$lib/components/common/Dialog.svelte';
	import Button from '$lib/components/common/Button.svelte';
	import Checkbox from '$lib/components/common/Checkbox.svelte';
	import TreeView from '$lib/components/common/TreeView.svelte';

	export let open = false;
	export let connectionId: string;

	let selectedSchemas: Set<string> = new Set(['public']);
	let selectedTables: Set<string> = new Set();
	let selectAllTables = true;

	let options: DiagramOptions = {
		column_display: 'all',
		show_data_types: true,
		show_nullable: true,
		show_indexes: false,
		show_constraints: true,
		color_by_schema: true,
		snap_to_grid: false,
		grid_size: 20
	};

	// Load schemas and tables
	$: schemas = $schemaStore.schemas;
	$: tables = $schemaStore.tables.filter((t) => selectedSchemas.has(t.schema));

	function toggleSchema(schema: string) {
		if (selectedSchemas.has(schema)) {
			selectedSchemas.delete(schema);
		} else {
			selectedSchemas.add(schema);
		}
		selectedSchemas = selectedSchemas;
	}

	function toggleTable(tableId: string) {
		if (selectedTables.has(tableId)) {
			selectedTables.delete(tableId);
		} else {
			selectedTables.add(tableId);
		}
		selectedTables = selectedTables;
	}

	async function generate() {
		const schemaList = Array.from(selectedSchemas);
		const tableList = selectAllTables ? null : Array.from(selectedTables);

		await diagramStore.generate(connectionId, schemaList, tableList, options);
		open = false;
	}
</script>

<Dialog bind:open title="Generate ER Diagram" size="lg">
	<div class="dialog-content">
		<div class="section">
			<h3>Select Schemas</h3>
			<div class="schema-list">
				{#each schemas as schema}
					<Checkbox
						checked={selectedSchemas.has(schema.name)}
						on:change={() => toggleSchema(schema.name)}
					>
						{schema.name}
					</Checkbox>
				{/each}
			</div>
		</div>

		<div class="section">
			<h3>Tables</h3>
			<Checkbox bind:checked={selectAllTables}>Include all tables from selected schemas</Checkbox>

			{#if !selectAllTables}
				<div class="table-list">
					{#each tables as table}
						<Checkbox
							checked={selectedTables.has(`${table.schema}.${table.name}`)}
							on:change={() => toggleTable(`${table.schema}.${table.name}`)}
						>
							<span class="table-schema">{table.schema}.</span>{table.name}
						</Checkbox>
					{/each}
				</div>
			{/if}
		</div>

		<div class="section">
			<h3>Display Options</h3>
			<div class="options-grid">
				<Checkbox bind:checked={options.show_data_types}>Show data types</Checkbox>
				<Checkbox bind:checked={options.show_nullable}>Show nullable indicators</Checkbox>
				<Checkbox bind:checked={options.show_indexes}>Show indexes</Checkbox>
				<Checkbox bind:checked={options.color_by_schema}>Color by schema</Checkbox>
			</div>
		</div>
	</div>

	<svelte:fragment slot="footer">
		<Button variant="ghost" on:click={() => (open = false)}>Cancel</Button>
		<Button variant="primary" on:click={generate}>Generate Diagram</Button>
	</svelte:fragment>
</Dialog>

<style>
	.dialog-content {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.section h3 {
		font-size: 14px;
		font-weight: 600;
		margin-bottom: 12px;
	}

	.schema-list,
	.table-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-height: 200px;
		overflow-y: auto;
		padding: 8px;
		background: var(--bg-secondary);
		border-radius: 6px;
	}

	.table-schema {
		color: var(--text-secondary);
	}

	.options-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 12px;
	}
</style>
```

### 26.3 IPC Command Registration

**File: `src-tauri/src/main.rs`** (add to invoke_handler)

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...

    // Diagram commands
    commands::diagram::diagram_generate,
    commands::diagram::diagram_apply_layout,
    commands::diagram::diagram_save,
    commands::diagram::diagram_load,
    commands::diagram::diagram_list,
    commands::diagram::diagram_delete,
])
```

## Acceptance Criteria

1. **Diagram Generation**
   - [ ] Generate diagrams from selected schemas/tables
   - [ ] Display tables as nodes with configurable columns
   - [ ] Show foreign key relationships as directed edges
   - [ ] Color-code tables by schema

2. **Layout Algorithms**
   - [ ] Hierarchical layout for dependency visualization
   - [ ] Force-directed layout for organic arrangement
   - [ ] Circular layout for overview

3. **Navigation**
   - [ ] Zoom with mouse wheel/trackpad
   - [ ] Pan by dragging canvas
   - [ ] Minimap for large diagrams
   - [ ] Fit-to-view control

4. **Node Interaction**
   - [ ] Drag nodes to reposition
   - [ ] Snap-to-grid option
   - [ ] Multi-select with Shift+click
   - [ ] Node selection highlighting

5. **Display Options**
   - [ ] Column visibility modes (all, PK/FK only, none)
   - [ ] Data type display toggle
   - [ ] Nullable indicator toggle
   - [ ] Index display toggle

6. **Export**
   - [ ] Export to PNG with configurable DPI
   - [ ] Export to SVG vector format
   - [ ] Save diagram configuration

7. **Persistence**
   - [ ] Save diagram layouts
   - [ ] Restore diagram configurations
   - [ ] Per-connection diagram list

## MCP Testing Instructions

### Using Tauri MCP

```typescript
// Generate and interact with ER diagram
await driver_session({ action: 'start', port: 9223 });

// Open diagram generator
await webview_click({ selector: '[data-testid="new-diagram"]' });

// Select schemas
await webview_click({ selector: '[data-testid="schema-public"]' });
await webview_click({ selector: '[data-testid="schema-auth"]' });

// Generate diagram
await webview_click({ selector: '[data-testid="generate-diagram"]' });

// Wait for diagram to render
await webview_wait_for({ type: 'selector', value: '.svelte-flow__node' });

// Screenshot the diagram
await webview_screenshot({ filePath: 'er-diagram.png' });

// Change layout
await webview_click({ selector: '[data-testid="layout-select"]' });
await webview_click({ selector: '[data-value="force_directed"]' });
await webview_click({ selector: '[data-testid="apply-layout"]' });

// Export PNG
await webview_click({ selector: '[data-testid="export-png"]' });

// Verify node interaction
const snapshot = await webview_dom_snapshot({ type: 'accessibility' });
console.log('Diagram nodes:', snapshot);

await driver_session({ action: 'stop' });
```

### Using Playwright MCP

```typescript
// Test diagram generation dialog
await browser_navigate({ url: 'http://localhost:1420' });

// Open new diagram dialog
await browser_click({ element: 'New Diagram button', ref: '[data-testid="new-diagram"]' });

// Verify schema selection
const snapshot = await browser_snapshot();
console.log('Schema list available');

// Select options and generate
await browser_fill_form({
	fields: [
		{ name: 'Show Data Types', type: 'checkbox', ref: '#show-data-types', value: 'true' },
		{ name: 'Color by Schema', type: 'checkbox', ref: '#color-by-schema', value: 'true' }
	]
});

await browser_click({ element: 'Generate button', ref: '[data-testid="generate-diagram"]' });

// Take screenshot of generated diagram
await browser_take_screenshot({ filename: 'er-diagram-test.png' });
```
