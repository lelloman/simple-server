//! Explicit, process-wide logging setup, independent of HTTP and Tokio.
//!
//! Applications supply configuration and decide how to handle errors. This
//! module does not install a `log` facade bridge or replace existing subscribers.

use std::{fmt, io::IsTerminal};

use tracing_subscriber::{
    EnvFilter, Registry, fmt::format::FmtSpan, fmt::writer::BoxMakeWriter, layer::SubscriberExt,
    reload,
};

/// Event output encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Human-readable tracing events.
    #[default]
    Text,
    /// Multiline human-readable events with source locations and span context.
    Pretty,
    /// Single-line events with compact span context.
    Compact,
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

/// Synthetic events emitted for span lifecycle changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpanEvents {
    /// Emit only application events (the default).
    #[default]
    None,
    /// Emit an event when the final reference to a span is dropped.
    Close,
    /// Emit new, enter, exit, and close events.
    Full,
}

impl SpanEvents {
    fn formatter(self) -> FmtSpan {
        match self {
            Self::None => FmtSpan::NONE,
            Self::Close => FmtSpan::CLOSE,
            Self::Full => FmtSpan::FULL,
        }
    }
}

/// Explicit logging configuration. No environment-based policy is applied.
#[derive(Debug, Clone)]
pub struct LoggingOptions {
    /// Tracing filter directives. Malformed input is rejected; the one-shot
    /// initializer additionally rejects empty input.
    pub filter: String,
    /// Text, pretty, compact, or JSON event encoding.
    pub format: LogFormat,
    /// Destination for events.
    pub output: LogOutput,
    /// Styling policy for text events.
    pub ansi: AnsiMode,
    /// Include event targets in the output.
    pub with_target: bool,
    /// Optional synthetic span events. Close events include busy/idle timing.
    pub span_events: SpanEvents,
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
            span_events: SpanEvents::None,
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
    let (filter, writer, ansi) = prepare(&options, false)?;
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_target(options.with_target)
        .with_ansi(ansi)
        .with_span_events(options.span_events.formatter());
    // Avoid SubscriberInitExt: feature unification could otherwise install a
    // global log bridge even when this crate did not request that capability.
    match options.format {
        LogFormat::Text => tracing::subscriber::set_global_default(builder.finish()),
        LogFormat::Pretty => tracing::subscriber::set_global_default(builder.pretty().finish()),
        LogFormat::Compact => tracing::subscriber::set_global_default(builder.compact().finish()),
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

fn prepare(
    options: &LoggingOptions,
    allow_empty: bool,
) -> Result<(EnvFilter, BoxMakeWriter, bool), InitError> {
    if !allow_empty && options.filter.trim().is_empty() {
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
    Ok((filter, writer, ansi))
}

/// A filter update or inspection failed. Supplied directives are never included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadError {
    /// The proposed filter could not be parsed; the current filter is unchanged.
    InvalidFilter,
    /// The subscriber is unavailable or its filter lock is poisoned.
    Unavailable,
}

impl fmt::Display for ReloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidFilter => "invalid logging filter",
            Self::Unavailable => "logging filter is unavailable",
        })
    }
}

impl std::error::Error for ReloadError {}

/// Cloneable, thread-safe control of the installed subscriber's filter.
///
/// Dropping handles does not disable logging. Updates rebuild tracing callsite
/// interest, including callsites previously disabled by the old filter.
#[derive(Debug, Clone)]
pub struct ReloadHandle {
    inner: reload::Handle<EnvFilter, Registry>,
}

impl ReloadHandle {
    /// Replace the filter atomically after strict parsing. Empty
    /// directives disable events, matching `EnvFilter::try_new` semantics.
    pub fn set_filter(&self, filter: &str) -> Result<(), ReloadError> {
        let filter = EnvFilter::try_new(filter).map_err(|_| ReloadError::InvalidFilter)?;
        self.inner
            .reload(filter)
            .map_err(|_| ReloadError::Unavailable)
    }

    /// Return the normalized active directives (empty for an empty filter).
    pub fn current_filter(&self) -> Result<String, ReloadError> {
        self.inner
            .with_current(ToString::to_string)
            .map_err(|_| ReloadError::Unavailable)
    }
}

/// Install an explicitly reloadable global subscriber and return its handle.
///
/// Formatting, destination, and span-event settings are fixed at initialization.
/// Unlike [`try_init`], this accepts empty directives as an explicit
/// empty filter, enabling callers to round-trip that state through runtime APIs.
/// Malformed directives are rejected. No log bridge is installed; applications
/// that bridge `log` must retain their bridge's filter-update policy.
pub fn try_init_reloadable(options: LoggingOptions) -> Result<ReloadHandle, InitError> {
    let (filter, writer, ansi) = prepare(&options, true)?;
    let (filter_layer, inner) = reload::Layer::new(filter);
    let registry = Registry::default().with(filter_layer);
    let layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_target(options.with_target)
        .with_ansi(ansi)
        .with_span_events(options.span_events.formatter());
    match options.format {
        LogFormat::Text => tracing::subscriber::set_global_default(registry.with(layer)),
        LogFormat::Pretty => tracing::subscriber::set_global_default(registry.with(layer.pretty())),
        LogFormat::Compact => {
            tracing::subscriber::set_global_default(registry.with(layer.compact()))
        }
        LogFormat::Json => tracing::subscriber::set_global_default(
            registry.with(
                layer
                    .json()
                    .flatten_event(false)
                    .with_current_span(true)
                    .with_span_list(true),
            ),
        ),
    }
    .map_err(|_| InitError::AlreadyInitialized)?;
    Ok(ReloadHandle { inner })
}
