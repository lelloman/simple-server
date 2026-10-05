//! Safe runtime bindings. Only this crate interprets the engine's C ABI.
pub use simple_server_abi::values;
use simple_server_abi::{Api, Buffer, RuntimeOptions, Task, Wake, *};
use std::{
    any::Any,
    cell::RefCell,
    ffi::c_void,
    fmt,
    future::Future,
    io,
    marker::PhantomData,
    mem::MaybeUninit,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    pin::Pin,
    ptr::NonNull,
    sync::{Arc, Mutex, OnceLock, Weak},
    task::{Context, Poll, Waker},
    time::Duration,
};

unsafe extern "C" {
    fn simple_server_engine_v1() -> *const Api;
}

fn api() -> &'static Api {
    static API: OnceLock<&'static Api> = OnceLock::new();
    API.get_or_init(|| {
        let pointer = unsafe { simple_server_engine_v1() };
        assert!(
            !pointer.is_null(),
            "simple-server engine returned a null interface"
        );
        // Read the fixed prefix before forming a reference to the complete table.
        assert!(
            unsafe { pointer.cast::<usize>().read() } >= size_of::<Api>(),
            "simple-server engine interface is too small"
        );
        let api = unsafe { &*pointer };
        assert_eq!(
            api.abi_version, ABI_VERSION,
            "simple-server engine ABI mismatch"
        );
        let version = take_buffer(unsafe { (api.version)() });
        assert_eq!(
            version,
            ENGINE_VERSION.as_bytes(),
            "simple-server engine release mismatch"
        );
        api
    })
}

fn take_buffer(buffer: Buffer) -> Vec<u8> {
    struct Release(Buffer);
    impl Drop for Release {
        fn drop(&mut self) {
            unsafe { (self.0.release)(self.0.context) };
        }
    }
    let buffer = Release(buffer);
    if buffer.0.len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(buffer.0.data, buffer.0.len) }.to_vec()
    }
}
fn last_error() -> io::Error {
    io::Error::other(
        String::from_utf8_lossy(&take_buffer(unsafe { (api().last_error)() })).into_owned(),
    )
}

struct ForeignWake(Wake);
// ABI wake/release callbacks are required to be thread safe.
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
        callback::release_safely(|| unsafe { &*context.cast::<Waker>() }.wake_by_ref());
    }
    unsafe extern "C" fn release(context: *mut c_void) {
        callback::release_safely(|| drop(unsafe { Box::from_raw(context.cast::<Waker>()) }));
    }
    Wake {
        context: Box::into_raw(Box::new(waker.clone())).cast(),
        wake,
        release,
    }
}

struct Inner(NonNull<c_void>);
// The engine runtime allows concurrent task submission and operation creation.
unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}
impl Drop for Inner {
    fn drop(&mut self) {
        unsafe { (api().runtime_release)(self.0.as_ptr()) };
    }
}

thread_local! { static CURRENT: RefCell<Weak<Inner>> = const { RefCell::new(Weak::new()) }; }
struct Enter(Weak<Inner>);
impl Enter {
    fn new(runtime: Weak<Inner>) -> Self {
        Self(CURRENT.with(|current| current.replace(runtime)))
    }
}
impl Drop for Enter {
    fn drop(&mut self) {
        CURRENT.with(|current| {
            current.replace(self.0.clone());
        });
    }
}
fn current() -> io::Result<Arc<Inner>> {
    CURRENT
        .with(|current| current.borrow().upgrade())
        .ok_or_else(|| io::Error::other("no active simple-server engine runtime"))
}

