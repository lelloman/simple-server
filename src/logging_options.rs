use std::fmt;

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
