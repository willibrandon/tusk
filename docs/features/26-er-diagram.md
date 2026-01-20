# Feature 26: ER Diagram Visualization

## Overview

The ER Diagram feature provides visual entity-relationship diagrams for PostgreSQL schemas using GPUI's native canvas rendering. Tables are rendered as interactive nodes with columns, and foreign keys are displayed as directed edges connecting related tables. The diagram supports multiple layout algorithms, manual positioning, zoom/pan navigation, and export to PNG/SVG formats.

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

- Feature 02: Backend Architecture (Rust services)
- Feature 05: Local Storage (SQLite for diagram configs)
- Feature 10: Schema Introspection Service (table/FK metadata)

## Technical Specification

### 26.1 Data Models

**File: `src/models/diagram.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use gpui::Hsla;

/// Unique identifier for diagrams
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagramId(pub String);

impl DiagramId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// ER Diagram configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramConfig {
    pub id: DiagramId,
    pub connection_id: String,
    pub name: String,
    pub schemas: Vec<String>,
    pub tables: Vec<DiagramTableConfig>,
    pub options: DiagramOptions,
    pub layout: DiagramLayout,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Table inclusion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramTableConfig {
    pub schema: String,
    pub name: String,
    pub included: bool,
    pub position: Option<Point>,
}

/// 2D point for positioning
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
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
    pub show_minimap: bool,
    pub show_grid: bool,
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
            show_minimap: true,
            show_grid: true,
        }
    }
}

/// Column visibility modes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ColumnDisplay {
    #[default]
    All,
    PkFkOnly,
    None,
}

/// Layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramLayout {
    pub algorithm: LayoutAlgorithm,
    pub node_positions: HashMap<String, Point>,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LayoutAlgorithm {
    #[default]
    Hierarchical,
    ForceDirected,
    Circular,
}

impl LayoutAlgorithm {
    pub fn all() -> &'static [LayoutAlgorithm] {
        &[
            LayoutAlgorithm::Hierarchical,
            LayoutAlgorithm::ForceDirected,
            LayoutAlgorithm::Circular,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Hierarchical => "Hierarchical",
            Self::ForceDirected => "Force Directed",
            Self::Circular => "Circular",
        }
    }
}

/// Viewport state for pan/zoom
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

impl Viewport {
    pub fn screen_to_world(&self, screen_x: f64, screen_y: f64) -> Point {
        Point {
            x: (screen_x - self.x) / self.zoom,
            y: (screen_y - self.y) / self.zoom,
        }
    }

    pub fn world_to_screen(&self, world: &Point) -> Point {
        Point {
            x: world.x * self.zoom + self.x,
            y: world.y * self.zoom + self.y,
        }
    }
}

/// Complete diagram data for rendering
#[derive(Debug, Clone, Default)]
pub struct DiagramData {
    pub nodes: Vec<TableNode>,
    pub edges: Vec<RelationshipEdge>,
}

/// Table node for canvas rendering
#[derive(Debug, Clone)]
pub struct TableNode {
    pub id: String,
    pub schema: String,
    pub name: String,
    pub columns: Vec<DiagramColumn>,
    pub indexes: Vec<DiagramIndex>,
    pub position: Point,
    pub size: Size,
    pub color: Hsla,
    pub selected: bool,
    pub hovered: bool,
}

/// Size dimensions
#[derive(Debug, Clone, Copy, Default)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// Column info for diagram
#[derive(Debug, Clone)]
pub struct DiagramColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub fk_reference: Option<String>,
}

/// Index info for diagram
#[derive(Debug, Clone)]
pub struct DiagramIndex {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
}

/// Relationship edge for canvas rendering
#[derive(Debug, Clone)]
pub struct RelationshipEdge {
    pub id: String,
    pub source_node: String,
    pub source_column: String,
    pub target_node: String,
    pub target_column: String,
    pub label: String,
    pub relationship_type: RelationshipType,
    pub selected: bool,
    pub hovered: bool,
}

/// FK relationship cardinality
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    OneToOne,
    OneToMany,
    ManyToMany,
}

/// Bounding box for hit testing
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Point,
    pub max: Point,
}

impl BoundingBox {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            min: Point::new(x, y),
            max: Point::new(x + width, y + height),
        }
    }

    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
}

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Png,
    Svg,
}

/// Export options
#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub scale: f64,
    pub background: bool,
    pub padding: u32,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Png,
            scale: 2.0,
            background: true,
            padding: 50,
        }
    }
}
```

### 26.2 Global State Management

**File: `src/state/diagram_state.rs`**

