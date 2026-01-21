//! Structured logging setup with console and file output.
//!
//! Provides:
//! - Daily rotating log files (FR-023)
//! - Build-type conditional log levels (FR-024)
//! - Console-only fallback when file logging fails (FR-024a)
//! - Environment variable override via TUSK_LOG or RUST_LOG

use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;

/// Logging configuration.
pub struct LogConfig {
    /// Directory for log files
    pub log_dir: PathBuf,
    /// Whether running in a PTY (affects output formatting)
    pub is_pty: bool,
    /// Optional custom log filter
    pub log_filter: Option<String>,
}

impl LogConfig {
    /// Create a new logging configuration.
    pub fn new(log_dir: PathBuf) -> Self {
        Self { log_dir, is_pty: atty::is(atty::Stream::Stdout), log_filter: None }
    }

    /// Set custom log filter.
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.log_filter = Some(filter.into());
        self
    }
}

/// Guard that must be held for the lifetime of the application.
///
/// Dropping this guard flushes pending log entries.
pub struct LoggingGuard {
    _worker_guard: Option<WorkerGuard>,
}

/// Initialize logging with the given configuration (FR-022, FR-023, FR-024).
///
/// If file logging initialization fails, falls back to console-only (FR-024a).
pub fn init_logging(config: LogConfig) -> LoggingGuard {
    // If running in PTY (interactive terminal), use stdout-only logging
    if config.is_pty {
        return init_stdout_logging(config.log_filter.as_deref());
    }

    // Try to initialize file logging
    match init_file_logging(&config) {
        Ok(guard) => LoggingGuard { _worker_guard: Some(guard) },
        Err(e) => {
            eprintln!("Warning: Failed to initialize file logging: {}. Using console only.", e);
            init_stdout_logging(config.log_filter.as_deref())
        }
    }
}

/// Initialize with defaults (convenience function).
pub fn init_logging_default() -> LoggingGuard {
    let log_dir = log_dir();
    init_logging(LogConfig::new(log_dir))
}

/// Initialize stdout-only logging.
fn init_stdout_logging(filter: Option<&str>) -> LoggingGuard {
    let env_filter = build_env_filter(filter);

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_ansi(true)
        .with_target(false)
        .with_thread_ids(false)
        .init();

    LoggingGuard { _worker_guard: None }
}

/// Initialize file + console logging (FR-022, FR-023).
fn init_file_logging(config: &LogConfig) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    // Create log directory if needed
    std::fs::create_dir_all(&config.log_dir)?;

    // Create daily rotating file appender (FR-023)
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("tusk")
        .filename_suffix("log")
        .build(&config.log_dir)?;

    // Non-blocking writes (SC-007: <100ms latency)
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Combine stdout and file output
    let stdout = std::io::stdout.with_max_level(tracing::Level::INFO);
    let combined = stdout.and(non_blocking);

    let env_filter = build_env_filter(config.log_filter.as_deref());

    tracing_subscriber::fmt()
        .with_writer(combined)
        .with_env_filter(env_filter)
        .with_ansi(true)
        .with_target(true)
        .with_thread_ids(false)
        .init();

    Ok(guard)
}

/// Build the environment filter from config or defaults (FR-024).
fn build_env_filter(custom_filter: Option<&str>) -> EnvFilter {
    // Priority: custom filter > TUSK_LOG > RUST_LOG > default
    if let Some(filter) = custom_filter {
        return EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new(default_log_filter()));
    }

    EnvFilter::try_from_env("TUSK_LOG")
        .or_else(|_| EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| EnvFilter::new(default_log_filter()))
}

/// Get the default log filter based on build type (FR-024).
pub fn default_log_filter() -> &'static str {
    #[cfg(debug_assertions)]
    {
        "debug,tusk=trace,tusk_core=trace,tokio_postgres=warn,hyper=warn,reqwest=warn"
    }
    #[cfg(not(debug_assertions))]
    {
        "info,tusk=info,tusk_core=info,tokio_postgres=warn,hyper=warn,reqwest=warn"
    }
}

/// Get the default log directory.
pub fn log_dir() -> PathBuf {
    crate::services::storage::default_data_dir().join("logs")
}
