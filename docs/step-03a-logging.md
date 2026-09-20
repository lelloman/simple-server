# Step 03a: logging setup

Status: implemented locally, with Favzetto and Crumbles canaries. Part of
[Step 03: observability](step-03-observability.md).

## Outcome and ownership

Provide an optional `logging` module that configures and explicitly installs a
process-wide tracing subscriber. Applications retain `main()`, configuration
loading, initialization order, and their logging statements.

The `logging` feature is opt-in. It must work with default features disabled,
without Axum, lifecycle helpers, or a Tokio runtime. Enabling a feature or
constructing options performs no global initialization.

## Evidence from consumers

Reviewed local source on 2026-09-20:

| Consumer | Existing behavior | Migration requirement |
| --- | --- | --- |
| Favzetto, `backend/src/observability/mod.rs` | Parses a caller-supplied filter; invalid input falls back to `info,favzetto_backend=debug`; text output includes targets | Preserve caller configuration, fallback behavior, output destination, and target display in the pilot |
| Pezzottify, `pezzottify-server/src/main.rs` | Reads `LOG_LEVEL`, with an INFO default and lossy filter parsing | Do not impose a different environment variable or parsing policy during later adoption |
| Crumbles, `crumbles/src/main.rs` | Reads the default tracing filter environment and adds an INFO directive | Preserve effective filtering when this service adopts 03a |

This evidence informs the API; only Favzetto is the initial implementation pilot.

## Public surface

Use `simple_server::logging` with these types. `LoggingOptions::new(filter)`
requires a filter; its other fields can be set explicitly before initialization.

| Concept | Purpose |
| --- | --- |
| `LoggingOptions` | Explicit filter string, output format, destination, ANSI policy, and target display |
| `LogFormat::{Text, Json}` | Human-readable text or one JSON object per event |
| `LogOutput::{Stdout, Stderr}` | Explicit output destination |
| `AnsiMode::{Auto, Always, Never}` | Text styling policy; automatic mode checks the selected output stream |
| `try_init(options)` | Validate options and install once, returning a structured error on failure |

Require an explicit filter. Defaults for the other options are text,
stderr, automatic ANSI, and visible targets. Consumer migrations must select
their existing destination explicitly rather than inheriting a changed default.
JSON output never contains ANSI styling; explicitly requesting JSON with
`AnsiMode::Always` is a configuration error.

## Behavioral contract

### Configuration and filtering

The library takes no configuration from environment variables and mutates none. Applications
resolve environment, CLI, and file configuration before calling the initializer.
Filtering supports tracing target/level directives, including `off`.

Malformed filter syntax returns `InvalidFilter` before installation. There is
no hidden fallback and no implicit additional INFO directive. Applications can
implement their existing fallback by handling that error and retrying with an
explicit fallback filter. Empty input is rejected; callers must choose a level
or `off` explicitly. Error messages must not echo the supplied filter string.

### Initialization and composition

The initializer installs at most one global subscriber. An already installed
subscriber produces `AlreadyInitialized`; it is not replaced, silently ignored,
or treated as successful adoption. Installation races return an error rather
than panic. Validation happens before any global subscriber mutation.

No library function exits the process or creates a runtime. The application
decides whether an initialization error is fatal and can report it directly to
stderr before logging exists.

Applications with custom subscriber layers can keep their existing subscriber
and later adopt 03b/03c independently. Arbitrary subscriber assembly, dynamic
filter reload, and automatic bridging of the `log` facade are outside 03a.
Implementation must avoid accidental global logger installation through helper
defaults. Any consumer that already depends on a log bridge needs an explicit
compatibility check before migration.

### Output

Text and JSON output include event level, timestamp, message, and event fields;
target display follows `with_target`. Timestamps are UTC RFC 3339 with six
fractional digits. JSON uses `timestamp`, `level`, optional `target`, and nested
`fields` (including `message`); active context appears under `span` and the
outer-to-inner `spans` array. Structured numeric and boolean fields retain their
types. Application fields cannot overwrite top-level metadata. This is the
tracing-subscriber formatter's layout, not a separately versioned ingestion schema.