/// Owns an engine runtime. Dropping the last owner requests runtime shutdown.
#[derive(Clone)]
pub struct Runtime(Arc<Inner>);
impl Runtime {
    pub fn new() -> io::Result<Self> {
        Builder::new_multi_thread().build()
    }
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        assert!(
            CURRENT.with(|current| current.borrow().upgrade().is_none()),
            "cannot block_on inside an engine task"
        );
        struct Root<F: Future> {
            future: Pin<Box<F>>,
            output: Option<F::Output>,
            panic: Option<Box<dyn Any + Send>>,
            runtime: Weak<Inner>,
        }
        unsafe extern "C" fn poll<F: Future>(context: *mut c_void, wake: Wake) -> i32 {
            let root = unsafe { &mut *context.cast::<Root<F>>() };
            let waker = import_wake(wake);
            let _entered = Enter::new(root.runtime.clone());
            match catch_unwind(AssertUnwindSafe(|| {
                root.future.as_mut().poll(&mut Context::from_waker(&waker))
            })) {
                Ok(Poll::Pending) => PENDING,
                Ok(Poll::Ready(value)) => {
                    root.output = Some(value);
                    READY
                }
                Err(panic) => {
                    root.panic = Some(panic);
                    PANICKED
                }
            }
        }
        unsafe extern "C" fn release(_: *mut c_void) {}
        let mut root = Root {
            future: Box::pin(future),
            output: None,
            panic: None,
            runtime: Arc::downgrade(&self.0),
        };
        let task = Task {
            context: (&mut root as *mut Root<F>).cast(),
            poll: poll::<F>,
            release,
        };
        let result = unsafe { (api().runtime_run)(self.0.0.as_ptr(), task) };
        if let Some(panic) = root.panic.take() {
            resume_unwind(panic);
        }
        assert_eq!(result, READY, "engine root task failed: {}", last_error());
        root.output
            .take()
            .expect("root task did not produce a result")
    }
}

pub struct Builder {
    workers: u32,
    paused: bool,
}
impl Builder {
    pub fn new_current_thread() -> Self {
        Self {
            workers: 0,
            paused: false,
        }
    }
    pub fn new_multi_thread() -> Self {
        Self {
            workers: std::thread::available_parallelism().map_or(1, |n| n.get() as u32),
            paused: false,
        }
    }
    pub fn worker_threads(&mut self, workers: usize) -> &mut Self {
        assert!(workers > 0, "worker count must be positive");
        self.workers = workers.try_into().expect("worker count exceeds u32");
        self
    }
    pub fn start_paused(&mut self, paused: bool) -> &mut Self {
        self.paused = paused;
        self
    }
    pub fn build(&self) -> io::Result<Runtime> {
        let options = RuntimeOptions {
            size: size_of::<RuntimeOptions>(),
            worker_threads: self.workers,
            start_paused: u8::from(self.paused),
        };
        let pointer = unsafe { (api().runtime_new)(options) };
        NonNull::new(pointer)
            .map(|p| Runtime(Arc::new(Inner(p))))
            .ok_or_else(last_error)
    }
}

/// Cancellation or an application panic. Panic payloads never enter the engine.
pub struct JoinError {
    panic: Option<Box<dyn Any + Send>>,
}
impl JoinError {
    pub fn is_cancelled(&self) -> bool {
        self.panic.is_none()
    }
    pub fn is_panic(&self) -> bool {
        self.panic.is_some()
    }
    pub fn into_panic(self) -> Box<dyn Any + Send> {
        self.panic.expect("task was cancelled")
    }
    pub fn try_into_panic(self) -> Result<Box<dyn Any + Send>, Self> {
        if self.is_panic() {
            Ok(self.into_panic())
        } else {
            Err(self)
        }
    }
}
impl fmt::Debug for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.is_panic() {
            "task panicked"
        } else {
            "task cancelled"
        })
    }
}
impl std::error::Error for JoinError {}

