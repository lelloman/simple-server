#![cfg(all(feature = "engine-logging", feature = "logging"))]
use native::{AnsiMode, FilterMode, LogFormat, LogOutput, LoggingOptions, SpanEvents};
use simple_server::{engine_logging as native, logging};
use std::process::Command;

#[test]
fn child() {
    let Ok(backend) = std::env::var("ENGINE_LOGGING_CHILD") else {
        return;
    };
    let mode = std::env::var("ENGINE_LOGGING_MODE").unwrap();
    let mut options = LoggingOptions::new("trace");
    options.ansi = AnsiMode::Never;
    options.output = LogOutput::Stdout;
    options.format = if mode == "text" {
        LogFormat::Text
    } else {
        LogFormat::Json
    };
    if mode == "lifecycle" {
        options.span_events = SpanEvents::Full;
    }
    if mode == "retry" {
        for filter in ["", "secret[=broken"] {
            let error =
                native::try_init(LoggingOptions::new(filter), FilterMode::Strict).unwrap_err();
            assert_eq!(error.to_string(), "invalid logging filter");
            assert!(!error.to_string().contains("secret"));
        }
        let mut invalid = options.clone();
        invalid.ansi = AnsiMode::Always;
        assert!(matches!(
            native::try_init(invalid, FilterMode::Strict),
            Err(native::Error::Configuration(
                native::InitError::AnsiWithJson
            ))
        ));
    }
    if mode == "local-before" {
        tracing::subscriber::with_default(tracing::subscriber::NoSubscriber::default(), || {
            tracing::info!("local discarded")
        });
    }
    if mode == "existing" {
        tracing::subscriber::set_global_default(tracing::subscriber::NoSubscriber::default())
            .unwrap();
        for _ in 0..3 {
            assert!(matches!(
                native::try_init(options.clone(), FilterMode::Strict),
                Err(native::Error::Configuration(
                    native::InitError::AlreadyInitialized
                ))
            ));
        }
        return;
    }
    if mode == "race" {
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let barrier = barrier.clone();
                let options = options.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    native::try_init(options, FilterMode::Strict)
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        return;
    }
    if backend == "engine" {
        native::try_init(options.clone(), FilterMode::Strict).unwrap();
        native::init_log_bridge().unwrap();
    } else {
        logging::try_init(options).unwrap();
        tracing_log::LogTracer::init().unwrap();
    }
    if mode == "retry" || mode == "local-before" {
        tracing::info!("bridge-installed");
        return;
    }
    if mode == "threads" {
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let threads: Vec<_> = (0..2)
            .map(|worker| {
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let span = tracing::info_span!("bridge-worker", worker);
                    let _entered = span.enter();
                    barrier.wait();
                    tracing::info!("bridge-thread-event");
                    assert_eq!(tracing::Span::current().id(), span.id());
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        return;
    }
    let parent = tracing::info_span!("bridge-parent", tenant = 7, changed = tracing::field::Empty);
    parent.record("changed", 8);
    let child = tracing::info_span!(parent:&parent,"bridge-child",ready=true);
    child.follows_from(&parent);
    {
        let _entered = parent.enter();
        assert_eq!(tracing::Span::current().id(), parent.id());
        tracing::info!(answer=42_i64, unsigned=43_u64, fraction=1.25, flag=true, text="quoted\ntext", debug=?vec![1,2], wide=1_u128<<80, signed=-(1_i128<<80), nan=f64::NAN,"bridge-typed");
        let error = std::io::Error::other("inner error");
        tracing::info!(
            error = &error as &(dyn std::error::Error + 'static),
            "bridge-error"
        );
        tracing::info!(parent:None,"bridge-root-event");
        tracing::info!(parent:&child,"bridge-explicit-event");
        tracing_log::log::warn!(target:"dependency","bridge-facade");
    }
    drop(parent); // The native registry retains the parent for its child.
    let cloned = child.clone();
    drop(child);
    {
        let _entered = cloned.enter();
        assert_eq!(tracing::Span::current().id(), cloned.id());
        tracing::info!("bridge-retained-parent");
    }
    drop(cloned);
    assert!(tracing::Span::current().is_none());
    tracing::info!("bridge-after-close");
}
fn run(backend: &str, mode: &str) -> Vec<String> {
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child", "--nocapture"])
        .env("ENGINE_LOGGING_CHILD", backend)
        .env("ENGINE_LOGGING_MODE", mode)
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
    let text = String::from_utf8(result.stdout).unwrap();
    let mut lines: Vec<_> = text
        .lines()
        .filter(|line| line.contains("bridge-"))
        .map(|line| {
            if mode == "text" {
                line.split_once(' ').unwrap().1.to_owned()
            } else {
                let mut v: serde_json::Value = serde_json::from_str(line).unwrap();
                v.as_object_mut().unwrap().remove("timestamp");
                if let Some(fields) = v["fields"].as_object_mut() {
                    fields.remove("time.busy");
                    fields.remove("time.idle");
                }
                v.to_string()
            }
        })
        .collect();
    if mode == "threads" {
        lines.sort();
    }
    lines
}
#[test]
fn matches_reference_for_values_parents_clones_and_span_lifecycle() {
    for mode in ["text", "json", "lifecycle", "threads"] {
        let expected = run("source", mode);
        assert!(!expected.is_empty());
        assert_eq!(run("engine", mode), expected, "mode={mode}");
    }
}
#[test]
fn invalid_configuration_can_retry_and_existing_global_is_preserved() {
    for mode in ["retry", "local-before", "existing", "race"] {
        run("engine", mode);
    }
}
