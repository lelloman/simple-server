//! Privately linked implementation of the simple-server binary interface.
use simple_server_abi::{Api, Buffer, RuntimeOptions, Task, Wake, *};
use std::{
    cell::RefCell,
    ffi::c_void,
    future::Future,
    mem::ManuallyDrop,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    ptr,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::Duration,
};
mod callback;
mod clock;
mod database;
mod multipart;
mod operations;
mod routing;
mod server;
mod signals;

thread_local! { static ERROR: RefCell<String> = const { RefCell::new(String::new()) }; }

fn guarded<T>(fallback: T, f: impl FnOnce() -> Result<T, String>) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            ERROR.with(|slot| *slot.borrow_mut() = error);
            fallback
        }
        Err(_) => {
            ERROR.with(|slot| *slot.borrow_mut() = "engine operation panicked".into());
            fallback
        }
    }
}

fn buffer(bytes: Vec<u8>) -> Buffer {
    let bytes = Box::new(bytes);
    Buffer {
        data: bytes.as_ptr(),
        len: bytes.len(),
        context: Box::into_raw(bytes).cast(),
        release: release_buffer,
    }
}
unsafe extern "C" fn release_buffer(context: *mut c_void) {
    drop(unsafe { Box::from_raw(context.cast::<Vec<u8>>()) });
}
unsafe extern "C" fn version() -> Buffer {
    buffer(ENGINE_VERSION.as_bytes().to_vec())
}
unsafe extern "C" fn last_error() -> Buffer {
    buffer(ERROR.with(|s| s.borrow().as_bytes().to_vec()))
}

struct ForeignWake(Wake);
// The ABI requires notification callbacks and their contexts to be thread safe.
unsafe impl Send for ForeignWake {}
unsafe impl Sync for ForeignWake {}
impl std::task::Wake for ForeignWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        unsafe { (self.0.wake)(self.0.context) };
    }
}
impl Drop for ForeignWake {
    fn drop(&mut self) {
        unsafe { (self.0.release)(self.0.context) };
    }
}
fn import_wake(wake: Wake) -> Waker {
    Waker::from(Arc::new(ForeignWake(wake)))
}
fn export_wake(waker: &Waker) -> Wake {
    unsafe extern "C" fn wake(context: *mut c_void) {
        unsafe { &*context.cast::<Waker>() }.wake_by_ref();
    }
    unsafe extern "C" fn release(context: *mut c_void) {
        drop(unsafe { Box::from_raw(context.cast::<Waker>()) });
    }
    Wake {
        context: Box::into_raw(Box::new(waker.clone())).cast(),
        wake,
        release,
    }
}

struct ForeignTask(Task);
// runtime_spawn requires a Send task. runtime_run never moves its root task.
unsafe impl Send for ForeignTask {}
impl Future for ForeignTask {
    type Output = i32;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<i32> {
        match unsafe { (self.0.poll)(self.0.context, export_wake(cx.waker())) } {
            PENDING => Poll::Pending,
            result => Poll::Ready(result),
        }
    }
}
impl Drop for ForeignTask {
    fn drop(&mut self) {
        unsafe { (self.0.release)(self.0.context) };
    }
}

unsafe extern "C" fn runtime_new(options: RuntimeOptions) -> *mut c_void {
    guarded(ptr::null_mut(), || {
        if options.size != size_of::<RuntimeOptions>() {
            return Err("invalid runtime options".into());
        }
        let mut builder = if options.worker_threads == 0 {
            tokio::runtime::Builder::new_current_thread()
        } else {
            let mut b = tokio::runtime::Builder::new_multi_thread();
            b.worker_threads(options.worker_threads as usize);
            b
        };
        builder.enable_all().start_paused(options.start_paused != 0);
        builder
            .build()
            .map(|runtime| Box::into_raw(Box::new(runtime)).cast())
            .map_err(|e| e.to_string())
    })
}
unsafe extern "C" fn runtime_release(runtime: *mut c_void) {
    guarded((), || {
        // Nonblocking destruction is also legal when the last owner is dropped
        // from a callback. Applications explicitly drain before releasing it.
        unsafe { Box::from_raw(runtime.cast::<tokio::runtime::Runtime>()) }.shutdown_background();
        Ok(())
    });
}
unsafe extern "C" fn runtime_run(runtime: *mut c_void, task: Task) -> i32 {
    guarded(PANICKED, || {
        let runtime = unsafe { &*runtime.cast::<tokio::runtime::Runtime>() };
        let mut task = ManuallyDrop::new(ForeignTask(task));
        Ok(runtime.block_on(Pin::new(&mut *task)))
    })
}
unsafe extern "C" fn runtime_spawn(runtime: *mut c_void, task: Task) -> *mut c_void {
    let task = ForeignTask(task);
    guarded(ptr::null_mut(), || {
        let runtime = unsafe { &*runtime.cast::<tokio::runtime::Runtime>() };
        let task = runtime.spawn(task);
        let abort = task.abort_handle();
        drop(task); // Detach; the glue owns the result and observes cancellation.
        Ok(Box::into_raw(Box::new(abort)).cast())
    })
}
unsafe extern "C" fn task_abort(task: *mut c_void) {
    unsafe { &*task.cast::<tokio::task::AbortHandle>() }.abort();
}

