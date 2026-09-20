//! Explicit, process-wide logging setup, independent of HTTP and Tokio.
//!
//! Applications supply configuration and decide how to handle errors. This
//! module does not install a `log` facade bridge or replace existing subscribers.

use std::{fmt, io::IsTerminal};

use tracing_subscriber::{EnvFilter, fmt::writer::BoxMakeWriter};

/// Event output encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Human-readable tracing events.
    #[default]
    Text,
    /// Multiline human-readable events with source locations and span context.
    Pretty,
    /// One JSON object per event, with nested fields and span context.
    Json,
}

/// Destination for synchronously written events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogOutput {
    /// Standard output.
    Stdout,
    /// Standard error.
    #[default]
    Stderr,
}

/// Styling for text output. JSON always disables ANSI styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnsiMode {
    /// Enable styling only if the selected stream is a terminal.
    #[default]
    Auto,
    /// Enable styling even for redirected output; invalid with JSON.
    Always,
    /// Disable styling.
    Never,
}

/// Explicit logging configuration. No environment-based policy is applied.
#[derive(Debug, Clone)]
pub struct LoggingOptions {
    /// Tracing filter directives. Empty or malformed input is rejected.
    pub filter: String,
    /// Text, pretty text, or JSON event encoding.
    pub format: LogFormat,
    /// Destination for events.
    pub output: LogOutput,
    /// Styling policy for text events.
    pub ansi: AnsiMode,
    /// Include event targets in the output.
    pub with_target: bool,
}

impl LoggingOptions {
    /// Require a filter; default to text on stderr, automatic ANSI, and targets.
    pub fn new(filter: impl Into<String>) -> Self {
        Self {
            filter: filter.into(),
            format: LogFormat::Text,
            output: LogOutput::Stderr,
            ansi: AnsiMode::Auto,
            with_target: true,
        }
    }
}

/// Initialization errors contain no supplied filter text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitError {
    /// Empty or malformed tracing filter syntax.
    InvalidFilter,
    /// ANSI styling was explicitly requested for JSON output.
    AnsiWithJson,
    /// Another global tracing subscriber is already installed.
    AlreadyInitialized,
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidFilter => "invalid logging filter",
            Self::AnsiWithJson => "JSON logging cannot use ANSI styling",
            Self::AlreadyInitialized => "a global tracing subscriber is already installed",
        })
    }
}

impl std::error::Error for InitError {}

/// Validate all options, then install a global subscriber exactly once.
///
/// Errors leave an existing subscriber untouched. Invalid configuration does
/// not install a subscriber, so applications may retry with explicit defaults.
/// Timestamps use UTC RFC 3339 with microsecond precision. JSON keeps event
/// fields under `fields`, and includes `span` and `spans` for active context.
/// No runtime, background writer, environment configuration, or log bridge is
/// installed. ANSI settings override the formatter's environment-based default.
pub fn try_init(options: LoggingOptions) -> Result<(), InitError> {
    if options.filter.trim().is_empty() {
        return Err(InitError::InvalidFilter);
    }
    let filter = EnvFilter::try_new(&options.filter).map_err(|_| InitError::InvalidFilter)?;
    if options.format == LogFormat::Json && options.ansi == AnsiMode::Always {
        return Err(InitError::AnsiWithJson);
    }
    let ansi = options.format != LogFormat::Json
        && match options.ansi {
            AnsiMode::Always => true,
            AnsiMode::Never => false,
            AnsiMode::Auto => match options.output {
                LogOutput::Stdout => std::io::stdout().is_terminal(),
                LogOutput::Stderr => std::io::stderr().is_terminal(),
            },
        };
    let writer = match options.output {
        LogOutput::Stdout => BoxMakeWriter::new(std::io::stdout),
        LogOutput::Stderr => BoxMakeWriter::new(std::io::stderr),
    };
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_target(options.with_target)
        .with_ansi(ansi);
    // Avoid SubscriberInitExt: feature unification could otherwise install a
    // global log bridge even when this crate did not request that capability.
    match options.format {
        LogFormat::Text => tracing::subscriber::set_global_default(builder.finish()),
        LogFormat::Pretty => tracing::subscriber::set_global_default(builder.pretty().finish()),
        LogFormat::Json => tracing::subscriber::set_global_default(
            builder
                .json()
                .flatten_event(false)
                .with_current_span(true)
                .with_span_list(true)
                .finish(),
        ),
    }
    .map_err(|_| InitError::AlreadyInitialized)
}
