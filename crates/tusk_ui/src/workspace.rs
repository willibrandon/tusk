//! Workspace component - the main application shell.
//!
//! The Workspace is the root component that manages the overall layout including
//! docks (left, right, bottom) and the center pane group.

use gpui::{
    canvas, div, prelude::*, px, App, Axis, Bounds, Context, DragMoveEvent, Entity, EventEmitter,
    FocusHandle, KeyContext, Pixels, Point, Render, Subscription, Window,
};
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use crate::context_menu::ContextMenuLayer;
use crate::dock::{Dock, DockEvent, DraggedDock};
use crate::icon::IconName;
use crate::modal::ModalLayer;
use crate::key_bindings::{
    ActivateTab1, ActivateTab2, ActivateTab3, ActivateTab4, ActivateTab5, ActivateTab6,
    ActivateTab7, ActivateTab8, ActivateTab9, CloseActiveTab, ClosePane, FocusNextPane,
    FocusPreviousPane, FocusResults, FocusSchemaBrowser, NewQueryTab, NextTab, PreviousTab,
    SplitDown, SplitRight, ToggleBottomDock, ToggleLeftDock, ToggleRightDock,
};
use crate::layout::sizes::STATUS_BAR_HEIGHT;
use crate::layout::spacing;
use crate::pane::{Pane, PaneGroup, PaneGroupEvent, PaneLayout, TabItem};
use crate::panel::{DockPosition, Focusable};
use crate::panels::{MessagesPanel, ResultsPanel, SchemaBrowserPanel};
use crate::status_bar::{ConnectionStatus, ExecutionState, StatusBar};
use crate::TuskTheme;

// ============================================================================
// QueryPlaceholderView - Placeholder for query editor tabs
// ============================================================================

/// A placeholder view shown in query tabs until the actual SQL editor is implemented.
///
/// This provides a simple UI that explains the tab is a placeholder and hints at
/// the keyboard shortcut for creating new tabs.
pub struct QueryPlaceholderView {
    title: String,
}

impl QueryPlaceholderView {
    /// Create a new placeholder view with the given title.
    pub fn new(title: String) -> Self {
        Self { title }
    }
}

impl Render for QueryPlaceholderView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .bg(theme.colors.editor_background)
            .gap(spacing::MD)
            .child(
                div()
                    .text_xl()
                    .text_color(theme.colors.text)
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.colors.text_muted)
                    .child("SQL Editor placeholder - implementation coming soon"),
            )
            .child(
                div()
                    .mt(spacing::LG)
                    .text_xs()
                    .text_color(theme.colors.text_muted)
                    .child("Press Cmd+W to close this tab"),
            )
    }
}

/// Key used to store workspace state in the UI state storage.
pub const WORKSPACE_STATE_KEY: &str = "workspace_state";

/// Events emitted by the workspace.
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    /// Dock visibility changed.
    DockToggled {
        position: DockPosition,
        visible: bool,
    },
    /// Active pane changed.
    ActivePaneChanged { pane: Entity<Pane> },
    /// Layout changed (split, close, resize).
    LayoutChanged,
}

/// Persisted workspace state for restoration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    /// Left dock size in pixels.
    pub left_dock_size: f32,
    /// Left dock visibility.
    pub left_dock_visible: bool,
    /// Right dock size in pixels (if enabled).
    pub right_dock_size: Option<f32>,
    /// Right dock visibility (if enabled).
    pub right_dock_visible: Option<bool>,
    /// Bottom dock size in pixels.
    pub bottom_dock_size: f32,
    /// Bottom dock visibility.
    pub bottom_dock_visible: bool,
    /// Serialized pane layout.
    pub pane_layout: PaneLayout,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            left_dock_size: 250.0,
            left_dock_visible: true,
            right_dock_size: None,
            right_dock_visible: None,
            bottom_dock_size: 200.0,
            bottom_dock_visible: true,
            pane_layout: PaneLayout::default(),
        }
    }
}

