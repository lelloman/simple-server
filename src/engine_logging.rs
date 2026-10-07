//! Filtering and formatting in the engine; the host keeps only tracing callsites
//! and an event/span transport. Installation is process-wide and requires no runtime.
pub use crate::logging_options::*;
use serde_json::{Value, json};
use std::{collections::HashMap, fmt, sync::Mutex};
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    subscriber::Interest,
};
use tracing_log::NormalizeEvent;

/// How the engine parses the supplied filter. Environment lookup stays in the app.
#[derive(Clone, Copy, Debug)]
pub enum FilterMode {
    /// Reject malformed or empty filters, as in the source logging initializer.
    Strict,
    /// Preserve `EnvFilter::new` behavior: discard invalid directives and use ERROR
    /// when no directives remain.
    Lossy,
    /// Discard invalid directives; use INFO when no valid directive remains.
    LossyOrInfo,
    /// Preserve `EnvFilter::try_new`, falling back to info on a parse error.
    StrictOrInfo,
}

/// Configuration errors remain distinct from an unavailable/incompatible engine.
#[derive(Debug)]
pub enum Error {
    Configuration(InitError),
    Engine(std::io::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(e) => e.fmt(f),
            Self::Engine(e) => write!(f, "native logging backend: {e}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<InitError> for Error {
    fn from(value: InitError) -> Self {
        Self::Configuration(value)
    }
}

/// Validate options and install the native-backed host subscriber once.
/// Does not install the host `log` bridge; call [`init_log_bridge`] explicitly.
pub fn try_init(options: LoggingOptions, mode: FilterMode) -> Result<(), Error> {
    static INIT: Mutex<bool> = Mutex::new(false);
    let mut initialized = INIT.lock().unwrap();
    if *initialized {
        return Err(InitError::AlreadyInitialized.into());
    }
    let result = command(json!({"op":"logging_new", "filter":options.filter,
        "mode":format!("{mode:?}"), "format":format!("{:?}",options.format),
        "output":format!("{:?}",options.output), "ansi":format!("{:?}",options.ansi),
        "target":options.with_target, "span_events":format!("{:?}",options.span_events)
    }))
    .map_err(Error::Engine)?;
    if result["error"] == "ansi" {
        return Err(InitError::AnsiWithJson.into());
    }
    if result["error"] == "filter" {
        return Err(InitError::InvalidFilter.into());
    }
    let id = result["id"].as_u64().ok_or(InitError::InvalidFilter)?;
    let max = result["max"]
        .as_str()
        .unwrap_or("TRACE")
        .parse()
        .unwrap_or(tracing::level_filters::LevelFilter::TRACE);
    let bridge = Bridge {
        resource: simple_server_sys::Resource::new(21, id),
        max,
        metadata: Mutex::new(HashMap::new()),
        spans: Mutex::new(HashMap::new()),
    };
    let result = tracing::subscriber::set_global_default(bridge);
    *initialized = true;
    result.map_err(|_| InitError::AlreadyInitialized.into())
}

/// Install the lightweight host log-facade adapter with the active tracing level.
/// An existing logger is not replaced. Filtering/formatting remain engine-owned.
pub fn init_log_bridge() -> Result<(), tracing_log::log::SetLoggerError> {
    use tracing_log::AsLog;
    tracing_log::LogTracer::builder()
        .with_max_level(tracing::level_filters::LevelFilter::current().as_log())
        .init()
}
fn command(header: Value) -> std::io::Result<Value> {
    let bytes = simple_server_sys::resource_new(&serde_json::to_vec(&header)?)?;
    let (header, _) = simple_server_sys::unframe(&bytes)?;
    Ok(serde_json::from_slice(header)?)
}
struct Bridge {
    resource: simple_server_sys::Resource,
    max: tracing::level_filters::LevelFilter,
    metadata: Mutex<HashMap<tracing::callsite::Identifier, (u64, &'static Metadata<'static>)>>,
    spans: Mutex<HashMap<u64, (&'static Metadata<'static>, usize)>>,
}
impl Bridge {
    fn call(&self, mut command_value: Value) -> Value {
        command_value["logger"] = json!(self.resource.id());
        command(command_value).expect("native logging protocol failed")
    }
    fn metadata(&self, meta: &'static Metadata<'static>) -> u64 {
        let mut sites = self.metadata.lock().unwrap();
        if let Some((id, _)) = sites.get(&meta.callsite()) {
            return *id;
        }
        let id = sites.len() as u64 + 1;
        self.call(json!({"op":"logging_metadata", "id":id, "meta":describe(meta)}));
        sites.insert(meta.callsite(), (id, meta));
        id
    }
}
fn describe(meta: &Metadata<'_>) -> Value {
    json!({"name":meta.name(),"target":meta.target(),"level":meta.level().as_str(),
        "file":meta.file(),"line":meta.line(),"module":meta.module_path(),"span":meta.is_span(),
        "fields":meta.fields().iter().map(|f| f.name()).collect::<Vec<_>>()})
}
fn parent(root: bool, parent: Option<&Id>) -> Value {
    if root {
        json!(0)
    } else {
        parent.map_or(Value::Null, |id| json!(id.into_u64()))
    }
}
#[derive(Default)]
struct Fields(Vec<Value>);
impl Fields {
    fn push(&mut self, field: &Field, kind: &str, value: Value) {
        self.0.push(json!([field.name(), kind, value]));
    }
}
impl Visit for Fields {
    fn record_debug(&mut self, f: &Field, v: &dyn fmt::Debug) {
        self.push(f, "debug", json!(format!("{v:?}")));
    }
    fn record_str(&mut self, f: &Field, v: &str) {
        self.push(f, "str", json!(v));
    }
    fn record_bool(&mut self, f: &Field, v: bool) {
        self.push(f, "bool", json!(v));
    }
    fn record_i64(&mut self, f: &Field, v: i64) {
        self.push(f, "i64", json!(v));
    }
    fn record_u64(&mut self, f: &Field, v: u64) {
        self.push(f, "u64", json!(v));
    }
    fn record_i128(&mut self, f: &Field, v: i128) {
        self.push(f, "i128", json!(v.to_string()));
    }
    fn record_u128(&mut self, f: &Field, v: u128) {
        self.push(f, "u128", json!(v.to_string()));
    }
    fn record_f64(&mut self, f: &Field, v: f64) {
        self.push(f, "f64", json!(v.to_bits()));
    }
    fn record_error(&mut self, f: &Field, v: &(dyn std::error::Error + 'static)) {
        let mut chain = vec![v.to_string()];
        let mut next = v.source();
        while let Some(error) = next {
            chain.push(error.to_string());
            next = error.source();
        }
        self.push(f, "error", json!(chain));
    }
}
impl Subscriber for Bridge {
    fn register_callsite(&self, meta: &'static Metadata<'static>) -> Interest {
        let id = self.metadata(meta);
        match self.call(json!({"op":"logging_register","id":id}))["interest"].as_str() {
            Some("always") => Interest::always(),
            Some("never") => Interest::never(),
            _ => Interest::sometimes(),
        }
    }
    fn max_level_hint(&self) -> Option<tracing::level_filters::LevelFilter> {
        Some(self.max)
    }
    fn enabled(&self, meta: &Metadata<'_>) -> bool {
        let id = self
            .metadata
            .lock()
            .unwrap()
            .get(&meta.callsite())
            .map(|(id, _)| *id);
        self.call(json!({"op":"logging_enabled","id":id,"meta":describe(meta)}))["enabled"] == true
    }
    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        let meta = self.metadata(attrs.metadata());
        let mut fields = Fields::default();
        attrs.record(&mut fields);
        let id=self.call(json!({"op":"logging_span","meta":meta,"fields":fields.0,"parent":parent(attrs.is_root(),attrs.parent())}))["id"].as_u64().expect("native span ID");
        self.spans.lock().unwrap().insert(id, (attrs.metadata(), 1));
        Id::from_u64(id)
    }
    fn record(&self, id: &Id, record: &Record<'_>) {
        let (meta, _) = *self
            .spans
            .lock()
            .unwrap()
            .get(&id.into_u64())
            .expect("live span");
        let mut fields = Fields::default();
        record.record(&mut fields);
        self.call(json!({"op":"logging_record","id":id.into_u64(),"meta":self.metadata(meta),"fields":fields.0}));
    }
    fn record_follows_from(&self, id: &Id, follows: &Id) {
        self.call(json!({"op":"logging_follows","id":id.into_u64(),"follows":follows.into_u64()}));
    }
    fn event(&self, event: &Event<'_>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let log = event.is_log();
        self.call(json!({"op":"logging_event","meta":self.metadata(event.metadata()),"fields":fields.0,"parent":parent(event.is_root(),event.parent()),"log":log}));
    }
    fn enter(&self, id: &Id) {
        self.call(json!({"op":"logging_enter","id":id.into_u64()}));
    }
    fn exit(&self, id: &Id) {
        self.call(json!({"op":"logging_exit","id":id.into_u64()}));
    }
    fn clone_span(&self, id: &Id) -> Id {
        let new = self.call(json!({"op":"logging_clone","id":id.into_u64()}))["id"]
            .as_u64()
            .expect("native cloned span");
        let mut spans = self.spans.lock().unwrap();
        let (meta, refs) = spans.get_mut(&id.into_u64()).expect("live cloned span");
        if new == id.into_u64() {
            *refs += 1;
        } else {
            let meta = *meta;
            spans.insert(new, (meta, 1));
        }
        Id::from_u64(new)
    }
    fn try_close(&self, id: Id) -> bool {
        let closed = self.call(json!({"op":"logging_close","id":id.into_u64()}))["closed"] == true;
        // Native child spans may retain their parent after all host handles have
        // gone. Track host references separately so those native closes do not
        // leave stale entries here; metadata itself is cached per callsite.
        let mut spans = self.spans.lock().unwrap();
        let (_, refs) = spans.get_mut(&id.into_u64()).expect("live closing span");
        *refs -= 1;
        if *refs == 0 {
            spans.remove(&id.into_u64());
        }
        closed
    }
    fn current_span(&self) -> tracing_core::span::Current {
        let result = self.call(json!({"op":"logging_current"}));
        let Some(id) = result["id"].as_u64() else {
            return tracing_core::span::Current::none();
        };
        let meta_id = result["meta"].as_u64().expect("current span metadata");
        let sites = self.metadata.lock().unwrap();
        let (_, meta) = sites
            .values()
            .find(|(key, _)| *key == meta_id)
            .expect("registered current metadata");
        tracing_core::span::Current::new(Id::from_u64(id), meta)
    }
}
