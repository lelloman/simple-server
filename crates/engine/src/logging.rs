//! Native logging subscriber instances. Host calls preserve caller-thread context.
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fmt,
    io::IsTerminal,
    sync::{Arc, Mutex, OnceLock},
};
use tracing::{
    Metadata, Subscriber,
    field::{FieldSet, Value as TraceValue},
    span::{Attributes, Id, Record},
    subscriber::Interest,
};
use tracing_core::{
    callsite::{Callsite, Identifier},
    metadata::Kind,
};
use tracing_subscriber::{
    EnvFilter,
    fmt::{format::FmtSpan, writer::BoxMakeWriter},
};
pub const LOGGER: u32 = 21;
struct Site(OnceLock<&'static Metadata<'static>>);
impl Callsite for Site {
    fn set_interest(&self, _: Interest) {}
    fn metadata(&self) -> &'static Metadata<'static> {
        self.0.get().copied().expect("initialized metadata")
    }
}
#[derive(Clone, Copy)]
struct Meta {
    metadata: &'static Metadata<'static>,
    names: &'static [&'static str],
}
struct Logger {
    dispatch: tracing::Dispatch,
    metadata: Mutex<HashMap<u64, Meta>>,
}
static LOGGERS: OnceLock<Mutex<HashMap<u64, Arc<Logger>>>> = OnceLock::new();
fn loggers() -> &'static Mutex<HashMap<u64, Arc<Logger>>> {
    LOGGERS.get_or_init(Default::default)
}
pub fn release(id: u64) {
    let logger = loggers().lock().unwrap().remove(&id);
    drop(logger);
}
fn output(v: Value) -> Vec<u8> {
    crate::operations::encode(v, &[])
}
fn leak(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str().ok_or_else(|| format!("invalid logging {k}"))
}
fn id(v: &Value, k: &str) -> Result<u64, String> {
    v[k].as_u64()
        .filter(|v| *v != 0)
        .ok_or_else(|| format!("invalid logging {k}"))
}
fn metadata<'a>(
    v: &'a Value,
    name: &'static str,
    names: &'static [&'static str],
    site: Identifier,
) -> Result<Metadata<'a>, String> {
    Ok(Metadata::new(
        name,
        text(v, "target")?,
        text(v, "level")?
            .parse()
            .map_err(|_| "invalid logging level")?,
        v["file"].as_str(),
        v["line"].as_u64().map(|v| v as u32),
        v["module"].as_str(),
        FieldSet::new(names, site),
        if v["span"] == true {
            Kind::SPAN
        } else {
            Kind::EVENT
        },
    ))
}
fn new(v: &Value) -> Result<Vec<u8>, String> {
    let raw = text(v, "filter")?;
    let filter = match text(v, "mode")? {
        "Strict" if !raw.trim().is_empty() => EnvFilter::try_new(raw).ok(),
        "Lossy" => Some(EnvFilter::new(raw)),
        "StrictOrInfo" => Some(EnvFilter::try_new(raw).unwrap_or_else(|_| EnvFilter::new("info"))),
        _ => None,
    };
    let Some(filter) = filter else {
        return Ok(output(json!({"error":"filter"})));
    };
    let json_mode = v["format"] == "Json";
    if json_mode && v["ansi"] == "Always" {
        return Ok(output(json!({"error":"ansi"})));
    }
    let stderr = v["output"] == "Stderr";
    let ansi = !json_mode
        && match text(v, "ansi")? {
            "Always" => true,
            "Never" => false,
            _ => {
                if stderr {
                    std::io::stderr().is_terminal()
                } else {
                    std::io::stdout().is_terminal()
                }
            }
        };
    let writer = if stderr {
        BoxMakeWriter::new(std::io::stderr)
    } else {
        BoxMakeWriter::new(std::io::stdout)
    };
    let spans = match text(v, "span_events")? {
        "Close" => FmtSpan::CLOSE,
        "Full" => FmtSpan::FULL,
        _ => FmtSpan::NONE,
    };
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_target(v["target"] == true)
        .with_ansi(ansi)
        .with_span_events(spans);
    let subscriber: Box<dyn Subscriber + Send + Sync> = match text(v, "format")? {
        "Json" => Box::new(
            builder
                .json()
                .flatten_event(false)
                .with_current_span(true)
                .with_span_list(true)
                .finish(),
        ),
        "Pretty" => Box::new(builder.pretty().finish()),
        "Compact" => Box::new(builder.compact().finish()),
        _ => Box::new(builder.finish()),
    };
    let max = subscriber
        .max_level_hint()
        .unwrap_or(tracing::level_filters::LevelFilter::TRACE)
        .to_string();
    let logger = Arc::new(Logger {
        dispatch: tracing::Dispatch::new(subscriber),
        metadata: Mutex::new(HashMap::new()),
    });
    let id = crate::operations::id();
    loggers().lock().unwrap().insert(id, logger);
    Ok(output(json!({"id":id,"max":max})))
}
#[derive(Debug)]
struct ErrorChain {
    message: String,
    source: Option<Box<ErrorChain>>,
}
impl fmt::Display for ErrorChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ErrorChain {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref().map(|s| s as _)
    }
}
struct RawDebug(String);
impl fmt::Debug for RawDebug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
type Fields = Vec<(String, Box<dyn TraceValue>)>;
fn values(v: &Value) -> Result<Fields, String> {
    v.as_array()
        .ok_or("invalid logging fields")?
        .iter()
        .map(|f| {
            let name = f[0]
                .as_str()
                .ok_or("invalid logging field name")?
                .to_owned();
            let value = &f[2];
            let bad = || "invalid logging field value".to_owned();
            let value: Box<dyn TraceValue> = match f[1].as_str() {
                Some("str") => Box::new(value.as_str().ok_or_else(bad)?.to_owned()),
                Some("bool") => Box::new(value.as_bool().ok_or_else(bad)?),
                Some("i64") => Box::new(value.as_i64().ok_or_else(bad)?),
                Some("u64") => Box::new(value.as_u64().ok_or_else(bad)?),
                Some("i128") => Box::new(
                    value
                        .as_str()
                        .ok_or_else(bad)?
                        .parse::<i128>()
                        .map_err(|_| bad())?,
                ),
                Some("u128") => Box::new(
                    value
                        .as_str()
                        .ok_or_else(bad)?
                        .parse::<u128>()
                        .map_err(|_| bad())?,
                ),
                Some("f64") => Box::new(f64::from_bits(value.as_u64().ok_or_else(bad)?)),
                Some("debug") => Box::new(tracing::field::debug(RawDebug(
                    value.as_str().ok_or_else(bad)?.to_owned(),
                ))),
                Some("error") => {
                    let mut chain = None;
                    for message in value.as_array().ok_or_else(bad)?.iter().rev() {
                        chain = Some(Box::new(ErrorChain {
                            message: message.as_str().ok_or_else(bad)?.to_owned(),
                            source: chain,
                        }));
                    }
                    // Error values retain their native Visit::record_error semantics.
                    {
                        let error: Box<dyn std::error::Error> = chain.ok_or_else(bad)?;
                        Box::new(error)
                    }
                }
                _ => return Err(bad()),
            };
            Ok((name, value))
        })
        .collect()
}
fn intern_name(name: &str) -> &'static str {
    static NAMES: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    NAMES
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .entry(name.to_owned())
        .or_insert_with(|| leak(name))
}
fn dummy() -> Identifier {
    static SITE: Site = Site(OnceLock::new());
    SITE.0.get_or_init(|| {
        Box::leak(Box::new(Metadata::new(
            "enabled",
            "",
            tracing::Level::TRACE,
            None,
            None,
            None,
            FieldSet::new(&[], Identifier(&SITE)),
            Kind::EVENT,
        )))
    });
    Identifier(&SITE)
}
pub fn execute(v: &Value) -> Result<Vec<u8>, String> {
    let op = text(v, "op")?;
    if op == "logging_new" {
        return new(v);
    }
    let logger = loggers()
        .lock()
        .unwrap()
        .get(&id(v, "logger")?)
        .cloned()
        .ok_or("unknown logger")?;
    if op == "logging_metadata" {
        let key = id(v, "id")?;
        let mut metas = logger.metadata.lock().unwrap();
        if metas.contains_key(&key) {
            return Ok(output(json!({})));
        }
        let m = &v["meta"];
        let names: Vec<_> = m["fields"]
            .as_array()
            .ok_or("invalid metadata fields")?
            .iter()
            .map(|v| v.as_str().map(leak).ok_or("invalid metadata field"))
            .collect::<Result<_, _>>()?;
        let names = Box::leak(names.into_boxed_slice());
        let site = Box::leak(Box::new(Site(OnceLock::new())));
        let name = leak(text(m, "name")?);
        let target = leak(text(m, "target")?);
        let meta = Box::leak(Box::new(Metadata::new(
            name,
            target,
            text(m, "level")?.parse().map_err(|_| "invalid level")?,
            m["file"].as_str().map(leak),
            m["line"].as_u64().map(|v| v as u32),
            m["module"].as_str().map(leak),
            FieldSet::new(names, Identifier(site)),
            if m["span"] == true {
                Kind::SPAN
            } else {
                Kind::EVENT
            },
        )));
        site.0.set(meta).unwrap();
        metas.insert(
            key,
            Meta {
                metadata: meta,
                names,
            },
        );
        return Ok(output(json!({})));
    }
    let meta = |key: &str| -> Result<Meta, String> {
        logger
            .metadata
            .lock()
            .unwrap()
            .get(&id(v, key)?)
            .copied()
            .ok_or_else(|| "unknown logging metadata".into())
    };
    let dispatch = &logger.dispatch;
    tracing::dispatcher::with_default(dispatch, || {
        let result = match op {
            "logging_register" => {
                let interest = dispatch.register_callsite(meta("id")?.metadata);
                json!({"interest":if interest.is_never(){"never"}else if interest.is_always(){"always"}else{"sometimes"}})
            }
            "logging_enabled" => {
                let known = v["id"]
                    .as_u64()
                    .and_then(|id| logger.metadata.lock().unwrap().get(&id).copied());
                let m = &v["meta"];
                let borrowed = metadata(
                    m,
                    known.map_or_else(
                        || intern_name(m["name"].as_str().unwrap_or("enabled")),
                        |m| m.metadata.name(),
                    ),
                    known.map_or(&[], |m| m.names),
                    known.map_or_else(dummy, |m| m.metadata.callsite()),
                )?;
                json!({"enabled":dispatch.enabled(&borrowed)})
            }
            "logging_current" => {
                let current = dispatch.current_span();
                if let (Some(id), Some(metadata)) = (current.id(), current.metadata()) {
                    let key = logger
                        .metadata
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|(_, m)| m.metadata.callsite() == metadata.callsite())
                        .map(|(id, _)| *id)
                        .ok_or("unknown current metadata")?;
                    json!({"id":id.into_u64(),"meta":key})
                } else {
                    json!({})
                }
            }
            "logging_enter" => {
                dispatch.enter(&Id::from_u64(id(v, "id")?));
                json!({})
            }
            "logging_exit" => {
                dispatch.exit(&Id::from_u64(id(v, "id")?));
                json!({})
            }
            "logging_close" => json!({"closed":dispatch.try_close(Id::from_u64(id(v,"id")?))}),
            "logging_clone" => {
                json!({"id":dispatch.clone_span(&Id::from_u64(id(v,"id")?)).into_u64()})
            }
            "logging_follows" => {
                dispatch.record_follows_from(
                    &Id::from_u64(id(v, "id")?),
                    &Id::from_u64(id(v, "follows")?),
                );
                json!({})
            }
            "logging_event" if v["log"] == true => {
                let fields = v["fields"].as_array().ok_or("invalid log fields")?;
                let field = |key: &str| fields.iter().find(|f| f[0] == key).map(|f| &f[2]);
                let message = field("message").and_then(Value::as_str).unwrap_or("");
                let target = field("log.target").and_then(Value::as_str).unwrap_or("log");
                use tracing_log::AsLog;
                tracing_log::format_trace(
                    &tracing_log::log::Record::builder()
                        .args(format_args!("{message}"))
                        .level(meta("meta")?.metadata.level().as_log())
                        .target(target)
                        .module_path(field("log.module_path").and_then(Value::as_str))
                        .file(field("log.file").and_then(Value::as_str))
                        .line(field("log.line").and_then(Value::as_u64).map(|v| v as u32))
                        .build(),
                )
                .map_err(|e| e.to_string())?;
                json!({})
            }
            "logging_span" | "logging_record" | "logging_event" => {
                let meta = meta("meta")?.metadata;
                let owned = values(&v["fields"])?;
                if owned
                    .iter()
                    .any(|(name, _)| meta.fields().field(name).is_none())
                {
                    return Err("unknown logging field".into());
                }
                let borrowed: Vec<_> = meta
                    .fields()
                    .iter()
                    .map(|field| {
                        owned
                            .iter()
                            .find(|(name, _)| name == field.name())
                            .map(|(_, v)| &**v as &dyn TraceValue)
                    })
                    .collect();
                let values = meta.fields().value_set_all(&borrowed);
                let parent = v["parent"].as_u64();
                match op {
                    "logging_span" => {
                        let attrs = match parent {
                            None => Attributes::new(meta, &values),
                            Some(0) => Attributes::new_root(meta, &values),
                            Some(id) => Attributes::child_of(Id::from_u64(id), meta, &values),
                        };
                        json!({"id":dispatch.new_span(&attrs).into_u64()})
                    }
                    "logging_record" => {
                        dispatch.record(&Id::from_u64(id(v, "id")?), &Record::new(&values));
                        json!({})
                    }
                    _ => {
                        let event = match parent {
                            None => tracing::Event::new(meta, &values),
                            Some(id) => tracing::Event::new_child_of(
                                if id == 0 {
                                    None
                                } else {
                                    Some(Id::from_u64(id))
                                },
                                meta,
                                &values,
                            ),
                        };
                        dispatch.event(&event);
                        json!({})
                    }
                }
            }
            _ => return Err("unknown logging operation".into()),
        };
        Ok(output(result))
    })
}