/// The main workspace component.
///
/// Manages the overall application layout including:
/// - Left dock (schema browser, etc.)
/// - Right dock (optional, for secondary panels)
/// - Bottom dock (results, messages, etc.)
/// - Center pane group (query editors)
pub struct Workspace {
    /// Left dock entity.
    left_dock: Entity<Dock>,
    /// Right dock entity (optional).
    right_dock: Option<Entity<Dock>>,
    /// Bottom dock entity.
    bottom_dock: Entity<Dock>,
    /// Center pane group.
    center: Entity<PaneGroup>,
    /// Schema browser panel entity.
    schema_browser: Entity<SchemaBrowserPanel>,
    /// Results panel entity.
    results_panel: Entity<ResultsPanel>,
    /// Messages panel entity.
    messages_panel: Entity<MessagesPanel>,
    /// Focus handle for the workspace.
    focus_handle: FocusHandle,
    /// Subscriptions to child component events.
    _subscriptions: Vec<Subscription>,
    /// Current bounds of the workspace (for drag calculations).
    bounds: Bounds<Pixels>,
    /// Previous dock drag coordinates (to avoid duplicate processing).
    previous_dock_drag_coordinates: Option<Point<Pixels>>,
    /// Last calculated viewport height (for bottom dock 50% constraint).
    last_viewport_height: Pixels,
    /// Current connection status for the status bar.
    connection_status: ConnectionStatus,
    /// Current query execution state for the status bar.
    execution_state: ExecutionState,
}

impl Workspace {
    /// Create a new workspace with default layout.
    ///
    /// Attempts to load persisted workspace state from storage. If persistence
    /// is available and state exists, the dock sizes and visibility will be
    /// restored to their previous values.
    ///
    /// Performance target: < 500ms (SC-001)
    #[tracing::instrument(level = "debug", skip_all, name = "workspace_new")]
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Try to load persisted state
        let persisted_state = Self::load_persisted_state(cx);

        // Create docks
        let left_dock = cx.new(|cx| Dock::new(DockPosition::Left, cx));
        let bottom_dock = cx.new(|cx| Dock::new(DockPosition::Bottom, cx));

        // Create and register the schema browser panel with the left dock
        let schema_browser = cx.new(SchemaBrowserPanel::new);
        left_dock.update(cx, |dock, cx| {
            dock.add_panel(Arc::new(schema_browser.clone()), cx);
        });

        // Create and register the results and messages panels with the bottom dock
        let results_panel = cx.new(ResultsPanel::new);
        let messages_panel = cx.new(MessagesPanel::new);
        bottom_dock.update(cx, |dock, cx| {
            dock.add_panel(Arc::new(results_panel.clone()), cx);
            dock.add_panel(Arc::new(messages_panel.clone()), cx);
        });

        // Create center pane group with one initial pane
        let center = cx.new(|cx| PaneGroup::new(window, cx));

        // Subscribe to dock events with persistence save
        let mut subscriptions = Vec::new();

        subscriptions.push(cx.subscribe(&left_dock, |this, _dock, event: &DockEvent, cx| {
            if let DockEvent::VisibilityChanged { visible } = event {
                cx.emit(WorkspaceEvent::DockToggled {
                    position: DockPosition::Left,
                    visible: *visible,
                });
            }
            cx.emit(WorkspaceEvent::LayoutChanged);
            // Save state on dock changes
            this.save_state_to_storage(cx);
            cx.notify();
        }));

        subscriptions.push(cx.subscribe(
            &bottom_dock,
            |this, _dock, event: &DockEvent, cx| {
                if let DockEvent::VisibilityChanged { visible } = event {
                    cx.emit(WorkspaceEvent::DockToggled {
                        position: DockPosition::Bottom,
                        visible: *visible,
                    });
                }
                cx.emit(WorkspaceEvent::LayoutChanged);
                // Save state on dock changes
                this.save_state_to_storage(cx);
                cx.notify();
            },
        ));