```rust
use crate::models::diagram::*;
use gpui::Global;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Application-wide diagram state
pub struct DiagramState {
    inner: Arc<RwLock<DiagramStateInner>>,
}

struct DiagramStateInner {
    /// Currently loaded diagram data
    current_diagram: Option<DiagramData>,

    /// Diagram configuration
    config: Option<DiagramConfig>,

    /// Selected node IDs
    selected_nodes: HashSet<String>,

    /// Selected edge IDs
    selected_edges: HashSet<String>,

    /// Hovered node ID
    hovered_node: Option<String>,

    /// Hovered edge ID
    hovered_edge: Option<String>,

    /// Current viewport state
    viewport: Viewport,

    /// Display options
    options: DiagramOptions,

    /// Drag state
    drag_state: Option<DragState>,

    /// Loading state
    loading: bool,

    /// Error message
    error: Option<String>,

    /// Saved diagrams list
    saved_diagrams: Vec<DiagramConfig>,
}

/// Drag operation state
#[derive(Debug, Clone)]
pub struct DragState {
    pub node_id: String,
    pub start_position: Point,
    pub current_position: Point,
    pub offset: Point,
}

impl Global for DiagramState {}

impl DiagramState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(DiagramStateInner {
                current_diagram: None,
                config: None,
                selected_nodes: HashSet::new(),
                selected_edges: HashSet::new(),
                hovered_node: None,
                hovered_edge: None,
                viewport: Viewport::default(),
                options: DiagramOptions::default(),
                drag_state: None,
                loading: false,
                error: None,
                saved_diagrams: Vec::new(),
            })),
        }
    }

    pub fn is_loading(&self) -> bool {
        self.inner.read().loading
    }

    pub fn set_loading(&self, loading: bool) {
        self.inner.write().loading = loading;
    }

    pub fn error(&self) -> Option<String> {
        self.inner.read().error.clone()
    }

    pub fn set_error(&self, error: Option<String>) {
        self.inner.write().error = error;
    }

    pub fn current_diagram(&self) -> Option<DiagramData> {
        self.inner.read().current_diagram.clone()
    }

    pub fn set_diagram(&self, diagram: DiagramData) {
        let mut inner = self.inner.write();
        inner.current_diagram = Some(diagram);
        inner.error = None;
    }

    pub fn clear_diagram(&self) {
        let mut inner = self.inner.write();
        inner.current_diagram = None;
        inner.selected_nodes.clear();
        inner.selected_edges.clear();
        inner.hovered_node = None;
        inner.hovered_edge = None;
    }

    pub fn config(&self) -> Option<DiagramConfig> {
        self.inner.read().config.clone()
    }

    pub fn set_config(&self, config: DiagramConfig) {
        self.inner.write().config = Some(config);
    }

    pub fn viewport(&self) -> Viewport {
        self.inner.read().viewport
    }

    pub fn set_viewport(&self, viewport: Viewport) {
        self.inner.write().viewport = viewport;
    }

    pub fn pan(&self, dx: f64, dy: f64) {
        let mut inner = self.inner.write();
        inner.viewport.x += dx;
        inner.viewport.y += dy;
    }

    pub fn zoom(&self, factor: f64, center: Point) {
        let mut inner = self.inner.write();
        let old_zoom = inner.viewport.zoom;
        let new_zoom = (old_zoom * factor).clamp(0.1, 3.0);

        // Zoom towards mouse position
        inner.viewport.x = center.x - (center.x - inner.viewport.x) * new_zoom / old_zoom;
        inner.viewport.y = center.y - (center.y - inner.viewport.y) * new_zoom / old_zoom;
        inner.viewport.zoom = new_zoom;
    }

    pub fn fit_to_view(&self, canvas_width: f64, canvas_height: f64) {
        let inner = self.inner.read();
        if let Some(ref diagram) = inner.current_diagram {
            if diagram.nodes.is_empty() {
                return;
            }

            let bounds = self.calculate_bounds(&diagram.nodes);
            drop(inner);

            let padding = 50.0;
            let content_width = bounds.max.x - bounds.min.x + padding * 2.0;
            let content_height = bounds.max.y - bounds.min.y + padding * 2.0;

            let zoom = (canvas_width / content_width)
                .min(canvas_height / content_height)
                .min(1.5);

            let viewport = Viewport {
                x: canvas_width / 2.0 - (bounds.min.x + content_width / 2.0 - padding) * zoom,
                y: canvas_height / 2.0 - (bounds.min.y + content_height / 2.0 - padding) * zoom,
                zoom,
            };

            self.inner.write().viewport = viewport;
        }
    }

    fn calculate_bounds(&self, nodes: &[TableNode]) -> BoundingBox {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for node in nodes {
            min_x = min_x.min(node.position.x);
            min_y = min_y.min(node.position.y);
            max_x = max_x.max(node.position.x + node.size.width);
            max_y = max_y.max(node.position.y + node.size.height);
        }

        BoundingBox {
            min: Point::new(min_x, min_y),
            max: Point::new(max_x, max_y),
        }
    }

    pub fn options(&self) -> DiagramOptions {
        self.inner.read().options.clone()
    }

    pub fn set_options(&self, options: DiagramOptions) {
        self.inner.write().options = options;
    }

    pub fn selected_nodes(&self) -> HashSet<String> {
        self.inner.read().selected_nodes.clone()
    }

    pub fn select_node(&self, node_id: &str, additive: bool) {
        let mut inner = self.inner.write();
        if !additive {
            inner.selected_nodes.clear();
            inner.selected_edges.clear();
        }

        if inner.selected_nodes.contains(node_id) && additive {
            inner.selected_nodes.remove(node_id);
        } else {
            inner.selected_nodes.insert(node_id.to_string());
        }

        // Update node selection state
        if let Some(ref mut diagram) = inner.current_diagram {
            for node in &mut diagram.nodes {
                node.selected = inner.selected_nodes.contains(&node.id);
            }
        }
    }

    pub fn clear_selection(&self) {
        let mut inner = self.inner.write();
        inner.selected_nodes.clear();
        inner.selected_edges.clear();

        if let Some(ref mut diagram) = inner.current_diagram {
            for node in &mut diagram.nodes {
                node.selected = false;
            }
            for edge in &mut diagram.edges {
                edge.selected = false;
            }
        }
    }

    pub fn set_hover_node(&self, node_id: Option<&str>) {
        let mut inner = self.inner.write();
        inner.hovered_node = node_id.map(|s| s.to_string());

        if let Some(ref mut diagram) = inner.current_diagram {
            for node in &mut diagram.nodes {
                node.hovered = Some(&node.id) == node_id.as_ref().map(|s| s as &String);
            }
        }
    }

    pub fn set_hover_edge(&self, edge_id: Option<&str>) {
        let mut inner = self.inner.write();
        inner.hovered_edge = edge_id.map(|s| s.to_string());

        if let Some(ref mut diagram) = inner.current_diagram {
            for edge in &mut diagram.edges {
                edge.hovered = Some(&edge.id) == edge_id.as_ref().map(|s| s as &String);
            }
        }
    }

    pub fn start_drag(&self, node_id: &str, mouse_pos: Point) {
        let mut inner = self.inner.write();

        if let Some(ref diagram) = inner.current_diagram {
            if let Some(node) = diagram.nodes.iter().find(|n| n.id == node_id) {
                inner.drag_state = Some(DragState {
                    node_id: node_id.to_string(),
                    start_position: node.position,
                    current_position: node.position,
                    offset: Point::new(
                        mouse_pos.x - node.position.x,
                        mouse_pos.y - node.position.y,
                    ),
                });
            }
        }
    }

    pub fn update_drag(&self, mouse_pos: Point) {
        let mut inner = self.inner.write();

        if let Some(ref mut drag) = inner.drag_state {
            let mut new_pos = Point::new(
                mouse_pos.x - drag.offset.x,
                mouse_pos.y - drag.offset.y,
            );

            // Snap to grid if enabled
            if inner.options.snap_to_grid {
                let grid = inner.options.grid_size as f64;
                new_pos.x = (new_pos.x / grid).round() * grid;
                new_pos.y = (new_pos.y / grid).round() * grid;
            }

            drag.current_position = new_pos;

            // Update node position in diagram
            if let Some(ref mut diagram) = inner.current_diagram {
                if let Some(node) = diagram.nodes.iter_mut().find(|n| n.id == drag.node_id) {
                    node.position = new_pos;
                }
            }
        }
    }

    pub fn end_drag(&self) {
        self.inner.write().drag_state = None;
    }

    pub fn is_dragging(&self) -> bool {
        self.inner.read().drag_state.is_some()
    }

    pub fn update_node_position(&self, node_id: &str, position: Point) {
        let mut inner = self.inner.write();

        if let Some(ref mut diagram) = inner.current_diagram {
            if let Some(node) = diagram.nodes.iter_mut().find(|n| n.id == node_id) {
                node.position = position;
            }
        }

        // Also update in config
        if let Some(ref mut config) = inner.config {
            config.layout.node_positions.insert(node_id.to_string(), position);
        }
    }

    pub fn saved_diagrams(&self) -> Vec<DiagramConfig> {
        self.inner.read().saved_diagrams.clone()
    }

    pub fn set_saved_diagrams(&self, diagrams: Vec<DiagramConfig>) {
        self.inner.write().saved_diagrams = diagrams;
    }

    pub fn node_at_position(&self, world_pos: &Point) -> Option<String> {
        let inner = self.inner.read();
        if let Some(ref diagram) = inner.current_diagram {
            // Check in reverse order (top-most first)
            for node in diagram.nodes.iter().rev() {
                let bbox = BoundingBox::new(
                    node.position.x,
                    node.position.y,
                    node.size.width,
                    node.size.height,
                );
                if bbox.contains(world_pos) {
                    return Some(node.id.clone());
                }
            }
        }
        None
    }

    pub fn edge_at_position(&self, world_pos: &Point) -> Option<String> {
        let inner = self.inner.read();
        if let Some(ref diagram) = inner.current_diagram {
            // Check edges with tolerance
            let tolerance = 5.0;
            for edge in &diagram.edges {
                if self.point_near_edge(world_pos, edge, &diagram.nodes, tolerance) {
                    return Some(edge.id.clone());
                }
            }
        }
        None
    }

    fn point_near_edge(
        &self,
        point: &Point,
        edge: &RelationshipEdge,
        nodes: &[TableNode],
        tolerance: f64,
    ) -> bool {
        // Find source and target nodes
        let source = nodes.iter().find(|n| n.id == edge.source_node);
        let target = nodes.iter().find(|n| n.id == edge.target_node);

        if let (Some(src), Some(tgt)) = (source, target) {
            // Get edge endpoints (center-right of source, center-left of target)
            let start = Point::new(
                src.position.x + src.size.width,
                src.position.y + src.size.height / 2.0,
            );
            let end = Point::new(
                tgt.position.x,
                tgt.position.y + tgt.size.height / 2.0,
            );

            // Check distance from point to line segment
            let dist = self.point_to_segment_distance(point, &start, &end);
            return dist <= tolerance;
        }
        false
    }

    fn point_to_segment_distance(&self, point: &Point, start: &Point, end: &Point) -> f64 {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let len_sq = dx * dx + dy * dy;

        if len_sq == 0.0 {
            return point.distance(start);
        }

        let t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);

        let proj = Point::new(start.x + t * dx, start.y + t * dy);
        point.distance(&proj)
    }
}
```

### 26.3 Diagram Service

**File: `src/services/diagram.rs`**