unsafe extern "C" fn runtime_spawn_blocking(runtime: *mut c_void, task: Task) -> *mut c_void {
    let mut task = ForeignTask(task);
    guarded(ptr::null_mut(), || {
        let runtime = unsafe { &*runtime.cast::<tokio::runtime::Runtime>() };
        let task = runtime.spawn_blocking(move || {
            let mut context = Context::from_waker(Waker::noop());
            match Pin::new(&mut task).poll(&mut context) {
                Poll::Ready(result) => result,
                Poll::Pending => INVALID,
            }
        });
        let abort = task.abort_handle();
        drop(task);
        Ok(Box::into_raw(Box::new(abort)).cast())
    })
}
unsafe extern "C" fn task_release(task: *mut c_void) {
    drop(unsafe { Box::from_raw(task.cast::<tokio::task::AbortHandle>()) });
}

struct Operation(Pin<Box<dyn Future<Output = Vec<u8>> + Send>>);

unsafe extern "C" fn operation_new(
    runtime: *mut c_void,
    data: *const u8,
    len: usize,
) -> *mut c_void {
    guarded(ptr::null_mut(), || {
        let runtime = unsafe { &*runtime.cast::<tokio::runtime::Runtime>() };
        let _entered = runtime.enter();
        let bytes = unsafe { std::slice::from_raw_parts(data, len) };
        let (command, payload) = operations::decode(bytes)?;
        let future: Pin<Box<dyn Future<Output = Vec<u8>> + Send>> = match command["op"].as_str() {
            Some("sleep") => {
                let duration = Duration::new(
                    command["secs"].as_u64().ok_or("missing seconds")?,
                    command["nanos"]
                        .as_u64()
                        .ok_or("missing nanoseconds")?
                        .try_into()
                        .map_err(|_| "invalid nanoseconds")?,
                );
                let sleep = tokio::time::sleep(duration);
                Box::pin(async move {
                    sleep.await;
                    Vec::new()
                })
            }
            Some("yield") => Box::pin(async {
                tokio::task::yield_now().await;
                Vec::new()
            }),
            Some("sleep_until") => {
                let deadline = clock::decode(MonotonicInstant {
                    seconds: command["seconds"]
                        .as_i64()
                        .ok_or("invalid deadline seconds")?,
                    nanoseconds: command["nanoseconds"]
                        .as_u64()
                        .ok_or("invalid deadline nanos")?
                        .try_into()
                        .map_err(|_| "invalid deadline nanos")?,
                })
                .ok_or("deadline out of range")?;
                let sleep = tokio::time::sleep_until(deadline);
                Box::pin(async move {
                    sleep.await;
                    Vec::new()
                })
            }
            Some("advance") => {
                let duration = if let Some(seconds) = command["secs"].as_u64() {
                    let nanos = command["nanos"]
                        .as_u64()
                        .filter(|n| *n < 1_000_000_000)
                        .ok_or("invalid nanoseconds")?;
                    Duration::new(seconds, nanos as u32)
                } else {
                    Duration::from_nanos(command["nanos"].as_u64().ok_or("missing nanoseconds")?)
                };
                Box::pin(async move {
                    tokio::time::advance(duration).await;
                    Vec::new()
                })
            }
            _ => operations::operation(command, payload)?,
        };
        Ok(Box::into_raw(Box::new(Operation(future))).cast())
    })
}

unsafe extern "C" fn resource_new(data: *const u8, len: usize, output: *mut Buffer) -> i32 {
    guarded(INVALID, || {
        let (command, body) = operations::decode(unsafe { std::slice::from_raw_parts(data, len) })?;
        let result = operations::resource_new(command, body)?;
        unsafe { output.write(buffer(result)) };
        Ok(READY)
    })
}
unsafe extern "C" fn resource_release(kind: u32, id: u64) {
    guarded((), || {
        operations::resource_release(kind, id);
        Ok(())
    });
}
unsafe extern "C" fn operation_poll(
    operation: *mut c_void,
    wake: Wake,
    output: *mut Buffer,
) -> i32 {
    let waker = import_wake(wake);
    guarded(PANICKED, || {
        let operation = unsafe { &mut *operation.cast::<Operation>() };
        match operation.0.as_mut().poll(&mut Context::from_waker(&waker)) {
            Poll::Pending => Ok(PENDING),
            Poll::Ready(result) => {
                unsafe { output.write(buffer(result)) };
                Ok(READY)
            }
        }
    })
}
unsafe extern "C" fn operation_release(operation: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(operation.cast::<Operation>()) });
        Ok(())
    });
}

static API: Api = Api {
    size: size_of::<Api>(),
    abi_version: ABI_VERSION,
    version,
    last_error,
    runtime_new,
    runtime_release,
    runtime_run,
    runtime_spawn,
    task_abort,
    task_release,
    operation_new,
    operation_poll,
    operation_release,
    resource_new,
    resource_release,
    runtime_spawn_blocking,
    callback_new,
    clock_now: clock::now,
    clock_valid: clock::valid,
};

unsafe extern "C" fn callback_new(callback: simple_server_abi::Callback) -> u64 {
    guarded(0, || Ok(callback::insert(callback)))
}

/// Obtain ABI v1. This is the only symbol consumers need to link.
#[unsafe(no_mangle)]
pub extern "C" fn simple_server_engine_v1() -> *const Api {
    &API
}
