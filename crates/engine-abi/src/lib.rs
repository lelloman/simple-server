//! Binary interface only. No Rust-owned value crosses this boundary.
//!
//! Handles are opaque and must originate from the corresponding constructor.
//! Callers serialize polling and destruction of an operation. Callbacks must
//! never unwind. Each transferred callback context is released exactly once.
#![no_std]

extern crate alloc;
pub mod values;

use core::ffi::c_void;

pub const ABI_VERSION: u32 = 1;
pub const ENGINE_VERSION: &str = "0.2.0";
pub const PENDING: i32 = 0;
pub const READY: i32 = 1;
pub const PANICKED: i32 = 2;
pub const CANCELLED: i32 = 3;
pub const INVALID: i32 = 4;

/// An immutable buffer owned by the producer. Release once, even when empty.
/// The buffer and its release context must be movable between threads. The
/// recipient may retain it while decoding referenced resources; release may
/// also destroy producer-owned captures that keep those resources alive.
#[repr(C)]
pub struct Buffer {
    pub data: *const u8,
    pub len: usize,
    pub context: *mut c_void,
    pub release: unsafe extern "C" fn(*mut c_void),
}

/// An owned notification reference. `wake` borrows; `release` consumes it.
/// Both functions must be thread safe. The recipient may retain it after poll.
#[repr(C)]
pub struct Wake {
    pub context: *mut c_void,
    pub wake: unsafe extern "C" fn(*mut c_void),
    pub release: unsafe extern "C" fn(*mut c_void),
}

/// An application future. Poll consumes the Wake, including on failure.
/// A task is never polled concurrently, or after completion. Release happens
/// once after its last poll returns. Root tasks remain on the calling thread;
/// spawned tasks must be movable between engine worker threads.
#[repr(C)]
pub struct Task {
    pub context: *mut c_void,
    pub poll: unsafe extern "C" fn(*mut c_void, Wake) -> i32,
    pub release: unsafe extern "C" fn(*mut c_void),
}

/// A host future yielding owned bytes. Poll consumes Wake on every path and
/// writes Buffer only for READY. Release consumes the future, even if pending.
#[repr(C)]
pub struct BytesFuture {
    pub context: *mut c_void,
    pub poll: unsafe extern "C" fn(*mut c_void, Wake, *mut Buffer) -> i32,
    pub release: unsafe extern "C" fn(*mut c_void),
}

/// Thread-safe reusable host callback. Call borrows input bytes for the duration
/// of the call and transfers a fresh BytesFuture to the engine. Call may run
/// concurrently. Release happens after the last call has returned; returned
/// futures must own their captures independently of the callback context.
#[repr(C)]
pub struct Callback {
    pub context: *mut c_void,
    pub call: unsafe extern "C" fn(*mut c_void, *const u8, usize) -> BytesFuture,
    pub release: unsafe extern "C" fn(*mut c_void),
}

#[repr(C)]
pub struct RuntimeOptions {
    pub size: usize,
    /// Zero selects current-thread mode; nonzero selects that many workers.
    pub worker_threads: u32,
    pub start_paused: u8,
}

/// Versioned, append-only function table. Constructors return null on error;
/// error messages are obtained immediately from `last_error` on that thread.
#[repr(C)]
pub struct Api {
    pub size: usize,
    pub abi_version: u32,
    pub version: unsafe extern "C" fn() -> Buffer,
    pub last_error: unsafe extern "C" fn() -> Buffer,
    pub runtime_new: unsafe extern "C" fn(RuntimeOptions) -> *mut c_void,
    pub runtime_release: unsafe extern "C" fn(*mut c_void),
    /// Borrows Task: caller releases its root context after this returns.
    pub runtime_run: unsafe extern "C" fn(*mut c_void, Task) -> i32,
    /// Consumes Task, including on error. Returns an owned task-control handle.
    pub runtime_spawn: unsafe extern "C" fn(*mut c_void, Task) -> *mut c_void,
    pub task_abort: unsafe extern "C" fn(*mut c_void),
    pub task_release: unsafe extern "C" fn(*mut c_void),
    /// Create an operation from a UTF-8 command, copying bytes before return.
    pub operation_new: unsafe extern "C" fn(*mut c_void, *const u8, usize) -> *mut c_void,
    /// Consumes Wake. Writes an owned result buffer only when READY.
    pub operation_poll: unsafe extern "C" fn(*mut c_void, Wake, *mut Buffer) -> i32,
    pub operation_release: unsafe extern "C" fn(*mut c_void),
    /// Synchronous resource construction. Writes a framed result on success.
    pub resource_new: unsafe extern "C" fn(*const u8, usize, *mut Buffer) -> i32,
    pub resource_release: unsafe extern "C" fn(u32, u64),
    /// Consumes a task whose first poll must complete synchronously.
    pub runtime_spawn_blocking: unsafe extern "C" fn(*mut c_void, Task) -> *mut c_void,
    /// Consumes Callback and returns its resource ID (kind 5), zero on failure.
    pub callback_new: unsafe extern "C" fn(Callback) -> u64,
}
