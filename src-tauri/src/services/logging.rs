use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Guard that must be kept alive for the duration of the application.
/// When dropped, it flushes any remaining log entries and shuts down
/// the non-blocking writer worker thread.
pub struct LogGuard {
    _guard: WorkerGuard,
}

/// Initialize the logging system with file-based daily rotation.
///
/// # Arguments
///
/// * `log_dir` - Directory where log files will be written
/// * `is_debug` - If true, enables debug-level logging and stdout output
///
/// # Returns
///
/// Returns a `LogGuard` that must be kept alive for the duration of the application.
/// Dropping this guard will flush any remaining log entries.
///
/// # Log File Location
///
/// Logs are written to `{log_dir}/tusk.YYYY-MM-DD.log` with daily rotation.
///
/// # Log Levels
///
/// - Debug builds: `debug` level by default
/// - Release builds: `info` level by default
/// - Overridable via `RUST_LOG` environment variable
pub fn init_logging(log_dir: &Path, is_debug: bool) -> LogGuard {
    // Create daily rotating file appender
    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("tusk")
        .filename_suffix("log")
        .build(log_dir)
        .expect("Failed to create log appender");

    // Create non-blocking writer to avoid slowing down the application
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Create environment filter with appropriate default level
    let default_level = if is_debug { "debug" } else { "info" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(default_level)
                .add_directive("tokio_postgres=info".parse().unwrap())
                .add_directive("hyper=info".parse().unwrap())
                .add_directive("mio=info".parse().unwrap())
        });

    // Create file logging layer
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    if is_debug {
        // In debug mode, also log to stdout with colors
        let stdout_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true)
            .with_target(true)
            .with_thread_ids(false);

        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .with(stdout_layer)
            .init();
    } else {
        // In release mode, only log to file
        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .init();
    }

    tracing::info!(
        "Logging initialized: dir={:?}, level={}",
        log_dir,
        default_level
    );

    LogGuard { _guard: guard }
}