```rust
use crate::models::diagram::*;
use crate::models::schema::{Column, ForeignKey, Index, Table};
use crate::services::schema::SchemaService;
use crate::services::storage::StorageService;
use crate::error::{Result, TuskError};
use gpui::Hsla;
use std::collections::{HashMap, HashSet};

/// Schema colors for visual distinction
const SCHEMA_COLORS: &[Hsla] = &[
    Hsla { h: 217.0 / 360.0, s: 0.91, l: 0.60, a: 1.0 }, // blue
    Hsla { h: 160.0 / 360.0, s: 0.84, l: 0.39, a: 1.0 }, // emerald
    Hsla { h: 262.0 / 360.0, s: 0.83, l: 0.58, a: 1.0 }, // violet
    Hsla { h: 38.0 / 360.0, s: 0.92, l: 0.50, a: 1.0 },  // amber
    Hsla { h: 0.0, s: 0.84, l: 0.60, a: 1.0 },           // red
    Hsla { h: 330.0 / 360.0, s: 0.81, l: 0.60, a: 1.0 }, // pink
    Hsla { h: 189.0 / 360.0, s: 0.94, l: 0.43, a: 1.0 }, // cyan
    Hsla { h: 84.0 / 360.0, s: 0.78, l: 0.45, a: 1.0 },  // lime
];

/// Default node dimensions
const NODE_WIDTH: f64 = 220.0;
const NODE_HEADER_HEIGHT: f64 = 32.0;
const NODE_COLUMN_HEIGHT: f64 = 24.0;
const NODE_INDEX_HEIGHT: f64 = 20.0;
const NODE_PADDING: f64 = 8.0;

pub struct DiagramService;

impl DiagramService {
    /// Generate diagram data from schema
    pub async fn generate_diagram(
        connection_id: &str,
        schemas: &[String],
        tables: Option<&[String]>,
        options: &DiagramOptions,
    ) -> Result<DiagramData> {
        // Fetch tables from specified schemas
        let mut all_tables: Vec<Table> = Vec::new();
        let mut schema_color_map: HashMap<String, Hsla> = HashMap::new();

        for (idx, schema) in schemas.iter().enumerate() {
            schema_color_map.insert(
                schema.clone(),
                SCHEMA_COLORS[idx % SCHEMA_COLORS.len()],
            );

            let schema_tables = SchemaService::list_tables(connection_id, schema).await?;
            all_tables.extend(schema_tables);
        }

        // Filter to requested tables if specified
        if let Some(table_filter) = tables {
            let filter_set: HashSet<&str> = table_filter.iter().map(|s| s.as_str()).collect();
            all_tables.retain(|t| {
                let full_name = format!("{}.{}", t.schema, t.name);
                filter_set.contains(full_name.as_str())
            });
        }

        // Build nodes
        let nodes = Self::build_nodes(&all_tables, options, &schema_color_map).await?;

        // Build edges from foreign keys
        let edges = Self::build_edges(&all_tables, options).await?;

        Ok(DiagramData { nodes, edges })
    }

    /// Build table nodes
    async fn build_nodes(
        tables: &[Table],
        options: &DiagramOptions,
        schema_colors: &HashMap<String, Hsla>,
    ) -> Result<Vec<TableNode>> {
        let mut nodes = Vec::new();

        for (idx, table) in tables.iter().enumerate() {
            // Get columns for this table
            let columns = SchemaService::list_columns(
                &table.connection_id,
                &table.schema,
                &table.name,
            ).await?;

            // Get indexes if requested
            let indexes = if options.show_indexes {
                SchemaService::list_indexes(
                    &table.connection_id,
                    &table.schema,
                    &table.name,
                ).await?
            } else {
                vec![]
            };

            // Get foreign keys to determine FK columns
            let fks = SchemaService::list_foreign_keys(
                &table.connection_id,
                &table.schema,
                &table.name,
            ).await?;

            let fk_columns: HashSet<String> = fks
                .iter()
                .flat_map(|fk| fk.columns.clone())
                .collect();

            // Get primary key columns
            let pk_columns: HashSet<String> = indexes
                .iter()
                .filter(|i| i.is_primary)
                .flat_map(|i| i.columns.clone())
                .collect();

            // Filter columns based on display option
            let diagram_columns: Vec<DiagramColumn> = columns
                .iter()
                .filter(|c| match options.column_display {
                    ColumnDisplay::All => true,
                    ColumnDisplay::PkFkOnly => {
                        pk_columns.contains(&c.name) || fk_columns.contains(&c.name)
                    }
                    ColumnDisplay::None => false,
                })
                .map(|c| {
                    // Find FK reference if exists
                    let fk_ref = fks
                        .iter()
                        .find(|fk| fk.columns.contains(&c.name))
                        .map(|fk| format!("{}.{}", fk.referenced_table, fk.referenced_columns[0]));

                    DiagramColumn {
                        name: c.name.clone(),
                        data_type: if options.show_data_types {
                            c.data_type.clone()
                        } else {
                            String::new()
                        },
                        nullable: c.nullable && options.show_nullable,
                        is_primary_key: pk_columns.contains(&c.name),
                        is_foreign_key: fk_columns.contains(&c.name),
                        fk_reference: fk_ref,
                    }
                })
                .collect();

            // Convert indexes
            let diagram_indexes: Vec<DiagramIndex> = indexes
                .iter()
                .map(|i| DiagramIndex {
                    name: i.name.clone(),
                    columns: i.columns.clone(),
                    is_unique: i.is_unique,
                    is_primary: i.is_primary,
                })
                .collect();

            // Calculate node size
            let column_count = diagram_columns.len();
            let index_count = if options.show_indexes { diagram_indexes.len() } else { 0 };
            let height = NODE_HEADER_HEIGHT
                + (column_count as f64 * NODE_COLUMN_HEIGHT)
                + (index_count as f64 * NODE_INDEX_HEIGHT)
                + NODE_PADDING * 2.0;

            // Initial grid position
            let position = Point {
                x: (idx % 5) as f64 * 280.0,
                y: (idx / 5) as f64 * (height + 50.0),
            };

            let color = if options.color_by_schema {
                *schema_colors
                    .get(&table.schema)
                    .unwrap_or(&Hsla { h: 0.0, s: 0.0, l: 0.42, a: 1.0 })
            } else {
                Hsla { h: 0.0, s: 0.0, l: 0.42, a: 1.0 }
            };

            nodes.push(TableNode {
                id: format!("{}.{}", table.schema, table.name),
                schema: table.schema.clone(),
                name: table.name.clone(),
                columns: diagram_columns,
                indexes: diagram_indexes,
                position,
                size: Size {
                    width: NODE_WIDTH,
                    height,
                },
                color,
                selected: false,
                hovered: false,
            });
        }

        Ok(nodes)
    }

    /// Build relationship edges from foreign keys
    async fn build_edges(
        tables: &[Table],
        _options: &DiagramOptions,
    ) -> Result<Vec<RelationshipEdge>> {
        let mut edges = Vec::new();
        let table_set: HashSet<String> = tables
            .iter()
            .map(|t| format!("{}.{}", t.schema, t.name))
            .collect();

        for table in tables {
            let fks = SchemaService::list_foreign_keys(
                &table.connection_id,
                &table.schema,
                &table.name,
            ).await?;

            for fk in fks {
                let target_id = format!("{}.{}", fk.referenced_schema, fk.referenced_table);

                // Only include edge if target table is in diagram
                if !table_set.contains(&target_id) {
                    continue;
                }

                let source_id = format!("{}.{}", table.schema, table.name);

                // Determine relationship type
                let rel_type = Self::determine_relationship_type(&fk, table).await?;

                edges.push(RelationshipEdge {
                    id: format!("{}_{}", source_id, fk.name),
                    source_node: source_id,
                    source_column: fk.columns.join(","),
                    target_node: target_id,
                    target_column: fk.referenced_columns.join(","),
                    label: fk.name.clone(),
                    relationship_type: rel_type,
                    selected: false,
                    hovered: false,
                });
            }
        }

        Ok(edges)
    }

    /// Determine relationship cardinality
    async fn determine_relationship_type(
        fk: &ForeignKey,
        table: &Table,
    ) -> Result<RelationshipType> {
        let indexes = SchemaService::list_indexes(
            &table.connection_id,
            &table.schema,
            &table.name,
        ).await?;

        // Check if FK columns have unique constraint (one-to-one)
        let fk_columns: HashSet<&str> = fk.columns.iter().map(|s| s.as_str()).collect();

        let is_unique = indexes.iter().any(|idx| {
            if !idx.is_unique {
                return false;
            }
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
        nodes: &mut [TableNode],
        edges: &[RelationshipEdge],
        algorithm: LayoutAlgorithm,
    ) {
        match algorithm {
            LayoutAlgorithm::Hierarchical => Self::hierarchical_layout(nodes, edges),
            LayoutAlgorithm::ForceDirected => Self::force_directed_layout(nodes, edges),
            LayoutAlgorithm::Circular => Self::circular_layout(nodes),
        }
    }

    /// Hierarchical layout (Sugiyama-style)
    fn hierarchical_layout(nodes: &mut [TableNode], edges: &[RelationshipEdge]) {
        if nodes.is_empty() {
            return;
        }

        // Build adjacency maps
        let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
        let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();

        for edge in edges {
            outgoing
                .entry(edge.source_node.clone())
                .or_default()
                .push(edge.target_node.clone());
            incoming
                .entry(edge.target_node.clone())
                .or_default()
                .push(edge.source_node.clone());
        }

        // Assign layers (topological sort)
        let mut layers: HashMap<String, usize> = HashMap::new();
        let mut remaining: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();

        let mut current_layer = 0;
        while !remaining.is_empty() {
            let roots: Vec<String> = remaining
                .iter()
                .filter(|n| {
                    incoming
                        .get(*n)
                        .map(|i| i.iter().all(|s| layers.contains_key(s)))
                        .unwrap_or(true)
                })
                .cloned()
                .collect();

            if roots.is_empty() {
                // Cycle detected - assign remaining to current layer
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
        let layer_height = 350.0;
        let node_spacing = 280.0;

        for (layer, layer_nodes) in layer_groups.iter_mut() {
            let count = layer_nodes.len();
            let total_width = count as f64 * node_spacing - 30.0;
            let start_x = -total_width / 2.0;

            for (idx, node) in layer_nodes.iter_mut().enumerate() {
                node.position.x = start_x + idx as f64 * node_spacing;
                node.position.y = *layer as f64 * layer_height;
            }
        }
    }

    /// Force-directed layout (Fruchterman-Reingold)
    fn force_directed_layout(nodes: &mut [TableNode], edges: &[RelationshipEdge]) {
        if nodes.is_empty() {
            return;
        }

        let iterations = 150;
        let k = 250.0; // Ideal edge length
        let temp_start = 150.0;

        // Build edge map
        let edge_pairs: Vec<(String, String)> = edges
            .iter()
            .map(|e| (e.source_node.clone(), e.target_node.clone()))
            .collect();

        for iter in 0..iterations {
            let temp = temp_start * (1.0 - iter as f64 / iterations as f64);

            // Calculate repulsive forces between all node pairs
            let mut forces: HashMap<String, (f64, f64)> = HashMap::new();

            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    let dx = nodes[j].position.x - nodes[i].position.x;
                    let dy = nodes[j].position.y - nodes[i].position.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);

                    // Repulsive force (Coulomb's law)
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

            // Calculate attractive forces along edges (Hooke's law)
            for (source, target) in &edge_pairs {
                let source_idx = nodes.iter().position(|n| &n.id == source);
                let target_idx = nodes.iter().position(|n| &n.id == target);

                if let (Some(si), Some(ti)) = (source_idx, target_idx) {
                    let dx = nodes[ti].position.x - nodes[si].position.x;
                    let dy = nodes[ti].position.y - nodes[si].position.y;
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

            // Apply forces with temperature limiting
            for node in nodes.iter_mut() {
                if let Some((fx, fy)) = forces.get(&node.id) {
                    let mag = (fx * fx + fy * fy).sqrt().max(0.001);
                    let limited_mag = mag.min(temp);

                    node.position.x += fx / mag * limited_mag;
                    node.position.y += fy / mag * limited_mag;
                }
            }
        }
    }

    /// Circular layout
    fn circular_layout(nodes: &mut [TableNode]) {
        let count = nodes.len();
        if count == 0 {
            return;
        }

        let radius = (count as f64 * 120.0).max(400.0);
        let angle_step = 2.0 * std::f64::consts::PI / count as f64;

        for (idx, node) in nodes.iter_mut().enumerate() {
            let angle = idx as f64 * angle_step - std::f64::consts::PI / 2.0;
            node.position.x = radius * angle.cos();
            node.position.y = radius * angle.sin();
        }
    }

    /// Save diagram configuration to storage
    pub async fn save_diagram(config: &DiagramConfig) -> Result<()> {
        let json = serde_json::to_string(config)?;
        StorageService::set(&format!("diagram:{}", config.id.0), &json).await
    }

    /// Load diagram configuration from storage
    pub async fn load_diagram(diagram_id: &DiagramId) -> Result<Option<DiagramConfig>> {
        let key = format!("diagram:{}", diagram_id.0);
        match StorageService::get(&key).await? {
            Some(json) => {
                let config: DiagramConfig = serde_json::from_str(&json)?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// List diagrams for a connection
    pub async fn list_diagrams(connection_id: &str) -> Result<Vec<DiagramConfig>> {
        let prefix = "diagram:";
        let all = StorageService::list_by_prefix(prefix).await?;

        let diagrams: Vec<DiagramConfig> = all
            .into_iter()
            .filter_map(|(_, json)| serde_json::from_str(&json).ok())
            .filter(|d: &DiagramConfig| d.connection_id == connection_id)
            .collect();

        Ok(diagrams)
    }

    /// Delete a diagram
    pub async fn delete_diagram(diagram_id: &DiagramId) -> Result<()> {
        StorageService::delete(&format!("diagram:{}", diagram_id.0)).await
    }

    /// Export diagram to image
    pub fn export_to_png(
        diagram: &DiagramData,
        viewport: &Viewport,
        options: &ExportOptions,
    ) -> Result<Vec<u8>> {
        // Calculate bounds with padding
        let bounds = Self::calculate_export_bounds(diagram, options.padding);

        let width = ((bounds.max.x - bounds.min.x) * options.scale) as u32;
        let height = ((bounds.max.y - bounds.min.y) * options.scale) as u32;

        // Create image buffer using tiny-skia or similar
        // This would require integration with a software rasterizer
        // For now, return placeholder
        Err(TuskError::NotImplemented("PNG export requires GPU or software rasterizer".into()))
    }

    /// Export diagram to SVG
    pub fn export_to_svg(
        diagram: &DiagramData,
        options: &ExportOptions,
    ) -> Result<String> {
        let bounds = Self::calculate_export_bounds(diagram, options.padding);
        let width = bounds.max.x - bounds.min.x;
        let height = bounds.max.y - bounds.min.y;

        let mut svg = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}" width="{}" height="{}">"#,
            bounds.min.x, bounds.min.y, width, height,
            width * options.scale, height * options.scale
        );

        // Add background
        if options.background {
            svg.push_str(&format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="#ffffff"/>"#,
                bounds.min.x, bounds.min.y, width, height
            ));
        }

        // Render edges
        for edge in &diagram.edges {
            let source = diagram.nodes.iter().find(|n| n.id == edge.source_node);
            let target = diagram.nodes.iter().find(|n| n.id == edge.target_node);

            if let (Some(src), Some(tgt)) = (source, target) {
                let start_x = src.position.x + src.size.width;
                let start_y = src.position.y + src.size.height / 2.0;
                let end_x = tgt.position.x;
                let end_y = tgt.position.y + tgt.size.height / 2.0;

                // Bezier curve
                let ctrl_x = (start_x + end_x) / 2.0;
                svg.push_str(&format!(
                    r#"<path d="M {} {} C {} {} {} {} {} {}" stroke="#6b7280" stroke-width="2" fill="none" marker-end="url(#arrow)"/>"#,
                    start_x, start_y, ctrl_x, start_y, ctrl_x, end_y, end_x, end_y
                ));
            }
        }

        // Render nodes
        for node in &diagram.nodes {
            let color_hex = Self::hsla_to_hex(&node.color);

            // Node background
            svg.push_str(&format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" rx="6" fill="#ffffff" stroke="{}" stroke-width="2"/>"#,
                node.position.x, node.position.y, node.size.width, node.size.height, color_hex
            ));

            // Header
            svg.push_str(&format!(
                r#"<rect x="{}" y="{}" width="{}" height="32" rx="6" fill="{}"/>"#,
                node.position.x, node.position.y, node.size.width, color_hex
            ));

            // Table name
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" fill="#ffffff" font-size="12" font-weight="600">{}.{}</text>"#,
                node.position.x + 12.0,
                node.position.y + 20.0,
                node.schema, node.name
            ));

            // Columns
            let mut y_offset = 44.0;
            for col in &node.columns {
                let mut name = col.name.clone();
                if col.is_primary_key {
                    name = format!("🔑 {}", name);
                } else if col.is_foreign_key {
                    name = format!("🔗 {}", name);
                }

                svg.push_str(&format!(
                    r#"<text x="{}" y="{}" fill="#374151" font-size="11">{}</text>"#,
                    node.position.x + 12.0,
                    node.position.y + y_offset,
                    name
                ));

                if !col.data_type.is_empty() {
                    svg.push_str(&format!(
                        r#"<text x="{}" y="{}" fill="#9ca3af" font-size="10" text-anchor="end">{}</text>"#,
                        node.position.x + node.size.width - 12.0,
                        node.position.y + y_offset,
                        col.data_type
                    ));
                }

                y_offset += 24.0;
            }
        }

        // Arrow marker definition
        svg.push_str(r#"
<defs>
    <marker id="arrow" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto">
        <path d="M0,0 L0,6 L9,3 z" fill="#6b7280"/>
    </marker>
</defs>"#);

        svg.push_str("</svg>");
        Ok(svg)
    }

    fn calculate_export_bounds(diagram: &DiagramData, padding: u32) -> BoundingBox {
        let pad = padding as f64;
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for node in &diagram.nodes {
            min_x = min_x.min(node.position.x);
            min_y = min_y.min(node.position.y);
            max_x = max_x.max(node.position.x + node.size.width);
            max_y = max_y.max(node.position.y + node.size.height);
        }

        BoundingBox {
            min: Point::new(min_x - pad, min_y - pad),
            max: Point::new(max_x + pad, max_y + pad),
        }
    }

    fn hsla_to_hex(hsla: &Hsla) -> String {
        // Convert HSLA to RGB
        let c = (1.0 - (2.0 * hsla.l - 1.0).abs()) * hsla.s;
        let x = c * (1.0 - ((hsla.h * 6.0) % 2.0 - 1.0).abs());
        let m = hsla.l - c / 2.0;

        let (r, g, b) = match (hsla.h * 6.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        format!(
            "#{:02x}{:02x}{:02x}",
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8
        )
    }
}
```

