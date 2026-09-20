#![cfg(feature = "logging")]

use simple_server::logging::{AnsiMode, InitError, LogFormat, LogOutput, LoggingOptions, try_init};
use std::process::{Command, Output};

#[test]
fn logging_child() {
    let Ok(mode) = std::env::var("SIMPLE_SERVER_LOGGING_TEST") else {
        return;
    };
    let mut options = LoggingOptions::new("off,pilot=debug");
    options.ansi = AnsiMode::Never;
    match mode.as_str() {
        "json" => options.format = LogFormat::Json,
        "stdout" => options.output = LogOutput::Stdout,
        "auto" => options.ansi = AnsiMode::Auto,
        "ansi" => options.ansi = AnsiMode::Always,
        "no-target" => options.with_target = false,
        "off" => options.filter = "off".into(),
        "invalid-retry" => {
            for filter in ["", "   ", "secret[=broken"] {
                let error = try_init(LoggingOptions::new(filter)).unwrap_err();
                assert_eq!(error, InitError::InvalidFilter);
                assert_eq!(error.to_string(), "invalid logging filter");
            }
            let mut invalid = options.clone();
            invalid.format = LogFormat::Json;
            invalid.ansi = AnsiMode::Always;
            assert_eq!(try_init(invalid), Err(InitError::AnsiWithJson));
        }
        "existing" => {
            tracing::subscriber::set_global_default(tracing::subscriber::NoSubscriber::default())
                .unwrap();
            assert_eq!(try_init(options), Err(InitError::AlreadyInitialized));
            return;
        }
        "race" => {
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
            let threads: Vec<_> = (0..8)
                .map(|_| {
                    let barrier = barrier.clone();
                    let options = options.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        try_init(options)
                    })
                })
                .collect();
            let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
            assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
            assert_eq!(
                results
                    .iter()
                    .filter(|r| **r == Err(InitError::AlreadyInitialized))
                    .count(),
                7
            );
            return;
        }
        _ => {}
    }
    try_init(options).unwrap();
    // This must neither replace the subscriber nor install a log bridge.
    assert_eq!(
        try_init(LoggingOptions::new("off")),
        Err(InitError::AlreadyInitialized)
    );
    struct Logger;
    impl log::Log for Logger {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }
        fn log(&self, _: &log::Record<'_>) {}
        fn flush(&self) {}
    }
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).expect("shared setup must not claim the log facade");
    let outer = tracing::info_span!(target: "pilot", "outer", tenant = 7);
    let _outer = outer.enter();
    let inner = tracing::info_span!(target: "pilot", "inner", task = "check");
    let _inner = inner.enter();
    tracing::debug!(target: "pilot", count = 42, ready = true, level = "user-field", "visible-marker");
    tracing::trace!(target: "pilot", "hidden-trace-marker");
    tracing::error!(target: "other", "hidden-target-marker");
}

fn run(mode: &str) -> Output {
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "logging_child", "--nocapture"])
        .env("SIMPLE_SERVER_LOGGING_TEST", mode)
        .env("RUST_LOG", "off")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    result
}

#[test]
fn filtering_destination_styling_and_initialization() {
    for mode in [
        "text",
        "stdout",
        "auto",
        "ansi",
        "no-target",
        "invalid-retry",
    ] {
        let output = run(mode);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let (events, other) = if mode == "stdout" {
            (&stdout, &stderr)
        } else {
            (&stderr, &stdout)
        };
        assert!(events.contains("visible-marker"), "{mode}: {events}");
        assert!(!other.contains("visible-marker"));
        assert!(!events.contains("hidden-"));
        assert_eq!(events.contains("\x1b["), mode == "ansi");
        assert_eq!(events.contains("pilot"), mode != "no-target");
        assert!(events.contains("outer") && events.contains("inner"));
    }
    for mode in ["off", "existing", "race"] {
        let output = run(mode);
        assert!(output.stderr.is_empty(), "{output:?}");
        assert!(
            !String::from_utf8(output.stdout)
                .unwrap()
                .contains("visible-marker")
        );
    }
}

#[test]
fn json_preserves_types_metadata_and_nested_span_context() {
    let output = run("json");
    let text = String::from_utf8(output.stderr).unwrap();
    assert!(!text.contains('\x1b'));
    let events: Vec<serde_json::Value> = text
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event["level"], "DEBUG");
    assert_eq!(event["target"], "pilot");
    assert_eq!(event["fields"]["level"], "user-field");
    assert_eq!(event["fields"]["count"], 42);
    assert_eq!(event["fields"]["ready"], true);
    assert_eq!(event["fields"]["message"], "visible-marker");
    assert_eq!(event["span"]["name"], "inner");
    assert_eq!(event["spans"][0]["tenant"], 7);
    assert_eq!(event["spans"][1]["task"], "check");
    let timestamp = event["timestamp"].as_str().unwrap();
    assert_eq!(timestamp.len(), 27);
    assert_eq!(&timestamp[10..11], "T");
    assert_eq!(&timestamp[19..20], ".");
    assert!(timestamp.ends_with('Z'));
}