        // Subscribe to center pane group events
        subscriptions.push(cx.subscribe(
            &center,
            |_this, _pane_group, event: &PaneGroupEvent, cx| {
                if let PaneGroupEvent::ActivePaneChanged { pane } = event {
                    cx.emit(WorkspaceEvent::ActivePaneChanged { pane: pane.clone() });
                }
                cx.emit(WorkspaceEvent::LayoutChanged);
                cx.notify();
            },
        ));

        let mut workspace = Self {
            left_dock,
            right_dock: None,
            bottom_dock,
            center,
            schema_browser,
            results_panel,
            messages_panel,
            focus_handle,
            _subscriptions: subscriptions,
            bounds: Bounds::default(),
            previous_dock_drag_coordinates: None,
            last_viewport_height: px(800.0), // Default, will be updated on first render
            connection_status: ConnectionStatus::default(),
            execution_state: ExecutionState::default(),
        };

        // Restore persisted state if available
        if let Some(state) = persisted_state {
            workspace.restore_state(state, cx);
        }

        workspace
    }

    /// Load persisted workspace state from storage.
    ///
    /// Returns None if TuskState is not available or no state has been saved.
    #[allow(unused_variables)]
    fn load_persisted_state(cx: &App) -> Option<WorkspaceState> {
        #[cfg(feature = "persistence")]
        {
            use tusk_core::TuskState;
            if let Some(state) = cx.try_global::<TuskState>() {
                if let Ok(Some(json_value)) = state.storage().load_ui_state(WORKSPACE_STATE_KEY) {
                    if let Ok(workspace_state) = serde_json::from_value(json_value) {
                        tracing::debug!("Loaded persisted workspace state");
                        return Some(workspace_state);
                    }
                }
            }
        }
        // Return None for non-persistence builds or if state doesn't exist
        None
    }

    /// Save current workspace state to storage.
    ///
    /// Called automatically when dock sizes or visibility change.
    fn save_state_to_storage(&self, cx: &App) {
        #[cfg(feature = "persistence")]
        {
            use tusk_core::TuskState;
            if let Some(tusk_state) = cx.try_global::<TuskState>() {
                let state = self.save_state(cx);
                if let Ok(json_value) = serde_json::to_value(&state) {
                    if let Err(e) = tusk_state.storage().save_ui_state(WORKSPACE_STATE_KEY, &json_value) {
                        tracing::warn!(error = %e, "Failed to save workspace state");
                    } else {
                        tracing::trace!("Saved workspace state");
                    }
                }
            }
        }
        // No-op for non-persistence builds
        let _ = cx;
    }

    /// Update the bottom dock's max height constraint based on viewport size.
    ///
    /// Called when the workspace bounds change. The bottom dock is constrained
    /// to a maximum of 50% of the available viewport height.
    fn update_bottom_dock_max_height(&mut self, viewport_height: Pixels, cx: &mut Context<Self>) {
        // Only update if the height has changed significantly
        let height_changed = (f32::from(viewport_height) - f32::from(self.last_viewport_height)).abs() > 1.0;
        if height_changed {
            self.last_viewport_height = viewport_height;

            // Calculate 50% of the viewport height (minus status bar)
            let available_height = viewport_height - STATUS_BAR_HEIGHT;
            let max_bottom_height = px(f32::from(available_height) * 0.5);

            self.bottom_dock.update(cx, |dock, cx| {
                dock.set_max_bottom_height(max_bottom_height, cx);
            });
        }
    }

    /// Get the left dock entity.
    pub fn left_dock(&self) -> &Entity<Dock> {
        &self.left_dock
    }

    /// Get the right dock entity (if present).
    pub fn right_dock(&self) -> Option<&Entity<Dock>> {
        self.right_dock.as_ref()
    }

    /// Get the bottom dock entity.
    pub fn bottom_dock(&self) -> &Entity<Dock> {
        &self.bottom_dock
    }

    /// Get the center pane group.
    pub fn center(&self) -> &Entity<PaneGroup> {
        &self.center
    }

    /// Get the schema browser panel entity.
    pub fn schema_browser(&self) -> &Entity<SchemaBrowserPanel> {
        &self.schema_browser
    }

    /// Get the results panel entity.
    pub fn results_panel(&self) -> &Entity<ResultsPanel> {
        &self.results_panel
    }

    /// Get the messages panel entity.
    pub fn messages_panel(&self) -> &Entity<MessagesPanel> {
        &self.messages_panel
    }

    /// Get the current connection status.
    pub fn connection_status(&self) -> &ConnectionStatus {
        &self.connection_status
    }

    /// Set the connection status (updates the status bar).
    pub fn set_connection_status(&mut self, status: ConnectionStatus, cx: &mut Context<Self>) {
        self.connection_status = status;
        cx.notify();
    }

    /// Get the current execution state.
    pub fn execution_state(&self) -> &ExecutionState {
        &self.execution_state
    }

    /// Set the execution state (updates the status bar).
    pub fn set_execution_state(&mut self, state: ExecutionState, cx: &mut Context<Self>) {
        self.execution_state = state;
        cx.notify();
    }

    /// Get the active pane from the center pane group.
    pub fn active_pane(&self, cx: &App) -> Entity<Pane> {
        self.center.read(cx).active_pane().clone()
    }

    /// Toggle dock visibility by position.
    pub fn toggle_dock(&mut self, position: DockPosition, cx: &mut Context<Self>) {
        match position {
            DockPosition::Left => {
                self.left_dock.update(cx, |dock, cx| {
                    dock.toggle_visibility(cx);
                });
            }
            DockPosition::Right => {
                if let Some(right_dock) = &self.right_dock {
                    right_dock.update(cx, |dock, cx| {
                        dock.toggle_visibility(cx);
                    });
                }
            }
            DockPosition::Bottom => {
                self.bottom_dock.update(cx, |dock, cx| {
                    dock.toggle_visibility(cx);
                });
            }
        }
    }

    /// Open a new tab in the active pane.
    pub fn open_tab(&mut self, item: TabItem, cx: &mut Context<Self>) {
        self.center.update(cx, |pane_group, cx| {
            let active_pane = pane_group.active_pane();
            active_pane.update(cx, |pane, cx| {
                pane.add_tab(item, cx);
            });
        });
    }

    /// Split the active pane along the given axis.
    pub fn split_pane(&mut self, axis: Axis, window: &mut Window, cx: &mut Context<Self>) {
        self.center.update(cx, |pane_group, cx| {
            pane_group.split_active_pane(axis, window, cx);
        });
    }

    /// Close the active tab.
    pub fn close_active_tab(&mut self, cx: &mut Context<Self>) {
        self.center.update(cx, |pane_group, cx| {
            let active_pane = pane_group.active_pane();
            active_pane.update(cx, |pane, cx| {
                pane.close_active_tab(cx);
            });
        });
    }

    /// Focus the next pane.
    pub fn focus_next_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.center.update(cx, |pane_group, cx| {
            pane_group.focus_next_pane(window, cx);
        });
    }

    /// Focus the previous pane.
    pub fn focus_previous_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.center.update(cx, |pane_group, cx| {
            pane_group.focus_previous_pane(window, cx);
        });
    }

    /// Close the active pane.
    ///
    /// If this is the last pane, the pane remains but its tabs are closed.
    pub fn close_active_pane(&mut self, cx: &mut Context<Self>) {
        let active_pane = self.center.read(cx).active_pane().clone();
        self.center.update(cx, |pane_group, cx| {
            pane_group.close_pane(active_pane, cx);
        });
    }

    /// Activate next tab in active pane.
    pub fn next_tab(&mut self, cx: &mut Context<Self>) {
        self.center.update(cx, |pane_group, cx| {
            let active_pane = pane_group.active_pane();
            active_pane.update(cx, |pane, cx| {
                pane.activate_next_tab(cx);
            });
        });
    }

    /// Activate previous tab in active pane.
    pub fn previous_tab(&mut self, cx: &mut Context<Self>) {
        self.center.update(cx, |pane_group, cx| {
            let active_pane = pane_group.active_pane();
            active_pane.update(cx, |pane, cx| {
                pane.activate_previous_tab(cx);
            });
        });
    }

    /// Activate a tab by index (0-based) in the active pane.
    pub fn activate_tab_by_index(&mut self, index: usize, cx: &mut Context<Self>) {
        self.center.update(cx, |pane_group, cx| {
            let active_pane = pane_group.active_pane();
            active_pane.update(cx, |pane, cx| {
                pane.activate_tab(index, cx);
            });
        });
    }

    /// Focus the schema browser panel.
    pub fn focus_schema_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Show the left dock if hidden
        self.left_dock.update(cx, |dock, cx| {
            if !dock.is_visible() {
                dock.set_visible(true, cx);
            }
        });
        // Focus the schema browser (window is captured, cx derefs to &mut App)
        self.schema_browser.update(cx, |sb, cx| {
            sb.focus_handle(cx).focus(window, cx);
        });
    }

    /// Focus the results panel in the bottom dock.
    pub fn focus_results(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Show the bottom dock if hidden, then focus it
        self.bottom_dock.update(cx, |dock, cx| {
            if !dock.is_visible() {
                dock.set_visible(true, cx);
            }
            dock.focus_handle(cx).focus(window, cx);
        });
    }

    /// Create a new query tab in the active pane.
    ///
    /// This creates a placeholder query tab. The actual SQL editor component
    /// will be implemented in a later feature.
    pub fn new_query_tab(&mut self, cx: &mut Context<Self>) {
        // Count existing tabs to generate a unique title
        let query_count = self.center.read(cx).active_pane().read(cx).tabs().len() + 1;
        let title = format!("Query {}", query_count);

        // Create a placeholder view for the query editor
        let placeholder_view = cx.new(|_cx| QueryPlaceholderView::new(title.clone()));

        let tab = TabItem::new(title, placeholder_view).with_icon(IconName::Code);

        self.open_tab(tab, cx);
    }

    /// Resize the left dock to the given size.
    pub fn resize_left_dock(&mut self, size: Pixels, cx: &mut Context<Self>) {
        self.left_dock.update(cx, |dock, cx| {
            dock.set_size(size, cx);
        });
    }

    /// Resize the right dock to the given size.
    pub fn resize_right_dock(&mut self, size: Pixels, cx: &mut Context<Self>) {
        if let Some(right_dock) = &self.right_dock {
            right_dock.update(cx, |dock, cx| {
                dock.set_size(size, cx);
            });
        }
    }

    /// Resize the bottom dock to the given size.
    pub fn resize_bottom_dock(&mut self, size: Pixels, cx: &mut Context<Self>) {
        self.bottom_dock.update(cx, |dock, cx| {
            dock.set_size(size, cx);
        });
    }

    /// Save workspace state to storage.
    pub fn save_state(&self, cx: &App) -> WorkspaceState {
        let left_dock = self.left_dock.read(cx);
        let bottom_dock = self.bottom_dock.read(cx);

        let (right_dock_size, right_dock_visible) = self
            .right_dock
            .as_ref()
            .map(|dock| {
                let d = dock.read(cx);
                let size: f32 = d.size().into();
                (Some(size), Some(d.is_visible()))
            })
            .unwrap_or((None, None));

        let left_size: f32 = left_dock.size().into();
        let bottom_size: f32 = bottom_dock.size().into();

        // Get pane layout from center pane group
        let pane_layout = self.center.read(cx).layout();

        WorkspaceState {
            left_dock_size: left_size,
            left_dock_visible: left_dock.is_visible(),
            right_dock_size,
            right_dock_visible,
            bottom_dock_size: bottom_size,
            bottom_dock_visible: bottom_dock.is_visible(),
            pane_layout,
        }
    }

    /// Restore workspace state from storage.
    pub fn restore_state(&mut self, state: WorkspaceState, cx: &mut Context<Self>) {
        self.left_dock.update(cx, |dock, cx| {
            dock.set_size(px(state.left_dock_size), cx);
            dock.set_visible(state.left_dock_visible, cx);
        });

        self.bottom_dock.update(cx, |dock, cx| {
            dock.set_size(px(state.bottom_dock_size), cx);
            dock.set_visible(state.bottom_dock_visible, cx);
        });

        if let Some(right_dock) = &self.right_dock {
            if let (Some(size), Some(visible)) = (state.right_dock_size, state.right_dock_visible) {
                right_dock.update(cx, |dock, cx| {
                    dock.set_size(px(size), cx);
                    dock.set_visible(visible, cx);
                });
            }
        }
    }

    /// Add a right dock (optional feature).
    pub fn add_right_dock(&mut self, cx: &mut Context<Self>) {
        if self.right_dock.is_none() {
            let right_dock = cx.new(|cx| Dock::new(DockPosition::Right, cx));

            self._subscriptions
                .push(cx.subscribe(&right_dock, |_this, _dock, event: &DockEvent, cx| {
                    if let DockEvent::VisibilityChanged { visible } = event {
                        cx.emit(WorkspaceEvent::DockToggled {
                            position: DockPosition::Right,
                            visible: *visible,
                        });
                    }
                    cx.emit(WorkspaceEvent::LayoutChanged);
                    cx.notify();
                }));

            self.right_dock = Some(right_dock);
            cx.notify();
        }
    }

    /// Build the key context for this workspace.
    fn dispatch_context() -> KeyContext {
        let mut context = KeyContext::new_with_defaults();
        context.add("Workspace");
        context
    }

    /// Render the status bar.
    fn render_status_bar(&self, _cx: &App) -> impl IntoElement {
        StatusBar::new()
            .connection_status(self.connection_status.clone())
            .execution_state(self.execution_state.clone())
    }
}