### 26.4 Canvas Rendering

**File: `src/ui/diagram/canvas.rs`**

```rust
use crate::models::diagram::*;
use crate::state::DiagramState;
use gpui::*;

/// ER Diagram canvas component with GPU-accelerated rendering
pub struct DiagramCanvas {
    conn_id: String,
    canvas_size: Size<Pixels>,
    last_mouse_pos: Option<Point<Pixels>>,
    panning: bool,
    focus_handle: FocusHandle,
}

impl DiagramCanvas {
    pub fn new(conn_id: String, cx: &mut Context<Self>) -> Self {
        Self {
            conn_id,
            canvas_size: Size::default(),
            last_mouse_pos: None,
            panning: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let state = cx.global::<DiagramState>();
        let viewport = state.viewport();

        let world_pos = viewport.screen_to_world(
            event.position.x.0 as f64,
            event.position.y.0 as f64,
        );

        // Check for node hit
        if let Some(node_id) = state.node_at_position(&world_pos) {
            let shift = event.modifiers.shift;
            state.select_node(&node_id, shift);
            state.start_drag(&node_id, world_pos);
        } else if let Some(_edge_id) = state.edge_at_position(&world_pos) {
            // Edge selection
        } else {
            // Start panning
            self.panning = true;
            state.clear_selection();
        }

        self.last_mouse_pos = Some(event.position);
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let state = cx.global::<DiagramState>();

        if let Some(last_pos) = self.last_mouse_pos {
            if self.panning {
                let dx = event.position.x.0 - last_pos.x.0;
                let dy = event.position.y.0 - last_pos.y.0;
                state.pan(dx as f64, dy as f64);
            } else if state.is_dragging() {
                let viewport = state.viewport();
                let world_pos = viewport.screen_to_world(
                    event.position.x.0 as f64,
                    event.position.y.0 as f64,
                );
                state.update_drag(world_pos);
            }
        }

        // Update hover state
        let viewport = state.viewport();
        let world_pos = viewport.screen_to_world(
            event.position.x.0 as f64,
            event.position.y.0 as f64,
        );

        let hovered_node = state.node_at_position(&world_pos);
        state.set_hover_node(hovered_node.as_deref());

        self.last_mouse_pos = Some(event.position);
        cx.notify();
    }

    fn on_mouse_up(&mut self, _event: &MouseUpEvent, cx: &mut Context<Self>) {
        let state = cx.global::<DiagramState>();
        state.end_drag();
        self.panning = false;
        cx.notify();
    }

    fn on_scroll(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let state = cx.global::<DiagramState>();

        let zoom_factor = if event.delta.y.0 > 0.0 { 0.9 } else { 1.1 };
        let center = crate::models::diagram::Point::new(
            event.position.x.0 as f64,
            event.position.y.0 as f64,
        );

        state.zoom(zoom_factor, center);
        cx.notify();
    }

    fn render_grid(&self, viewport: &Viewport, cx: &Context<Self>) -> impl IntoElement {
        let grid_size = 20.0 * viewport.zoom;
        let offset_x = viewport.x % grid_size;
        let offset_y = viewport.y % grid_size;

        let width = self.canvas_size.width.0 as f64;
        let height = self.canvas_size.height.0 as f64;

        // Generate grid lines
        let mut lines = Vec::new();

        let mut x = offset_x;
        while x < width {
            lines.push(
                div()
                    .absolute()
                    .left(px(x as f32))
                    .top(px(0.0))
                    .w(px(1.0))
                    .h_full()
                    .bg(rgb(0xf0f0f0))
            );
            x += grid_size;
        }

        let mut y = offset_y;
        while y < height {
            lines.push(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(y as f32))
                    .w_full()
                    .h(px(1.0))
                    .bg(rgb(0xf0f0f0))
            );
            y += grid_size;
        }

        div()
            .absolute()
            .inset_0()
            .overflow_hidden()
            .children(lines)
    }

    fn render_edges(&self, diagram: &DiagramData, viewport: &Viewport) -> impl IntoElement {
        let edges: Vec<_> = diagram.edges.iter().map(|edge| {
            let source = diagram.nodes.iter().find(|n| n.id == edge.source_node);
            let target = diagram.nodes.iter().find(|n| n.id == edge.target_node);

            if let (Some(src), Some(tgt)) = (source, target) {
                let start = viewport.world_to_screen(&crate::models::diagram::Point::new(
                    src.position.x + src.size.width,
                    src.position.y + src.size.height / 2.0,
                ));
                let end = viewport.world_to_screen(&crate::models::diagram::Point::new(
                    tgt.position.x,
                    tgt.position.y + tgt.size.height / 2.0,
                ));

                // Use SVG for bezier curves
                let ctrl_x = (start.x + end.x) / 2.0;
                let path = format!(
                    "M {} {} C {} {} {} {} {} {}",
                    start.x, start.y,
                    ctrl_x, start.y,
                    ctrl_x, end.y,
                    end.x, end.y
                );

                let color = if edge.selected {
                    rgb(0x3b82f6)
                } else if edge.hovered {
                    rgb(0x6b7280)
                } else {
                    rgb(0x9ca3af)
                };

                let stroke_width = if edge.selected || edge.hovered { 3.0 } else { 2.0 };

                Some(
                    svg()
                        .absolute()
                        .inset_0()
                        .overflow_visible()
                        .child(
                            svg_path()
                                .d(path)
                                .stroke(color)
                                .stroke_width(stroke_width)
                                .fill("none")
                        )
                )
            } else {
                None
            }
        }).flatten().collect();

        div().absolute().inset_0().children(edges)
    }

    fn render_nodes(&self, diagram: &DiagramData, viewport: &Viewport, cx: &Context<Self>) -> impl IntoElement {
        let state = cx.global::<DiagramState>();
        let options = state.options();

        let nodes: Vec<_> = diagram.nodes.iter().map(|node| {
            let screen_pos = viewport.world_to_screen(&node.position);
            let scaled_width = node.size.width * viewport.zoom;
            let scaled_height = node.size.height * viewport.zoom;

            TableNodeView::new(
                node.clone(),
                screen_pos,
                Size {
                    width: scaled_width,
                    height: scaled_height,
                },
                options.clone(),
            )
        }).collect();

        div().absolute().inset_0().children(nodes)
    }

    fn render_minimap(&self, diagram: &DiagramData, viewport: &Viewport) -> impl IntoElement {
        let minimap_width = 150.0;
        let minimap_height = 100.0;

        // Calculate bounds
        let bounds = if diagram.nodes.is_empty() {
            BoundingBox::new(0.0, 0.0, 1000.0, 1000.0)
        } else {
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;

            for node in &diagram.nodes {
                min_x = min_x.min(node.position.x);
                min_y = min_y.min(node.position.y);
                max_x = max_x.max(node.position.x + node.size.width);
                max_y = max_y.max(node.position.y + node.size.height);
            }

            BoundingBox {
                min: crate::models::diagram::Point::new(min_x - 50.0, min_y - 50.0),
                max: crate::models::diagram::Point::new(max_x + 50.0, max_y + 50.0),
            }
        };

        let scale_x = minimap_width / (bounds.max.x - bounds.min.x);
        let scale_y = minimap_height / (bounds.max.y - bounds.min.y);
        let scale = scale_x.min(scale_y);

        // Render mini nodes
        let mini_nodes: Vec<_> = diagram.nodes.iter().map(|node| {
            let x = (node.position.x - bounds.min.x) * scale;
            let y = (node.position.y - bounds.min.y) * scale;
            let w = node.size.width * scale;
            let h = node.size.height * scale;

            div()
                .absolute()
                .left(px(x as f32))
                .top(px(y as f32))
                .w(px(w as f32))
                .h(px(h as f32))
                .bg(node.color)
                .rounded(px(1.0))
        }).collect();

        // Viewport indicator
        let vp_x = (-viewport.x / viewport.zoom - bounds.min.x) * scale;
        let vp_y = (-viewport.y / viewport.zoom - bounds.min.y) * scale;
        let vp_w = (self.canvas_size.width.0 as f64 / viewport.zoom) * scale;
        let vp_h = (self.canvas_size.height.0 as f64 / viewport.zoom) * scale;

        div()
            .absolute()
            .bottom(px(16.0))
            .right(px(16.0))
            .w(px(minimap_width as f32))
            .h(px(minimap_height as f32))
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .rounded_lg()
            .shadow_md()
            .overflow_hidden()
            .children(mini_nodes)
            .child(
                div()
                    .absolute()
                    .left(px(vp_x as f32))
                    .top(px(vp_y as f32))
                    .w(px(vp_w as f32))
                    .h(px(vp_h as f32))
                    .border_2()
                    .border_color(rgb(0x3b82f6))
                    .rounded(px(2.0))
            )
    }
}

impl Render for DiagramCanvas {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<DiagramState>();
        let viewport = state.viewport();
        let diagram = state.current_diagram();
        let options = state.options();

        div()
            .id("diagram-canvas")
            .flex_1()
            .relative()
            .bg(rgb(0xfafafa))
            .overflow_hidden()
            .track_focus(&self.focus_handle)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .child(
                // Grid background
                if options.show_grid {
                    self.render_grid(&viewport, cx).into_any_element()
                } else {
                    div().into_any_element()
                }
            )
            .when_some(diagram.as_ref(), |this, diagram| {
                this
                    .child(self.render_edges(diagram, &viewport))
                    .child(self.render_nodes(diagram, &viewport, cx))
            })
            .when(options.show_minimap && diagram.is_some(), |this| {
                this.child(self.render_minimap(diagram.as_ref().unwrap(), &viewport))
            })
    }
}

impl FocusableView for DiagramCanvas {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
```

