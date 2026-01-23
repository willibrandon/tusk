# Quickstart: Service Integration Layer

**Feature**: 004-service-integration
**Date**: 2026-01-21

## Prerequisites

- Rust 1.80+ installed
- Local PostgreSQL database for testing
- Zed repository cloned at `/Users/brandon/src/zed`

## Building

```bash
cd /Users/brandon/src/tusk
cargo build
```

## Running

```bash
cargo run
```

## Testing Service Integration

### 1. Connect to Database

```rust
// In a UI component
fn connect(&mut self, cx: &mut Context<Self>) {
    let config = ConnectionConfig::builder()
        .name("Local Dev")
        .host("localhost")
        .port(5432)
        .database("postgres")
        .username("brandon")
        .build()
        .unwrap();

    let password = "your_password".to_string();

    cx.spawn(async move |this, cx| {
        let state = cx.global::<TuskState>();
        let runtime = state.runtime().handle().clone();

        let result = runtime.spawn(async move {
            state.connect(&config, &password).await
        }).await;

        this.update(&cx, |this, cx| {
            match result {
                Ok(Ok(connection_id)) => {
                    this.connection_id = Some(connection_id);
                    cx.notify();
                }
                Ok(Err(e)) => {
                    let error_info = e.to_error_info();
                    this.show_error(error_info, cx);
                }
                Err(e) => {
                    // Task panicked
                }
            }
        })?;

        Ok(())
    }).detach();
}
```

### 2. Execute Query

```rust
fn execute_query(&mut self, cx: &mut Context<Self>) {
    let connection_id = self.connection_id.unwrap();
    let sql = self.editor_content.clone();

    // Create channel for streaming results
    let (tx, rx) = mpsc::channel::<QueryEvent>(100);

    // Start streaming
    self.results_panel.start_streaming(rx, cx);

    cx.spawn(async move |this, cx| {
        let state = cx.global::<TuskState>();
        let runtime = state.runtime().handle().clone();

        let result = runtime.spawn(async move {
            state.execute_query_streaming(connection_id, &sql, tx).await
        }).await;

        this.update(&cx, |this, cx| {
            match result {
                Ok(Ok(handle)) => {
                    this.active_query = Some(handle);
                }
                Ok(Err(e)) => {
                    this.show_error(e.to_error_info(), cx);
                }
                _ => {}
            }
        })?;

        Ok(())
    }).detach();
}
```

### 3. Cancel Query

```rust
fn cancel_query(&mut self, cx: &mut Context<Self>) {
    if let Some(handle) = &self.active_query {
        let query_id = handle.id();

        cx.spawn(async move |this, cx| {
            let state = cx.global::<TuskState>();
            state.cancel_query(query_id).await?;

            this.update(&cx, |this, cx| {
                this.active_query = None;
                this.show_toast("Query cancelled", cx);
            })?;

            Ok(())
        }).detach();
    }
}
```

### 4. Load Schema

```rust
fn load_schema(&mut self, cx: &mut Context<Self>) {
    let connection_id = self.connection_id.unwrap();
    self.loading = true;

    cx.spawn(async move |this, cx| {
        let state = cx.global::<TuskState>();
        let runtime = state.runtime().handle().clone();

        let result = runtime.spawn(async move {
            state.get_schema(connection_id).await
        }).await;

        this.update(&cx, |this, cx| {
            this.loading = false;
            match result {
                Ok(Ok(schema)) => {
                    this.tree_items = build_tree_items(&schema);
                    cx.notify();
                }
                Ok(Err(e)) => {
                    this.show_error(e.to_error_info(), cx);
                }
                _ => {}
            }
        })?;

        Ok(())
    }).detach();
}
```

### 5. Handle Streaming Results

```rust
fn start_streaming(&mut self, rx: mpsc::Receiver<QueryEvent>, cx: &mut Context<Self>) {
    self.status = ResultsStatus::Loading;
    self.rows.clear();

    self._stream_task = Some(cx.spawn(async move |this, cx| {
        while let Some(event) = rx.recv().await {
            let should_continue = this.update(&cx, |panel, cx| {
                match event {
                    QueryEvent::Columns(columns) => {
                        panel.columns = columns;
                        panel.status = ResultsStatus::Streaming;
                    }
                    QueryEvent::Rows(rows, total) => {
                        panel.rows.extend(rows);
                        panel.total_rows = total;
                    }
                    QueryEvent::Complete { total_rows, execution_time_ms, .. } => {
                        panel.total_rows = total_rows;
                        panel.execution_time_ms = Some(execution_time_ms);
                        panel.status = ResultsStatus::Complete;
                        return false; // Stop loop
                    }
                    QueryEvent::Error(err) => {
                        panel.error = Some(err.to_error_info());
                        panel.status = ResultsStatus::Error;
                        return false; // Stop loop
                    }
                    _ => {}
                }
                cx.notify();
                true // Continue loop
            })?;

            if !should_continue {
                break;
            }
        }
        Ok(())
    }));
}
```

## Key Patterns

### Accessing Global State

```rust
let state = cx.global::<TuskState>();
```

### Spawning Async Database Work

```rust
cx.spawn(async move |this, cx| {
    let state = cx.global::<TuskState>();
    let runtime = state.runtime().handle().clone();

    let result = runtime.spawn(async move {
        // Database operations here
    }).await;

    this.update(&cx, |this, cx| {
        // Update UI with result
        cx.notify();
    })?;

    Ok(())
}).detach();
```

### Error Display

```rust
// Toast for recoverable errors
workspace.show_toast(StatusToast::new(&error.message, cx, |t, _| {
    t.icon(ToastIcon::new(IconName::Warning).color(Color::Warning))
}), cx);

// Panel for detailed errors
results_panel.show_error(error_info, cx);
```

### Cancellation via Task Replacement

```rust
// Old task automatically cancelled when replaced
self._query_task = Some(new_task);
```

## Testing

```bash
# Run all tests
cargo test

# Run core service tests
cargo test -p tusk_core

# Run UI integration tests
cargo test -p tusk_ui
```

## Common Issues

### Connection Timeout

If connections timeout, check:
1. PostgreSQL is running: `pg_isready -h localhost`
2. Firewall allows port 5432
3. Connection config is correct

### Query Cancellation Not Working

Ensure:
1. QueryHandle is stored in component state
2. `cancel_query()` is called with correct query_id
3. PostgreSQL supports cancellation (most versions do)

### UI Not Updating

Remember to call `cx.notify()` after state changes in:
- `this.update(&cx, |...|)` callbacks
- Event handlers
- After receiving streaming results

## Next Steps

After implementing service integration:
1. Run `/speckit.tasks` to generate implementation tasks
2. Implement tasks in order
3. Test each user story acceptance scenario
