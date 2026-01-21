# Logging Contract

**Module**: `tusk_core::logging`
**Requirements**: FR-022 through FR-024a

---

## LogConfig

Logging configuration.

| Field | Type | Description |
|-------|------|-------------|
| `log_dir` | `PathBuf` | Directory for log files |
| `is_pty` | `bool` | Whether stdout is a PTY (terminal) |
| `log_filter` | `Option<String>` | Override default log level filter |

---

## LoggingGuard

Guard that ensures logs are flushed on shutdown.

Must be held in main() until application exits.

| Field | Type | Description |
|-------|------|-------------|
| `_worker_guard` | `Option<WorkerGuard>` | Tracing appender worker guard |

---

## Initialization Functions

```rust
// Initialize application logging (FR-022, FR-023, FR-024)
// Logs to console and file simultaneously (FR-022)
// Uses daily rotating log files (FR-023)
// Log level varies by build type (FR-024)
// If file logging fails, continues with console-only (FR-024a)
// Log write latency < 100ms (SC-007)
//
// Returns guard that must be held until application exits
fn init_logging(config: LogConfig) -> LoggingGuard

// Initialize logging with default configuration
// Uses default log directory from data directory
fn init_logging_default(data_dir: &PathBuf) -> LoggingGuard
```

---

## Helper Functions

```rust
// Get the default log filter based on build type
// Debug builds: `debug,tusk=trace,tokio_postgres=warn,hyper=warn`
// Release builds: `info,tusk=info,tokio_postgres=warn`
// Can be overridden by TUSK_LOG or RUST_LOG environment variables
fn default_log_filter() -> &'static str

// Get the log directory path
// Default: `{data_dir}/logs`
fn log_dir(data_dir: &PathBuf) -> PathBuf
```

---

## Log Levels by Build Type (FR-024)

| Build Type | Default Filter |
|------------|----------------|
| Debug | `debug,tusk=trace,tokio_postgres=warn,hyper=warn` |
| Release | `info,tusk=info,tokio_postgres=warn` |

### Environment Variable Override

Priority order:
1. `TUSK_LOG` environment variable
2. `RUST_LOG` environment variable
3. Build-type default

---

## Log File Configuration (FR-023)

| Setting | Value |
|---------|-------|
| Rotation | Daily |
| File naming | `tusk.YYYY-MM-DD.log` |
| Location | `{data_dir}/logs/` |
| Format | Text with timestamps |

---

## Fallback Behavior (FR-024a)

If file logging fails:

1. Log error to stderr
2. Continue with stdout-only logging
3. Don't block application startup

### User-Facing Message

> "Failed to initialize file logging: {error}. Using stdout."

---

## Implementation Notes

- Non-blocking file writes via `tracing_appender::non_blocking`
- Console output includes ANSI colors when stdout is a PTY
- Uses `MakeWriterExt::and()` to combine console and file writers
- WorkerGuard ensures pending logs are flushed on shutdown

---

## Performance Requirements

| Metric | Target | Source |
|--------|--------|--------|
| Log write latency | < 100ms | SC-007 |