### 26.5 Table Node View

**File: `src/ui/diagram/table_node.rs`**

```rust
use crate::models::diagram::*;
use gpui::*;

/// Visual representation of a table node
pub struct TableNodeView {
    node: TableNode,
    screen_pos: crate::models::diagram::Point,
    size: crate::models::diagram::Size,
    options: DiagramOptions,
}

impl TableNodeView {
    pub fn new(
        node: TableNode,
        screen_pos: crate::models::diagram::Point,
        size: crate::models::diagram::Size,
        options: DiagramOptions,
    ) -> Self {
        Self {
            node,
            screen_pos,
            size,
            options,
        }
    }

    fn render_header(&self) -> impl IntoElement {
        div()
            .w_full()
            .h(px(32.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .bg(self.node.color)
            .rounded_t_md()
            .child(
                div()
                    .text_color(rgb(0xffffff))
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("{}.{}", self.node.schema, self.node.name))
            )
    }

    fn render_columns(&self) -> impl IntoElement {
        let columns: Vec<_> = self.node.columns.iter().map(|col| {
            div()
                .w_full()
                .h(px(24.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .hover(|s| s.bg(rgb(0xf9fafb)))
                .child(
                    // Icons
                    div()
                        .flex()
                        .gap(px(2.0))
                        .w(px(28.0))
                        .flex_shrink_0()
                        .when(col.is_primary_key, |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xf59e0b))
                                    .child("🔑")
                            )
                        })
                        .when(col.is_foreign_key, |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x3b82f6))
                                    .child("🔗")
                            )
                        })
                )
                .child(
                    // Column name
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_xs()
                        .text_color(rgb(0x374151))
                        .when(col.is_primary_key || col.is_foreign_key, |this| {
                            this.font_weight(FontWeight::MEDIUM)
                        })
                        .child(col.name.clone())
                )
                .when(self.options.show_data_types && !col.data_type.is_empty(), |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x9ca3af))
                            .font_family("monospace")
                            .child(col.data_type.clone())
                    )
                })
                .when(self.options.show_nullable && col.nullable, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xd1d5db))
                            .italic()
                            .child("?")
                    )
                })
        }).collect();

        div()
            .w_full()
            .py(px(4.0))
            .border_b_1()
            .border_color(rgb(0xe5e7eb))
            .children(columns)
    }

    fn render_indexes(&self) -> impl IntoElement {
        if !self.options.show_indexes || self.node.indexes.is_empty() {
            return div().into_any_element();
        }

        let indexes: Vec<_> = self.node.indexes.iter().map(|idx| {
            let color = if idx.is_unique {
                rgb(0x10b981) // green
            } else {
                rgb(0x9ca3af) // gray
            };

            div()
                .w_full()
                .h(px(20.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_xs()
                        .text_color(color)
                        .child("#")
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x9ca3af))
                        .child(idx.name.clone())
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0xd1d5db))
                        .child(format!("({})", idx.columns.join(", ")))
                )
        }).collect();

        div()
            .w_full()
            .py(px(4.0))
            .children(indexes)
            .into_any_element()
    }
}

impl IntoElement for TableNodeView {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let border_color = if self.node.selected {
            self.node.color
        } else {
            Hsla {
                h: self.node.color.h,
                s: self.node.color.s * 0.5,
                l: self.node.color.l,
                a: 1.0,
            }
        };

        let shadow = if self.node.selected {
            Some(BoxShadow {
                color: Hsla {
                    h: self.node.color.h,
                    s: self.node.color.s,
                    l: self.node.color.l,
                    a: 0.3,
                },
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(0.0),
                spread_radius: px(3.0),
            })
        } else if self.node.hovered {
            Some(BoxShadow {
                color: Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.15 },
                offset: point(px(0.0), px(4.0)),
                blur_radius: px(12.0),
                spread_radius: px(0.0),
            })
        } else {
            Some(BoxShadow {
                color: Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.1 },
                offset: point(px(0.0), px(2.0)),
                blur_radius: px(8.0),
                spread_radius: px(0.0),
            })
        };

        div()
            .id(SharedString::from(self.node.id.clone()))
            .absolute()
            .left(px(self.screen_pos.x as f32))
            .top(px(self.screen_pos.y as f32))
            .w(px(self.size.width as f32))
            .bg(rgb(0xffffff))
            .border_2()
            .border_color(border_color)
            .rounded_lg()
            .overflow_hidden()
            .when_some(shadow, |this, shadow| {
                this.shadow(smallvec::smallvec![shadow])
            })
            .cursor_move()
            .child(self.render_header())
            .child(self.render_columns())
            .child(self.render_indexes())
    }
}
```

