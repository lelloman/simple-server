#![cfg(feature = "logging")]

use simple_server::logging::{
    AnsiMode, InitError, LogFormat, LoggingOptions, ReloadError, SpanEvents, try_init,
    try_init_reloadable,
};
use std::process::Command;

fn event(phase: &str) {
    tracing::debug!(target: "reload_test", phase, "reload-marker");
}

#[test]
fn child() {
    let Ok(mode) = std::env::var("RELOAD_TEST_CHILD") else {
        return;
    };
    let mut options = LoggingOptions::new("warn");
    options.ansi = AnsiMode::Never;
    options.format = match mode.as_str() {
        "json" => LogFormat::Json,
        "pretty" => LogFormat::Pretty,
        "compact" => LogFormat::Compact,
        _ => LogFormat::Text,
    };
    options.span_events = if mode == "full" {
        SpanEvents::Full
    } else {
        SpanEvents::Close
    };
    if mode == "static" {
        options.filter = "info".into();
        try_init(options).unwrap();
    } else {
        let mut invalid = options.clone();
        invalid.filter = "secret[broken".into();
        assert_eq!(
            try_init_reloadable(invalid).unwrap_err(),
            InitError::InvalidFilter
        );
        let mut invalid = options.clone();
        invalid.format = LogFormat::Json;
        invalid.ansi = AnsiMode::Always;
        assert_eq!(
            try_init_reloadable(invalid).unwrap_err(),
            InitError::AnsiWithJson
        );
        let handle = try_init_reloadable(options.clone()).unwrap();
        assert_eq!(
            try_init_reloadable(options).unwrap_err(),
            InitError::AlreadyInitialized
        );
        assert_eq!(handle.current_filter().unwrap(), "warn");
        event("hidden-before");
        handle.set_filter("reload_test=debug,info").unwrap();
        event("visible-after");
        let old = handle.current_filter().unwrap();
        assert_eq!(
            handle.set_filter("secret[broken"),
            Err(ReloadError::InvalidFilter)
        );
        assert_eq!(handle.current_filter().unwrap(), old);
        event("visible-after-invalid");
        handle.set_filter("").unwrap();
        assert_eq!(handle.current_filter().unwrap(), "");
        event("hidden-empty");
        assert_eq!(handle.set_filter("   "), Err(ReloadError::InvalidFilter));
        assert_eq!(handle.current_filter().unwrap(), "");
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let handle = handle.clone();
                std::thread::spawn(move || {
                    for _ in 0..10 {
                        handle.set_filter("debug").unwrap();
                        assert!(matches!(
                            handle.current_filter().unwrap().as_str(),
                            "info" | "debug"
                        ));
                        handle.set_filter("info").unwrap();
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        handle.set_filter("info").unwrap();
        drop(handle);
    }
    // Neither initializer may claim the application's global log bridge.
    struct Logger;
    impl log::Log for Logger {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }
        fn log(&self, _: &log::Record<'_>) {}
        fn flush(&self) {}
    }
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).unwrap();
    let span = tracing::info_span!(target: "reload_test", "span-marker", tenant = 42);
    {
        let _entered = span.enter();
        tracing::info!("span-body-marker");
    }
    drop(span);
}

#[test]
fn reload_and_span_events_work_in_each_formatter() {
    for mode in ["text", "pretty", "compact", "json", "full", "static"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "child", "--nocapture"])
            .env("RELOAD_TEST_CHILD", mode)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        let events = String::from_utf8(output.stderr).unwrap();
        assert!(!events.contains("hidden-"), "{mode}: {events}");
        if mode != "static" {
            assert!(events.contains("visible-after"), "{events}");
            assert!(events.contains("visible-after-invalid"), "{events}");
        }
        assert!(
            events.contains("close") && events.contains("span-marker"),
            "{events}"
        );
        assert!(
            events.contains("time.busy") && events.contains("time.idle"),
            "{events}"
        );
        assert!(!events.contains('\x1b'));
        if mode == "json" {
            for line in events.lines() {
                serde_json::from_str::<serde_json::Value>(line).unwrap();
            }
        }
        if mode == "full" {
            for lifecycle in ["new", "enter", "exit", "close"] {
                assert!(events.contains(lifecycle), "{events}");
            }
        }
    }
}
