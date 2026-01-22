# API Contracts: Frontend Architecture

**Feature Branch**: `003-frontend-architecture`
**Generated**: 2026-01-21

## Overview

This directory contains API contracts for all UI components in the frontend architecture. Since this is a UI feature (not REST/GraphQL), contracts define component public interfaces, event signatures, and trait contracts.

## Contract Files

| File | Component(s) | Description |
|------|--------------|-------------|
| [workspace.md](./workspace.md) | `Workspace` | Root workspace container |
| [dock.md](./dock.md) | `Dock` | Collapsible panel containers |
| [pane.md](./pane.md) | `Pane`, `PaneGroup`, `PaneNode`, `TabItem` | Tab/pane management |
| [panel.md](./panel.md) | `Panel` trait | Extensible dock content |
| [tree.md](./tree.md) | `Tree<T>`, `TreeItem` trait | Schema browser tree |
| [components.md](./components.md) | `Button`, `TextInput`, `Select`, `StatusBar`, `Icon`, `Spinner` | Component library |
| [modal.md](./modal.md) | `Modal`, `ModalAction`, `ModalLayer` | Modal dialog system |
| [context-menu.md](./context-menu.md) | `ContextMenu`, `ContextMenuItem`, `ContextMenuLayer` | Context menu system |
| [keyboard.md](./keyboard.md) | Actions, KeyBindings | Keyboard navigation |
| [resizer.md](./resizer.md) | `Resizer` | Drag resize handles |

## Requirements Coverage

All 46 functional requirements from the spec are mapped to contracts:

### Workspace Shell (FR-001 to FR-003)
- [workspace.md](./workspace.md): FR-001, FR-002, FR-003

### Dock System (FR-004 to FR-007)
- [dock.md](./dock.md): FR-004, FR-005, FR-006, FR-007
- [resizer.md](./resizer.md): FR-004

### Pane and Tab System (FR-008 to FR-013)
- [pane.md](./pane.md): FR-008, FR-009, FR-010, FR-011, FR-012, FR-013

### Panel System (FR-014 to FR-016)
- [panel.md](./panel.md): FR-014, FR-015, FR-016

### Schema Browser (FR-017 to FR-020)
- [tree.md](./tree.md): FR-017, FR-018, FR-019, FR-020

### Status Bar (FR-021 to FR-024)
- [components.md](./components.md): FR-021, FR-022, FR-023, FR-024

### Component Library (FR-025 to FR-031)
- [components.md](./components.md): FR-025, FR-026, FR-027, FR-028, FR-029, FR-030, FR-031

### Modal System (FR-032 to FR-035)
- [modal.md](./modal.md): FR-032, FR-033, FR-034, FR-035

### Context Menus (FR-036 to FR-039)
- [context-menu.md](./context-menu.md): FR-036, FR-037, FR-038, FR-039

### Keyboard Navigation (FR-040 to FR-043)
- [keyboard.md](./keyboard.md): FR-040, FR-041, FR-042, FR-043

### Icons (FR-044 to FR-046)
- [components.md](./components.md): FR-044, FR-045, FR-046

## Trait Summary

| Trait | Purpose | Implementors |
|-------|---------|--------------|
| `Render` | Component rendering | All stateful components |
| `RenderOnce` | Stateless component rendering | Button, Icon, Spinner, Resizer |
| `Focusable` | Focus management | Workspace, Dock, Pane, Tree, TextInput, Select, Modal, ContextMenu |
| `EventEmitter<E>` | Event emission | Most components |
| `Panel` | Dock panel content | SchemaBrowserPanel, ResultsPanel, MessagesPanel |
| `TreeItem` | Tree node data | SchemaItem |
| `Global` | Application-wide state | ModalLayer, ContextMenuLayer |

## Event Summary

| Component | Event Type | Events |
|-----------|------------|--------|
| Workspace | `WorkspaceEvent` | DockToggled, ActivePaneChanged, LayoutChanged |
| Dock | `DockEvent` | Resized, VisibilityChanged, PanelChanged |
| Pane | `PaneEvent` | TabAdded, TabClosed, ActiveTabChanged, TabMoved, Close |
| PaneGroup | `PaneGroupEvent` | Split, PaneClosed, ActivePaneChanged, RatiosChanged |
| Panel | `PanelEvent` | Focus, Close, ActivateTab |
| Tree | `TreeEvent<Id>` | Selected, Expanded, Collapsed, Activated, ContextMenu |
| TextInput | `TextInputEvent` | Changed, Submitted, Focus, Blur |
| Select | `SelectEvent<T>` | Changed, Opened, Closed |
| Modal | `ModalEvent` | Close |
| ContextMenu | `ContextMenuEvent` | Close |
