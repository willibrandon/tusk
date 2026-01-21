# Data Model: Project Initialization

**Feature**: 001-project-init
**Date**: 2026-01-20

## Overview

The Project Initialization feature establishes foundational types and structures. This feature has minimal data modeling requirements since it focuses on application scaffolding rather than data persistence.

## Entities

### TuskApp

The root application component that manages the main window.

| Field | Type | Description |
|-------|------|-------------|
| — | — | No persistent state in init phase |

**Notes**: TuskApp is a stateless component for init. State fields (connections, editors, etc.) will be added in subsequent features.

### TuskTheme

Theme configuration for application styling.

| Field | Type | Description |
|-------|------|-------------|
| name | String | Theme identifier ("dark" or "light") |
| appearance | Appearance | GPUI Appearance enum (Light/Dark) |
| colors | ThemeColors | Color palette struct |

**Relationships**: Global singleton accessed via `cx.global::<TuskTheme>()`

### ThemeColors

Color palette for UI rendering.

| Field | Type | Description |
|-------|------|-------------|
| background | Hsla | Window background color |
| surface | Hsla | Panel/card background |
| elevated_surface | Hsla | Elevated panel background |
| text | Hsla | Primary text color |
| text_muted | Hsla | Secondary/dimmed text |
| text_accent | Hsla | Accent/link text |
| border | Hsla | Element border color |
| border_variant | Hsla | Subtle border variant |
| accent | Hsla | Primary accent color |
| accent_hover | Hsla | Accent hover state |
| status_success | Hsla | Success indicator |
| status_warning | Hsla | Warning indicator |
| status_error | Hsla | Error indicator |
| status_info | Hsla | Info indicator |

**Notes**: Color values use GPUI's `Hsla` type (Hue, Saturation, Lightness, Alpha). Initial implementation provides dark theme only per spec FR-005.

### TuskError

Application error type for error handling.

| Variant | Fields | Description |
|---------|--------|-------------|
| Window | message: String | Window creation/management errors |
| Theme | message: String | Theme loading errors |
| Font | message: String, path: Option<String> | Font loading errors |
| Config | message: String | Configuration errors |

**Notes**: Error variants expanded in later features (Query, Connection, Storage, etc.)

## State Transitions

### Application Lifecycle

```
Uninitialized → Starting → Running → Terminating → Terminated
     │              │          │
     └──────────────┴──────────┴─── Error states possible
```

| State | Description | Triggered By |
|-------|-------------|--------------|
| Uninitialized | Before main() | — |
| Starting | Application::new().run() executing | Process start |
| Running | Window visible, event loop active | Window opened |
| Terminating | User requested close | Window close / Cmd+Q |
| Terminated | Process exiting | Clean shutdown |

### Theme State

```
Dark (default) ↔ Light
```

| State | Description | Triggered By |
|-------|-------------|--------------|
| Dark | Dark color palette active | Default / User toggle |
| Light | Light color palette active | User toggle |

**Notes**: Only dark theme implemented in init. Light theme added in theme feature.

## Validation Rules

### Window Dimensions

| Rule | Constraint | Error |
|------|------------|-------|
| Min width | ≥ 800 px | Enforced by GPUI window_min_size |
| Min height | ≥ 600 px | Enforced by GPUI window_min_size |
| Default width | 1400 px | Set at window creation |
| Default height | 900 px | Set at window creation |

### Theme Colors

| Rule | Constraint |
|------|------------|
| All colors | Valid Hsla (h: 0-360, s: 0-1, l: 0-1, a: 0-1) |
| Background | Must have sufficient contrast with text |
| Accent | Must be visually distinct from background |

## Data Volume Assumptions

| Entity | Expected Volume |
|--------|-----------------|
| TuskApp | 1 instance (singleton) |
| TuskTheme | 1 active theme (Global) |
| ThemeColors | ~20 color fields |

**Notes**: No persistence layer in init feature. All state is in-memory and ephemeral.

## Future Considerations

Entities added in subsequent features (not implemented in init):

- **Connection**: Database connection configuration (Feature: Connection Management)
- **QueryResult**: Query execution results (Feature: Query Execution)
- **SchemaNode**: Schema browser tree nodes (Feature: Schema Browser)
- **Editor**: SQL editor state (Feature: Query Editor)

These are documented here for architectural context but will NOT be implemented in 001-project-init.
