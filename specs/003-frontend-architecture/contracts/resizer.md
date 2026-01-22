# Contract: Resizer

**Component**: `Resizer`
**Module**: `tusk_ui::resizer`
**Implements**: `Render`

## Public Interface

```rust
pub struct Resizer {
    axis: Axis,
    on_resize: Box<dyn Fn(Pixels, &mut App)>,
}

impl Resizer {
    /// Create a new resizer for the given axis
    pub fn new(axis: Axis) -> Self;

    /// Set the resize callback (receives delta in pixels)
    pub fn on_resize(self, handler: impl Fn(Pixels, &mut App) + 'static) -> Self;
}
```

## Axis Behavior

| Axis | Cursor | Drag Direction | Delta |
|------|--------|----------------|-------|
| Horizontal | `CursorStyle::ResizeLeftRight` | Left/Right | x delta |
| Vertical | `CursorStyle::ResizeUpDown` | Up/Down | y delta |

## Usage Pattern

```rust
// In Dock render (side dock)
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let entity = cx.entity().clone();

    h_flex()
        .child(self.render_content(cx))
        .child(
            Resizer::new(Axis::Horizontal)
                .on_resize(move |delta, cx| {
                    entity.update(cx, |dock, cx| {
                        let new_size = dock.size + delta;
                        dock.set_size(new_size, cx);
                    });
                })
        )
}

// In PaneGroup render (pane divider)
fn render_split(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let entity = cx.entity().clone();
    let index = self.divider_index;

    match self.axis {
        Axis::Horizontal => {
            h_flex()
                .child(self.render_child(0, cx))
                .child(
                    Resizer::new(Axis::Horizontal)
                        .on_resize(move |delta, cx| {
                            entity.update(cx, |group, cx| {
                                group.resize_at(index, delta, cx);
                            });
                        })
                )
                .child(self.render_child(1, cx))
        }
        Axis::Vertical => {
            v_flex()
                .child(self.render_child(0, cx))
                .child(
                    Resizer::new(Axis::Vertical)
                        .on_resize(move |delta, cx| {
                            entity.update(cx, |group, cx| {
                                group.resize_at(index, delta, cx);
                            });
                        })
                )
                .child(self.render_child(1, cx))
        }
    }
}
```

## Rendering

```rust
impl RenderOnce for Resizer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();
        let (width, height, cursor) = match self.axis {
            Axis::Horizontal => (px(6.0), relative(1.0), CursorStyle::ResizeLeftRight),
            Axis::Vertical => (relative(1.0), px(6.0), CursorStyle::ResizeUpDown),
        };

        div()
            .w(width)
            .h(height)
            .flex_shrink_0()
            .cursor(cursor)
            .bg(theme.colors.border.opacity(0.0))
            .hover(|s| s.bg(theme.colors.border))
            .on_drag(DragState::new(), move |drag, event, cx| {
                let delta = match self.axis {
                    Axis::Horizontal => event.position.x - drag.start.x,
                    Axis::Vertical => event.position.y - drag.start.y,
                };
                (self.on_resize)(px(delta), cx);
                drag.start = event.position;
            })
    }
}
```

## Drag State

```rust
struct DragState {
    start: Point<f32>,
}

impl DragState {
    fn new() -> Self {
        Self { start: Point::default() }
    }
}
```

## Performance

- Resize events are processed at render rate (60fps)
- No debouncing needed; GPUI handles this efficiently
- Delta is computed per frame, not accumulated
- Size constraints enforced in `Dock::set_size()` / `PaneGroup::resize_at()`

## Requirements Mapping

| Requirement | Implementation |
|-------------|----------------|
| FR-004 | Dock resize via Resizer component |
| FR-005, FR-006 | Constraints enforced in Dock.set_size() |
| SC-002 | 60fps via GPUI immediate mode rendering |
| SC-010 | No debounce needed; frame-rate limited naturally |