### 26.6 Diagram Toolbar

**File: `src/ui/diagram/toolbar.rs`**

```rust
use crate::models::diagram::*;
use crate::services::diagram::DiagramService;
use crate::state::DiagramState;
use crate::ui::components::{Button, Icon, IconName, Select, SelectOption};
use gpui::*;

pub struct DiagramToolbar {
    conn_id: String,
    selected_layout: LayoutAlgorithm,
}

pub enum DiagramToolbarEvent {
    LayoutChanged(LayoutAlgorithm),
    FitToView,
    ZoomIn,
    ZoomOut,
    ExportPng,
    ExportSvg,
    SaveDiagram,
}

impl EventEmitter<DiagramToolbarEvent> for DiagramToolbar {}

impl DiagramToolbar {
    pub fn new(conn_id: String) -> Self {
        Self {
            conn_id,
            selected_layout: LayoutAlgorithm::Hierarchical,
        }
    }

    fn on_layout_change(&mut self, layout: LayoutAlgorithm, cx: &mut Context<Self>) {
        self.selected_layout = layout;

        let state = cx.global::<DiagramState>();
        if let Some(mut diagram) = state.current_diagram() {
            DiagramService::apply_layout(
                &mut diagram.nodes,
                &diagram.edges,
                layout,
            );
            state.set_diagram(diagram);
        }

        cx.emit(DiagramToolbarEvent::LayoutChanged(layout));
        cx.notify();
    }

    fn on_fit_view(&mut self, cx: &mut Context<Self>) {
        cx.emit(DiagramToolbarEvent::FitToView);
    }

    fn on_zoom_in(&mut self, cx: &mut Context<Self>) {
        cx.emit(DiagramToolbarEvent::ZoomIn);
    }

    fn on_zoom_out(&mut self, cx: &mut Context<Self>) {
        cx.emit(DiagramToolbarEvent::ZoomOut);
    }

    fn on_export_png(&mut self, cx: &mut Context<Self>) {
        cx.emit(DiagramToolbarEvent::ExportPng);
    }

    fn on_export_svg(&mut self, cx: &mut Context<Self>) {
        let state = cx.global::<DiagramState>();

        if let Some(diagram) = state.current_diagram() {
            let options = ExportOptions {
                format: ExportFormat::Svg,
                scale: 1.0,
                background: true,
                padding: 50,
            };

            match DiagramService::export_to_svg(&diagram, &options) {
                Ok(svg) => {
                    // Save to file using file dialog
                    cx.spawn(|_, _| async move {
                        if let Some(path) = rfd::AsyncFileDialog::new()
                            .set_title("Save SVG")
                            .add_filter("SVG", &["svg"])
                            .save_file()
                            .await
                        {
                            let _ = std::fs::write(path.path(), svg);
                        }
                    }).detach();
                }
                Err(e) => {
                    state.set_error(Some(format!("Export failed: {}", e)));
                }
            }
        }

        cx.emit(DiagramToolbarEvent::ExportSvg);
    }

    fn on_save(&mut self, cx: &mut Context<Self>) {
        let state = cx.global::<DiagramState>();

        if let (Some(diagram), Some(config)) = (state.current_diagram(), state.config()) {
            let mut updated_config = config.clone();

            // Update node positions in config
            for node in &diagram.nodes {
                updated_config.layout.node_positions.insert(
                    node.id.clone(),
                    node.position,
                );
            }

            updated_config.layout.viewport = state.viewport();
            updated_config.updated_at = chrono::Utc::now().timestamp();

            cx.spawn(|_, _| async move {
                let _ = DiagramService::save_diagram(&updated_config).await;
            }).detach();
        }

        cx.emit(DiagramToolbarEvent::SaveDiagram);
    }
}

impl Render for DiagramToolbar {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let layout_options: Vec<SelectOption<LayoutAlgorithm>> = LayoutAlgorithm::all()
            .iter()
            .map(|&algo| SelectOption {
                value: algo,
                label: algo.label().into(),
            })
            .collect();

        div()
            .w_full()
            .h(px(48.0))
            .px(px(16.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .bg(rgb(0xffffff))
            .border_b_1()
            .border_color(rgb(0xe5e7eb))
            // Layout section
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x6b7280))
                            .child("Layout:")
                    )
                    .child(
                        Select::new(
                            "layout-select",
                            layout_options,
                            self.selected_layout,
                            cx.listener(|this, layout, cx| {
                                this.on_layout_change(layout, cx);
                            }),
                        )
                    )
            )
            // Divider
            .child(
                div()
                    .w(px(1.0))
                    .h(px(24.0))
                    .bg(rgb(0xe5e7eb))
                    .mx(px(8.0))
            )
            // Zoom controls
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        Button::ghost()
                            .icon(IconName::Maximize)
                            .tooltip("Fit to View")
                            .on_click(cx.listener(|this, _, cx| this.on_fit_view(cx)))
                    )
                    .child(
                        Button::ghost()
                            .icon(IconName::ZoomIn)
                            .tooltip("Zoom In")
                            .on_click(cx.listener(|this, _, cx| this.on_zoom_in(cx)))
                    )
                    .child(
                        Button::ghost()
                            .icon(IconName::ZoomOut)
                            .tooltip("Zoom Out")
                            .on_click(cx.listener(|this, _, cx| this.on_zoom_out(cx)))
                    )
            )
            // Divider
            .child(
                div()
                    .w(px(1.0))
                    .h(px(24.0))
                    .bg(rgb(0xe5e7eb))
                    .mx(px(8.0))
            )
            // Export controls
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        Button::ghost()
                            .icon(IconName::Download)
                            .label("PNG")
                            .tooltip("Export as PNG")
                            .on_click(cx.listener(|this, _, cx| this.on_export_png(cx)))
                    )
                    .child(
                        Button::ghost()
                            .icon(IconName::Download)
                            .label("SVG")
                            .tooltip("Export as SVG")
                            .on_click(cx.listener(|this, _, cx| this.on_export_svg(cx)))
                    )
            )
            // Spacer
            .child(div().flex_1())
            // Save button
            .child(
                Button::primary()
                    .icon(IconName::Save)
                    .label("Save Diagram")
                    .on_click(cx.listener(|this, _, cx| this.on_save(cx)))
            )
    }
}
```

### 26.7 Diagram Options Panel

**File: `src/ui/diagram/options_panel.rs`**

```rust
use crate::models::diagram::*;
use crate::state::DiagramState;
use crate::ui::components::{Checkbox, Icon, IconName, Select, SelectOption};
use gpui::*;

pub struct DiagramOptionsPanel {
    expanded: bool,
}

impl DiagramOptionsPanel {
    pub fn new() -> Self {
        Self { expanded: false }
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
        cx.notify();
    }

    fn update_option<F>(&mut self, updater: F, cx: &mut Context<Self>)
    where
        F: FnOnce(&mut DiagramOptions),
    {
        let state = cx.global::<DiagramState>();
        let mut options = state.options();
        updater(&mut options);
        state.set_options(options);
        cx.notify();
    }
}

impl Render for DiagramOptionsPanel {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<DiagramState>();
        let options = state.options();

        let column_display_options = vec![
            SelectOption { value: ColumnDisplay::All, label: "All Columns".into() },
            SelectOption { value: ColumnDisplay::PkFkOnly, label: "PK/FK Only".into() },
            SelectOption { value: ColumnDisplay::None, label: "No Columns".into() },
        ];

        div()
            .absolute()
            .top(px(8.0))
            .right(px(8.0))
            .w(px(220.0))
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .rounded_lg()
            .shadow_md()
            .z_index(10)
            .overflow_hidden()
            // Toggle header
            .child(
                div()
                    .w_full()
                    .px(px(12.0))
                    .py(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0xf9fafb)))
                    .on_click(cx.listener(|this, _, cx| this.toggle(cx)))
                    .child(Icon::new(IconName::Settings).size(px(16.0)))
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(rgb(0x374151))
                            .child("Display Options")
                    )
                    .child(
                        Icon::new(if self.expanded {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size(px(16.0))
                    )
            )
            // Options content
            .when(self.expanded, |this| {
                this.child(
                    div()
                        .w_full()
                        .px(px(12.0))
                        .py(px(12.0))
                        .border_t_1()
                        .border_color(rgb(0xe5e7eb))
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        // Column display
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x6b7280))
                                        .child("Show Columns")
                                )
                                .child(
                                    Select::new(
                                        "column-display",
                                        column_display_options,
                                        options.column_display,
                                        cx.listener(|this, value, cx| {
                                            this.update_option(|opts| opts.column_display = value, cx);
                                        }),
                                    )
                                )
                        )
                        // Checkboxes
                        .child(
                            Checkbox::new(
                                "show-data-types",
                                options.show_data_types,
                                "Show data types",
                                cx.listener(|this, checked, cx| {
                                    this.update_option(|opts| opts.show_data_types = checked, cx);
                                }),
                            )
                        )
                        .child(
                            Checkbox::new(
                                "show-nullable",
                                options.show_nullable,
                                "Show nullable indicators",
                                cx.listener(|this, checked, cx| {
                                    this.update_option(|opts| opts.show_nullable = checked, cx);
                                }),
                            )
                        )
                        .child(
                            Checkbox::new(
                                "show-indexes",
                                options.show_indexes,
                                "Show indexes",
                                cx.listener(|this, checked, cx| {
                                    this.update_option(|opts| opts.show_indexes = checked, cx);
                                }),
                            )
                        )
                        .child(
                            Checkbox::new(
                                "color-by-schema",
                                options.color_by_schema,
                                "Color by schema",
                                cx.listener(|this, checked, cx| {
                                    this.update_option(|opts| opts.color_by_schema = checked, cx);
                                }),
                            )
                        )
                        .child(
                            Checkbox::new(
                                "snap-to-grid",
                                options.snap_to_grid,
                                "Snap to grid",
                                cx.listener(|this, checked, cx| {
                                    this.update_option(|opts| opts.snap_to_grid = checked, cx);
                                }),
                            )
                        )
                        .child(
                            Checkbox::new(
                                "show-minimap",
                                options.show_minimap,
                                "Show minimap",
                                cx.listener(|this, checked, cx| {
                                    this.update_option(|opts| opts.show_minimap = checked, cx);
                                }),
                            )
                        )
                        .child(
                            Checkbox::new(
                                "show-grid",
                                options.show_grid,
                                "Show grid",
                                cx.listener(|this, checked, cx| {
                                    this.update_option(|opts| opts.show_grid = checked, cx);
                                }),
                            )
                        )
                )
            })
    }
}
```

