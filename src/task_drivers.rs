//! Engine-backed drivers for application-owned durable work.
//!
//! Enable `task-drivers` without default features to use these helpers without
//! compiling Tokio or the full scheduler. The application owns the engine
//! runtime (`simple_server::main` or `runtime::Builder`), admission, persistence,
//! reporting, and retry policy. No runtime or supervisor is started implicitly.
//!
//! These helpers share their algorithms with `task_scheduling`, whose APIs
//! continue using the source runtime even when both features are enabled.
use crate::runtime as execution;
// Intentional separate instantiations: each parent supplies its own execution
// backend, while the algorithms and error contracts have one maintained source.
#[allow(clippy::duplicate_mod)]
#[path = "task_scheduling/driver.rs"]
mod driver;
#[allow(clippy::duplicate_mod)]
#[path = "task_scheduling/join_error.rs"]
mod join_error;

pub use driver::{PollCadence, PollOutcome, run_bounded_batch, run_poll_worker};
pub use join_error::{BatchTaskId, JoinError};