struct Completion<T> {
    result: Option<Result<T, JoinError>>,
    waker: Option<Waker>,
    finished: bool,
}
impl<T> Completion<T> {
    fn finish(&mut self, result: Result<T, JoinError>) -> Option<Waker> {
        self.finished = true;
        self.result = Some(result);
        self.waker.take()
    }
}
struct Spawned<F: Future> {
    future: Option<Pin<Box<F>>>,
    completion: Arc<Mutex<Completion<F::Output>>>,
    runtime: Weak<Inner>,
}
unsafe extern "C" fn poll_spawned<F: Future>(context: *mut c_void, wake: Wake) -> i32 {
    let task = unsafe { &mut *context.cast::<Spawned<F>>() };
    let waker = import_wake(wake);
    let _entered = Enter::new(task.runtime.clone());
    let result = catch_unwind(AssertUnwindSafe(|| {
        let result = task
            .future
            .as_mut()
            .expect("task polled after completion")
            .as_mut()
            .poll(&mut Context::from_waker(&waker));
        if result.is_ready() {
            drop(task.future.take());
        }
        result
    }));
    let (result, status) = match result {
        Ok(Poll::Pending) => return PENDING,
        Ok(Poll::Ready(value)) => (Ok(value), READY),
        Err(panic) => (Err(JoinError { panic: Some(panic) }), PANICKED),
    };
    let waker = task
        .completion
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .finish(result);
    if let Some(waker) = waker {
        callback::release_safely(|| waker.wake());
    }
    status
}
unsafe extern "C" fn release_spawned<F: Future>(context: *mut c_void) {
    // Detached task outputs and panic payloads are application-owned and may
    // themselves panic on drop. Keep the entire release inside the host guard.
    callback::release_safely(|| {
        let task = unsafe { Box::from_raw(context.cast::<Spawned<F>>()) };
        let _entered = Enter::new(task.runtime.clone());
        let completion = task.completion.clone();
        let dropped = catch_unwind(AssertUnwindSafe(|| drop(task)));
        let waker = {
            let mut completion = completion.lock().unwrap();
            if completion.finished {
                None
            } else {
                completion.finish(Err(JoinError {
                    panic: dropped.err(),
                }))
            }
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    });
}

struct TaskControl(NonNull<c_void>);
// Abort handles may be used concurrently and outlive runtime shutdown.
unsafe impl Send for TaskControl {}
unsafe impl Sync for TaskControl {}
impl Drop for TaskControl {
    fn drop(&mut self) {
        unsafe { (api().task_release)(self.0.as_ptr()) };
    }
}

#[derive(Clone)]
pub struct AbortHandle(Arc<TaskControl>);
impl AbortHandle {
    pub fn abort(&self) {
        unsafe { (api().task_abort)(self.0.0.as_ptr()) };
    }
}

pub struct JoinHandle<T> {
    completion: Arc<Mutex<Completion<T>>>,
    control: AbortHandle,
}
impl<T> JoinHandle<T> {
    pub fn abort(&self) {
        self.control.abort();
    }
    pub fn abort_handle(&self) -> AbortHandle {
        self.control.clone()
    }
    pub fn is_finished(&self) -> bool {
        self.completion.lock().unwrap().finished
    }
}
impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut completion = self.completion.lock().unwrap();
        if let Some(result) = completion.result.take() {
            Poll::Ready(result)
        } else {
            assert!(!completion.finished, "JoinHandle polled after completion");
            completion.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    spawn_impl(future, false)
}

pub fn spawn_blocking<F, R>(function: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    spawn_impl(async move { function() }, true)
}

fn spawn_impl<F>(future: F, blocking: bool) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let runtime = current().expect("spawn requires an engine runtime");
    let completion = Arc::new(Mutex::new(Completion {
        result: None,
        waker: None,
        finished: false,
    }));
    let task = Box::new(Spawned {
        future: Some(Box::pin(future)),
        completion: completion.clone(),
        runtime: Arc::downgrade(&runtime),
    });
    let task = Task {
        context: Box::into_raw(task).cast(),
        poll: poll_spawned::<F>,
        release: release_spawned::<F>,
    };
    let spawn = if blocking {
        api().runtime_spawn_blocking
    } else {
        api().runtime_spawn
    };
    let control = unsafe { spawn(runtime.0.as_ptr(), task) };
    let control =
        NonNull::new(control).unwrap_or_else(|| panic!("engine spawn failed: {}", last_error()));
    JoinHandle {
        completion,
        control: AbortHandle(Arc::new(TaskControl(control))),
    }
}

/// One engine operation. No Rust future or allocator crosses the ABI.
pub struct Operation {
    pointer: NonNull<c_void>,
    complete: bool,
    _not_sync: PhantomData<std::cell::Cell<()>>,
}
// Engine operations are Send but are only polled by one caller at a time.
unsafe impl Send for Operation {}
impl Operation {
    pub fn command(header: &[u8], body: &[u8]) -> io::Result<Self> {
        Self::new(&frame(header, body)?)
    }
    pub fn new(command: &[u8]) -> io::Result<Self> {
        let runtime = current()?;
        let pointer =
            unsafe { (api().operation_new)(runtime.0.as_ptr(), command.as_ptr(), command.len()) };
        NonNull::new(pointer)
            .map(|pointer| Self {
                pointer,
                complete: false,
                _not_sync: PhantomData,
            })
            .ok_or_else(last_error)
    }
}