### 26.8 Diagram Generator Dialog

**File: `src/ui/diagram/generator_dialog.rs`**

```rust
use crate::models::diagram::*;
use crate::services::diagram::DiagramService;
use crate::services::schema::SchemaService;
use crate::state::DiagramState;
use crate::ui::components::{Button, Checkbox, Modal};
use gpui::*;
use std::collections::HashSet;

pub struct DiagramGeneratorDialog {
    conn_id: String,
    schemas: Vec<String>,
    tables: Vec<(String, String)>, // (schema, table)
    selected_schemas: HashSet<String>,
    selected_tables: HashSet<String>,
    select_all_tables: bool,
    options: DiagramOptions,
    loading: bool,
    focus_handle: FocusHandle,
}

pub enum DiagramGeneratorEvent {
    Cancel,
    Generated,
}

impl EventEmitter<DiagramGeneratorEvent> for DiagramGeneratorDialog {}

impl DiagramGeneratorDialog {
    pub fn new(conn_id: String, cx: &mut Context<Self>) -> Self {
        let mut dialog = Self {
            conn_id: conn_id.clone(),
            schemas: Vec::new(),
            tables: Vec::new(),
            selected_schemas: HashSet::from(["public".to_string()]),
            selected_tables: HashSet::new(),
            select_all_tables: true,
            options: DiagramOptions::default(),
            loading: false,
            focus_handle: cx.focus_handle(),
        };

        // Load schemas
        cx.spawn(|this, mut cx| async move {
            if let Ok(schemas) = SchemaService::list_schemas(&conn_id).await {
                let _ = this.update(&mut cx, |this, cx| {
                    this.schemas = schemas.into_iter().map(|s| s.name).collect();
                    cx.notify();
                });
            }
        }).detach();

        dialog
    }

    fn load_tables(&mut self, cx: &mut Context<Self>) {
        let conn_id = self.conn_id.clone();
        let schemas: Vec<String> = self.selected_schemas.iter().cloned().collect();

        cx.spawn(|this, mut cx| async move {
            let mut all_tables = Vec::new();

            for schema in &schemas {
                if let Ok(tables) = SchemaService::list_tables(&conn_id, schema).await {
                    for table in tables {
                        all_tables.push((table.schema, table.name));
                    }
                }
            }

            let _ = this.update(&mut cx, |this, cx| {
                this.tables = all_tables;
                cx.notify();
            });
        }).detach();
    }

    fn toggle_schema(&mut self, schema: String, cx: &mut Context<Self>) {
        if self.selected_schemas.contains(&schema) {
            self.selected_schemas.remove(&schema);
        } else {
            self.selected_schemas.insert(schema);
        }
        self.load_tables(cx);
        cx.notify();
    }

    fn toggle_table(&mut self, table_id: String, cx: &mut Context<Self>) {
        if self.selected_tables.contains(&table_id) {
            self.selected_tables.remove(&table_id);
        } else {
            self.selected_tables.insert(table_id);
        }
        cx.notify();
    }

    fn generate(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        cx.notify();

        let conn_id = self.conn_id.clone();
        let schemas: Vec<String> = self.selected_schemas.iter().cloned().collect();
        let tables = if self.select_all_tables {
            None
        } else {
            Some(self.selected_tables.iter().cloned().collect::<Vec<_>>())
        };
        let options = self.options.clone();

        cx.spawn(|this, mut cx| async move {
            match DiagramService::generate_diagram(
                &conn_id,
                &schemas,
                tables.as_deref(),
                &options,
            ).await {
                Ok(mut diagram) => {
                    // Apply initial layout
                    DiagramService::apply_layout(
                        &mut diagram.nodes,
                        &diagram.edges,
                        LayoutAlgorithm::Hierarchical,
                    );

                    let _ = this.update(&mut cx, |this, cx| {
                        let state = cx.global::<DiagramState>();
                        state.set_diagram(diagram);

                        // Create config
                        let config = DiagramConfig {
                            id: DiagramId::new(),
                            connection_id: this.conn_id.clone(),
                            name: format!("Diagram {}", chrono::Local::now().format("%Y-%m-%d %H:%M")),
                            schemas: schemas.clone(),
                            tables: Vec::new(),
                            options: this.options.clone(),
                            layout: DiagramLayout::default(),
                            created_at: chrono::Utc::now().timestamp(),
                            updated_at: chrono::Utc::now().timestamp(),
                        };
                        state.set_config(config);

                        this.loading = false;
                        cx.emit(DiagramGeneratorEvent::Generated);
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        let state = cx.global::<DiagramState>();
                        state.set_error(Some(e.to_string()));
                        this.loading = false;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(DiagramGeneratorEvent::Cancel);
    }
}

impl Render for DiagramGeneratorDialog {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // Filter tables by selected schemas
        let filtered_tables: Vec<_> = self.tables
            .iter()
            .filter(|(schema, _)| self.selected_schemas.contains(schema))
            .collect();

        Modal::new("diagram-generator")
            .title("Generate ER Diagram")
            .width(px(600.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(24.0))
                    .p(px(16.0))
                    // Schema selection
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0x374151))
                                    .child("Select Schemas")
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.0))
                                    .max_h(px(150.0))
                                    .overflow_y_auto()
                                    .p(px(8.0))
                                    .bg(rgb(0xf9fafb))
                                    .rounded_md()
                                    .children(
                                        self.schemas.iter().map(|schema| {
                                            let schema_clone = schema.clone();
                                            Checkbox::new(
                                                SharedString::from(format!("schema-{}", schema)),
                                                self.selected_schemas.contains(schema),
                                                schema.clone(),
                                                cx.listener(move |this, _, cx| {
                                                    this.toggle_schema(schema_clone.clone(), cx);
                                                }),
                                            )
                                        })
                                    )
                            )
                    )
                    // Table selection
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0x374151))
                                    .child("Tables")
                            )
                            .child(
                                Checkbox::new(
                                    "select-all-tables",
                                    self.select_all_tables,
                                    "Include all tables from selected schemas",
                                    cx.listener(|this, checked, cx| {
                                        this.select_all_tables = checked;
                                        cx.notify();
                                    }),
                                )
                            )
                            .when(!self.select_all_tables, |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(8.0))
                                        .max_h(px(200.0))
                                        .overflow_y_auto()
                                        .p(px(8.0))
                                        .bg(rgb(0xf9fafb))
                                        .rounded_md()
                                        .children(
                                            filtered_tables.iter().map(|(schema, table)| {
                                                let table_id = format!("{}.{}", schema, table);
                                                let table_id_clone = table_id.clone();
                                                Checkbox::new(
                                                    SharedString::from(format!("table-{}", table_id)),
                                                    self.selected_tables.contains(&table_id),
                                                    format!("{}.{}", schema, table),
                                                    cx.listener(move |this, _, cx| {
                                                        this.toggle_table(table_id_clone.clone(), cx);
                                                    }),
                                                )
                                            })
                                        )
                                )
                            })
                    )
                    // Display options
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0x374151))
                                    .child("Display Options")
                            )
                            .child(
                                div()
                                    .grid()
                                    .grid_cols_2()
                                    .gap(px(12.0))
                                    .child(
                                        Checkbox::new(
                                            "opt-data-types",
                                            self.options.show_data_types,
                                            "Show data types",
                                            cx.listener(|this, checked, cx| {
                                                this.options.show_data_types = checked;
                                                cx.notify();
                                            }),
                                        )
                                    )
                                    .child(
                                        Checkbox::new(
                                            "opt-nullable",
                                            self.options.show_nullable,
                                            "Show nullable",
                                            cx.listener(|this, checked, cx| {
                                                this.options.show_nullable = checked;
                                                cx.notify();
                                            }),
                                        )
                                    )
                                    .child(
                                        Checkbox::new(
                                            "opt-indexes",
                                            self.options.show_indexes,
                                            "Show indexes",
                                            cx.listener(|this, checked, cx| {
                                                this.options.show_indexes = checked;
                                                cx.notify();
                                            }),
                                        )
                                    )
                                    .child(
                                        Checkbox::new(
                                            "opt-color-schema",
                                            self.options.color_by_schema,
                                            "Color by schema",
                                            cx.listener(|this, checked, cx| {
                                                this.options.color_by_schema = checked;
                                                cx.notify();
                                            }),
                                        )
                                    )
                            )
                    )
            )
            .footer(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        Button::ghost()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, cx| this.cancel(cx)))
                    )
                    .child(
                        Button::primary()
                            .label("Generate Diagram")
                            .loading(self.loading)
                            .disabled(self.selected_schemas.is_empty())
                            .on_click(cx.listener(|this, _, cx| this.generate(cx)))
                    )
            )
    }
}

impl FocusableView for DiagramGeneratorDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
```