ANSI is set explicitly after constructing the formatter, overriding its internal
`NO_COLOR` default. Applications that honor `NO_COLOR` must resolve it themselves.

03a writes synchronously to the selected standard stream. It creates no
background writer, log files, rotation policy, or shutdown flush task. It does
not claim durable delivery if the process or destination fails.

The module formats application events as supplied; it does not redact arbitrary
application fields. Safe automatic HTTP fields belong to 03c.

## Acceptance checks

Library checks must cover:

- `logging` alone builds and runs with default features disabled; existing
  feature combinations and default behavior remain valid.
- Filtering includes/excludes events by level and target; `off` suppresses them.
- Malformed and empty filters fail without installing a subscriber; a valid
  retry succeeds.
- Text and JSON output, destination selection, target display, ANSI policy,
  structured values, and nested span context meet the documented contract.
- Existing subscriber installation and repeated initialization return errors
  without replacing the original subscriber or panicking.
- Global installation checks run in separate processes so test ordering cannot
  disguise failures or contaminate other tests.

The Favzetto pilot must preserve its logging configuration source, invalid-filter
fallback, output destination, targets, and startup/CLI behavior. Validate the
same representative events and filtering before and after adoption. Run its
relevant checks and a real-process startup/shutdown smoke test. Request IDs,
HTTP middleware, and domain/audit events are unchanged by this pilot.

Record the reviewed library revision, consumer commit, verification results,
and any compatibility limits before marking Favzetto's 03a adoption complete.
The migration matrix must keep 03b and 03c pending even after 03a is implemented.

## Pilot compatibility details

Favzetto explicitly selects stdout, text, targets, and its historical `NO_COLOR`
policy (colors otherwise remain enabled even when redirected). An exactly empty
filter maps to `off`; whitespace-only or malformed input retains its fallback
`info,favzetto_backend=debug`. It explicitly installs `tracing-log` with the
subscriber's maximum level after initialization, preserving the previous bridge.
The shared module never installs that bridge, including under Cargo feature
unification. Initialization errors are returned and reported to stderr by main.

The process comparison runs the old initializer and new adapter in separate
processes and compares events after removing timestamps. It covers levels,
targets, fields, invalid/empty filters, destination, ANSI, and log-facade records.

## Verification (2026-09-20)

- Library: all-feature tests (20 passed), logging-only tests (three passed),
  no-default-feature tests, strict all-feature and logging-only Clippy, rustfmt,
  and the standalone JSON example passed.
- The logging-only normal dependency graph contains no Axum, Tokio, or log bridge.
- Favzetto: 131 unit tests and 93 API tests pass. The same two API tests fail on
  the untouched baseline and migrated suite: runtime-bridge approval/rejection
  of terminal flows. Both lifecycle process tests and both logging process tests
  pass after the final adapter fixes.
- Favzetto strict Clippy retains the documented 53 library / 55 test errors in
  existing code. No release image, browser suite, deployment, or push is claimed.

## Second canary: Crumbles

Both the text CLI/server and JSON integration daemon use the shared initializer.
They retain their own `EnvFilter` parsing before passing normalized directives:
lossy environment parsing with an added INFO directive for the main server, and
strict parsing with an INFO fallback for the daemon. Both preserve stdout and
explicit log bridges; the main server retains its `NO_COLOR` policy.

Fresh-process comparisons against the former initializers cover global, target,
and span-field filters, invalid and missing environment values, ANSI, structured
fields, JSON span context, and stderr diagnostics. No shared API changes were
needed. This confirms an application can retain its existing filter parser while
adopting shared subscriber setup. See the [adoption record](migration-status.md)
for the consumer revision and final verification.