/// Frame control metadata separately from arbitrary binary payloads.
pub fn frame(header: &[u8], body: &[u8]) -> io::Result<Vec<u8>> {
    let length = u32::try_from(header.len())
        .map_err(|_| io::Error::other("engine command header too large"))?;
    let mut output = Vec::with_capacity(8 + header.len() + body.len());
    output.extend_from_slice(b"SS01");
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(header);
    output.extend_from_slice(body);
    Ok(output)
}

pub fn unframe(bytes: &[u8]) -> io::Result<(&[u8], &[u8])> {
    if bytes.len() < 8 || &bytes[..4] != b"SS01" {
        return Err(io::Error::other("invalid engine frame"));
    }
    let length = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let end = 8_usize
        .checked_add(length)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| io::Error::other("truncated engine frame"))?;
    Ok((&bytes[8..end], &bytes[end..]))
}

pub fn resource_new(command: &[u8]) -> io::Result<Vec<u8>> {
    let mut output = MaybeUninit::uninit();
    let status =
        unsafe { (api().resource_new)(command.as_ptr(), command.len(), output.as_mut_ptr()) };
    if status == READY {
        Ok(take_buffer(unsafe { output.assume_init() }))
    } else {
        Err(last_error())
    }
}

/// Ref-count in Rust, release in the engine. Resource IDs never name Rust types.
#[derive(Clone)]
pub struct Resource(Arc<ResourceInner>);
struct ResourceInner {
    kind: u32,
    id: u64,
}
impl Resource {
    /// A resource ID must be returned by the engine with the corresponding kind.
    /// Unknown IDs are harmless at release; operations reject stale IDs.
    pub fn new(kind: u32, id: u64) -> Self {
        Self(Arc::new(ResourceInner { kind, id }))
    }
    pub fn id(&self) -> u64 {
        self.0.id
    }
}
impl Drop for ResourceInner {
    fn drop(&mut self) {
        unsafe { (api().resource_release)(self.kind, self.id) };
    }
}
impl Future for Operation {
    type Output = io::Result<Vec<u8>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        assert!(!self.complete, "engine operation polled after completion");
        let mut output = MaybeUninit::uninit();
        let status = unsafe {
            (api().operation_poll)(
                self.pointer.as_ptr(),
                export_wake(cx.waker()),
                output.as_mut_ptr(),
            )
        };
        match status {
            PENDING => Poll::Pending,
            READY => {
                self.complete = true;
                Poll::Ready(Ok(take_buffer(unsafe { output.assume_init() })))
            }
            _ => {
                self.complete = true;
                Poll::Ready(Err(last_error()))
            }
        }
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        unsafe { (api().operation_release)(self.pointer.as_ptr()) };
    }
}

mod callback;
pub use callback::Callback;

pub async fn sleep(duration: Duration) {
    let command = format!(
        "{{\"op\":\"sleep\",\"secs\":{},\"nanos\":{}}}",
        duration.as_secs(),
        duration.subsec_nanos()
    );
    Operation::new(command.as_bytes())
        .expect("sleep requires an engine runtime")
        .await
        .expect("engine timer failed");
}
pub async fn yield_now() {
    Operation::new(b"{\"op\":\"yield\"}")
        .expect("yield requires an engine runtime")
        .await
        .expect("engine yield failed");
}
pub async fn advance(duration: Duration) {
    let command = format!("{{\"op\":\"advance\",\"nanos\":{}}}", duration.as_nanos());
    Operation::new(command.as_bytes())
        .expect("advance requires an engine runtime")
        .await
        .expect("engine clock failed");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Elapsed;
impl fmt::Display for Elapsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("deadline elapsed")
    }
}
impl std::error::Error for Elapsed {}

pub async fn timeout<F: Future>(duration: Duration, future: F) -> Result<F::Output, Elapsed> {
    let mut future = Box::pin(future);
    let mut timer = Box::pin(sleep(duration));
    std::future::poll_fn(|cx| {
        if let Poll::Ready(value) = future.as_mut().poll(cx) {
            return Poll::Ready(Ok(value));
        }
        timer.as_mut().poll(cx).map(|()| Err(Elapsed))
    })
    .await
}