### 26.9 Main ER Diagram View

**File: `src/ui/diagram/er_diagram.rs`**

```rust
use crate::models::diagram::*;
use crate::state::DiagramState;
use crate::ui::diagram::{
    canvas::DiagramCanvas,
    toolbar::{DiagramToolbar, DiagramToolbarEvent},
    options_panel::DiagramOptionsPanel,
    generator_dialog::{DiagramGeneratorDialog, DiagramGeneratorEvent},
};
use gpui::*;

pub struct ERDiagram {
    conn_id: String,
    canvas: Entity<DiagramCanvas>,
    toolbar: Entity<DiagramToolbar>,
    options_panel: Entity<DiagramOptionsPanel>,
    show_generator: bool,
    generator_dialog: Option<Entity<DiagramGeneratorDialog>>,
    focus_handle: FocusHandle,
}

impl ERDiagram {
    pub fn new(conn_id: String, cx: &mut Context<Self>) -> Self {
        let canvas = cx.new(|cx| DiagramCanvas::new(conn_id.clone(), cx));
        let toolbar = cx.new(|_| DiagramToolbar::new(conn_id.clone()));
        let options_panel = cx.new(|_| DiagramOptionsPanel::new());

        // Subscribe to toolbar events
        cx.subscribe(&toolbar, Self::on_toolbar_event).detach();

        Self {
            conn_id,
            canvas,
            toolbar,
            options_panel,
            show_generator: true, // Show generator on first open
            generator_dialog: None,
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_toolbar_event(
        &mut self,
        _toolbar: Entity<DiagramToolbar>,
        event: &DiagramToolbarEvent,
        cx: &mut Context<Self>,
    ) {
        let state = cx.global::<DiagramState>();

        match event {
            DiagramToolbarEvent::FitToView => {
                // Get canvas size and fit
                state.fit_to_view(800.0, 600.0); // TODO: Get actual canvas size
            }
            DiagramToolbarEvent::ZoomIn => {
                let center = crate::models::diagram::Point::new(400.0, 300.0);
                state.zoom(1.2, center);
            }
            DiagramToolbarEvent::ZoomOut => {
                let center = crate::models::diagram::Point::new(400.0, 300.0);
                state.zoom(0.8, center);
            }
            _ => {}
        }

        cx.notify();
    }

    fn show_generator_dialog(&mut self, cx: &mut Context<Self>) {
        let dialog = cx.new(|cx| DiagramGeneratorDialog::new(self.conn_id.clone(), cx));
        cx.subscribe(&dialog, Self::on_generator_event).detach();
        self.generator_dialog = Some(dialog);
        self.show_generator = true;
        cx.notify();
    }

    fn on_generator_event(
        &mut self,
        _dialog: Entity<DiagramGeneratorDialog>,
        event: &DiagramGeneratorEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            DiagramGeneratorEvent::Cancel | DiagramGeneratorEvent::Generated => {
                self.show_generator = false;
                self.generator_dialog = None;
            }
        }
        cx.notify();
    }
}

impl Render for ERDiagram {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<DiagramState>();
        let has_diagram = state.current_diagram().is_some();

        div()
            .id("er-diagram")
            .flex_1()
            .flex()
            .flex_col()
            .bg(rgb(0xf9fafb))
            .track_focus(&self.focus_handle)
            // Toolbar
            .child(self.toolbar.clone())
            // Main content
            .child(
                div()
                    .flex_1()
                    .relative()
                    .overflow_hidden()
                    .when(has_diagram, |this| {
                        this
                            .child(self.canvas.clone())
                            .child(self.options_panel.clone())
                    })
                    .when(!has_diagram, |this| {
                        this.child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .gap(px(16.0))
                                        .child(
                                            div()
                                                .text_6xl()
                                                .text_color(rgb(0xd1d5db))
                                                .child("📊")
                                        )
                                        .child(
                                            div()
                                                .text_lg()
                                                .text_color(rgb(0x6b7280))
                                                .child("No diagram generated")
                                        )
                                        .child(
                                            crate::ui::components::Button::primary()
                                                .label("Generate ER Diagram")
                                                .on_click(cx.listener(|this, _, cx| {
                                                    this.show_generator_dialog(cx);
                                                }))
                                        )
                                )
                        )
                    })
            )
            // Generator dialog overlay
            .when_some(self.generator_dialog.clone(), |this, dialog| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.5 })
                        .z_index(100)
                        .child(dialog)
                )
            })
    }
}

impl FocusableView for ERDiagram {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
```

### 26.10 Module Organization

**File: `src/ui/diagram/mod.rs`**

```rust
mod canvas;
mod table_node;
mod toolbar;
mod options_panel;
mod generator_dialog;
mod er_diagram;

pub use canvas::DiagramCanvas;
pub use table_node::TableNodeView;
pub use toolbar::{DiagramToolbar, DiagramToolbarEvent};
pub use options_panel::DiagramOptionsPanel;
pub use generator_dialog::{DiagramGeneratorDialog, DiagramGeneratorEvent};
pub use er_diagram::ERDiagram;
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

## Testing Instructions

### Using Tauri MCP

```typescript
// Generate and interact with ER diagram
await driver_session({ action: 'start', port: 9223 });

// Open diagram generator
await webview_click({ selector: '[data-testid="generate-diagram-btn"]' });

// Select schemas
await webview_click({ selector: '[data-testid="schema-public"]' });
await webview_click({ selector: '[data-testid="schema-auth"]' });

// Generate diagram
await webview_click({ selector: '[data-testid="generate-btn"]' });

// Wait for diagram to render
await webview_wait_for({ type: 'selector', value: '#diagram-canvas' });

// Screenshot the diagram
await webview_screenshot({ filePath: 'er-diagram.png' });

// Test layout change
await webview_click({ selector: '[data-testid="layout-select"]' });
await webview_click({ selector: '[data-value="force_directed"]' });

// Export SVG
await webview_click({ selector: '[data-testid="export-svg"]' });

// Verify node interaction
const snapshot = await webview_dom_snapshot({ type: 'accessibility' });
console.log('Diagram nodes:', snapshot);

await driver_session({ action: 'stop' });
```

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_layout() {
        let mut nodes = vec![
            TableNode {
                id: "public.users".into(),
                schema: "public".into(),
                name: "users".into(),
                columns: vec![],
                indexes: vec![],
                position: Point::default(),
                size: Size { width: 220.0, height: 100.0 },
                color: Hsla::default(),
                selected: false,
                hovered: false,
            },
            TableNode {
                id: "public.posts".into(),
                schema: "public".into(),
                name: "posts".into(),
                columns: vec![],
                indexes: vec![],
                position: Point::default(),
                size: Size { width: 220.0, height: 100.0 },
                color: Hsla::default(),
                selected: false,
                hovered: false,
            },
        ];

        let edges = vec![
            RelationshipEdge {
                id: "fk_posts_users".into(),
                source_node: "public.posts".into(),
                source_column: "user_id".into(),
                target_node: "public.users".into(),
                target_column: "id".into(),
                label: "fk_posts_users".into(),
                relationship_type: RelationshipType::OneToMany,
                selected: false,
                hovered: false,
            },
        ];

        DiagramService::apply_layout(&mut nodes, &edges, LayoutAlgorithm::Hierarchical);

        // Users (root) should be at layer 0, posts at layer 1
        assert!(nodes[0].position.y < nodes[1].position.y);
    }

    #[test]
    fn test_viewport_transform() {
        let viewport = Viewport {
            x: 100.0,
            y: 50.0,
            zoom: 2.0,
        };

        let world = Point::new(200.0, 300.0);
        let screen = viewport.world_to_screen(&world);

        assert_eq!(screen.x, 500.0); // 200 * 2 + 100
        assert_eq!(screen.y, 650.0); // 300 * 2 + 50

        let back_to_world = viewport.screen_to_world(screen.x, screen.y);
        assert!((back_to_world.x - world.x).abs() < 0.001);
        assert!((back_to_world.y - world.y).abs() < 0.001);
    }

    #[test]
    fn test_bounding_box_contains() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0);

        assert!(bbox.contains(&Point::new(50.0, 40.0))); // Inside
        assert!(bbox.contains(&Point::new(10.0, 20.0))); // Corner
        assert!(!bbox.contains(&Point::new(5.0, 40.0)));  // Outside left
        assert!(!bbox.contains(&Point::new(50.0, 80.0))); // Outside bottom
    }

    #[test]
    fn test_circular_layout() {
        let mut nodes = vec![
            TableNode {
                id: "t1".into(),
                position: Point::default(),
                ..Default::default()
            },
            TableNode {
                id: "t2".into(),
                position: Point::default(),
                ..Default::default()
            },
            TableNode {
                id: "t3".into(),
                position: Point::default(),
                ..Default::default()
            },
            TableNode {
                id: "t4".into(),
                position: Point::default(),
                ..Default::default()
            },
        ];

        DiagramService::apply_layout(&mut nodes, &[], LayoutAlgorithm::Circular);

        // All nodes should be equidistant from center
        let center = Point::new(0.0, 0.0);
        let radius = nodes[0].position.distance(&center);

        for node in &nodes {
            let dist = node.position.distance(&center);
            assert!((dist - radius).abs() < 1.0);
        }
    }
}
```