impl EventEmitter<WorkspaceEvent> for Workspace {}

impl Focusable for Workspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Workspace {
    /// Performance target: render within 16ms for 60fps
    #[tracing::instrument(level = "trace", skip_all, name = "workspace_render")]
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();
        let dispatch_context = Self::dispatch_context();

        // Get entity handle for bounds tracking canvas
        let this = cx.entity().clone();

        // Main workspace container
        div()
            .key_context(dispatch_context)
            .track_focus(&self.focus_handle)
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(theme.colors.background)
            .text_color(theme.colors.text)
            // Track bounds using canvas element (Zed's pattern)
            // Also update bottom dock max height constraint when viewport changes
            .child({
                canvas(
                    {
                        let this = this.clone();
                        move |bounds, _window, cx| {
                            this.update(cx, |workspace, cx| {
                                workspace.bounds = bounds;
                                // Update bottom dock 50% viewport constraint when height changes
                                workspace.update_bottom_dock_max_height(bounds.size.height, cx);
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            // Handle dock resize via drag move
            .on_drag_move(cx.listener(
                |this, e: &DragMoveEvent<DraggedDock>, _window, cx| {
                    // Avoid processing duplicate coordinates
                    if this.previous_dock_drag_coordinates != Some(e.event.position) {
                        this.previous_dock_drag_coordinates = Some(e.event.position);

                        match e.drag(cx).0 {
                            DockPosition::Left => {
                                // Left dock: width = mouse X position relative to workspace left
                                let new_size = e.event.position.x - this.bounds.left();
                                this.resize_left_dock(new_size, cx);
                            }
                            DockPosition::Right => {
                                // Right dock: width = workspace right edge - mouse X position
                                let new_size = this.bounds.right() - e.event.position.x;
                                this.resize_right_dock(new_size, cx);
                            }
                            DockPosition::Bottom => {
                                // Bottom dock: height = workspace bottom edge - mouse Y position
                                let new_size = this.bounds.bottom() - e.event.position.y;
                                this.resize_bottom_dock(new_size, cx);
                            }
                        }
                    }
                },
            ))
            // Register action handlers
            .on_action(cx.listener(|this, _: &ToggleLeftDock, _window, cx| {
                this.toggle_dock(DockPosition::Left, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleRightDock, _window, cx| {
                this.toggle_dock(DockPosition::Right, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleBottomDock, _window, cx| {
                this.toggle_dock(DockPosition::Bottom, cx);
            }))
            .on_action(cx.listener(|this, _: &CloseActiveTab, _window, cx| {
                this.close_active_tab(cx);
            }))
            .on_action(cx.listener(|this, _: &NextTab, _window, cx| {
                this.next_tab(cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousTab, _window, cx| {
                this.previous_tab(cx);
            }))
            .on_action(cx.listener(|this, _: &SplitRight, window, cx| {
                this.split_pane(Axis::Horizontal, window, cx);
            }))
            .on_action(cx.listener(|this, _: &SplitDown, window, cx| {
                this.split_pane(Axis::Vertical, window, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusNextPane, window, cx| {
                this.focus_next_pane(window, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusPreviousPane, window, cx| {
                this.focus_previous_pane(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ClosePane, _window, cx| {
                this.close_active_pane(cx);
            }))
            .on_action(cx.listener(|this, _: &NewQueryTab, _window, cx| {
                this.new_query_tab(cx);
            }))
            // Tab activation by index (Cmd+1-9)
            .on_action(cx.listener(|this, _: &ActivateTab1, _window, cx| {
                this.activate_tab_by_index(0, cx);
            }))
            .on_action(cx.listener(|this, _: &ActivateTab2, _window, cx| {
                this.activate_tab_by_index(1, cx);
            }))
            .on_action(cx.listener(|this, _: &ActivateTab3, _window, cx| {
                this.activate_tab_by_index(2, cx);
            }))
            .on_action(cx.listener(|this, _: &ActivateTab4, _window, cx| {
                this.activate_tab_by_index(3, cx);
            }))
            .on_action(cx.listener(|this, _: &ActivateTab5, _window, cx| {
                this.activate_tab_by_index(4, cx);
            }))
            .on_action(cx.listener(|this, _: &ActivateTab6, _window, cx| {
                this.activate_tab_by_index(5, cx);
            }))
            .on_action(cx.listener(|this, _: &ActivateTab7, _window, cx| {
                this.activate_tab_by_index(6, cx);
            }))
            .on_action(cx.listener(|this, _: &ActivateTab8, _window, cx| {
                this.activate_tab_by_index(7, cx);
            }))
            .on_action(cx.listener(|this, _: &ActivateTab9, _window, cx| {
                this.activate_tab_by_index(8, cx);
            }))
            // Panel focus shortcuts
            .on_action(cx.listener(|this, _: &FocusSchemaBrowser, window, cx| {
                this.focus_schema_browser(window, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusResults, window, cx| {
                this.focus_results(window, cx);
            }))
            // Main content area (horizontal: left dock | center | right dock)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    // Left dock
                    .child(self.left_dock.clone())
                    // Center content (vertical: panes | bottom dock)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            // Center pane group
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .child(self.center.clone()),
                            )
                            // Bottom dock
                            .child(self.bottom_dock.clone()),
                    )
                    // Right dock (if present)
                    .children(self.right_dock.clone()),
            )
            // Status bar
            .child(self.render_status_bar(cx))
            // Context menu layer (T104) - rendered above main content but below modals
            .children(cx.try_global::<ContextMenuLayer>().and_then(|layer| layer.render()))
            // Modal layer (T094) - rendered above all content
            .children(cx.try_global::<ModalLayer>().and_then(|layer| layer.render()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_state_default() {
        let state = WorkspaceState::default();
        assert_eq!(state.left_dock_size, 250.0);
        assert!(state.left_dock_visible);
        assert_eq!(state.bottom_dock_size, 200.0);
        assert!(state.bottom_dock_visible);
        assert!(state.right_dock_size.is_none());
    }

    #[test]
    fn test_workspace_state_serialization() {
        let state = WorkspaceState::default();
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: WorkspaceState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.left_dock_size, deserialized.left_dock_size);
    }
}
