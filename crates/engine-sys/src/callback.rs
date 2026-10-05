use super::*;
use simple_server_abi::{BytesFuture, Callback as AbiCallback};

/// A reusable asynchronous host callback owned by the engine. Captures and
/// results are destroyed in this crate; only owned byte buffers cross the ABI.
#[derive(Clone)]
pub struct Callback(Resource);

/// Bytes plus host-owned resources borrowed by the engine while it decodes the
/// reply. The producer releases both only when the engine releases its buffer.
/// This makes returning another callback safe without leaking a registration
/// or dropping it before the engine has acquired its own reference.
pub struct Reply {
    bytes: Vec<u8>,
    retained: Vec<Box<dyn Send>>,
}
impl Reply {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            retained: Vec::new(),
        }
    }
    pub fn keep_alive<T: Send + 'static>(mut self, value: T) -> Self {
        self.retained.push(Box::new(value));
        self
    }
}

struct ContextData<F> {
    function: Arc<F>,
    runtime: Weak<Inner>,
}
type ByteTask = Pin<Box<dyn Future<Output = Reply> + Send>>;
struct FutureData {
    future: ByteTask,
    runtime: Weak<Inner>,
}

// Drop during an FFI release must not unwind into the engine. A second panic
// from a panic payload's destructor must not cross the boundary either.
pub(super) fn release_safely(f: impl FnOnce()) {
    if let Err(panic) = catch_unwind(AssertUnwindSafe(f)) {
        discard_panic(panic);
    }
}
fn discard_panic(panic: Box<dyn Any + Send>) {
    if let Err(second_panic) = catch_unwind(AssertUnwindSafe(|| drop(panic))) {
        std::mem::forget(second_panic);
    }
}
fn output_buffer(reply: Reply) -> Buffer {
    unsafe extern "C" fn release(context: *mut c_void) {
        release_safely(|| drop(unsafe { Box::from_raw(context.cast::<Reply>()) }));
    }
    let reply = Box::new(reply);
    Buffer {
        data: reply.bytes.as_ptr(),
        len: reply.bytes.len(),
        context: Box::into_raw(reply).cast(),
        release,
    }
}

unsafe extern "C" fn poll(context: *mut c_void, wake: Wake, output: *mut Buffer) -> i32 {
    let future = unsafe { &mut *context.cast::<FutureData>() };
    match catch_unwind(AssertUnwindSafe(|| {
        let _entered = Enter::new(future.runtime.clone());
        let waker = import_wake(wake);
        match future
            .future
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
        {
            Poll::Pending => PENDING,
            Poll::Ready(bytes) => {
                unsafe { output.write(output_buffer(bytes)) };
                READY
            }
        }
    })) {
        Ok(status) => status,
        Err(panic) => {
            discard_panic(panic);
            PANICKED
        }
    }
}
unsafe extern "C" fn release_future(context: *mut c_void) {
    release_safely(|| {
        let future = unsafe { Box::from_raw(context.cast::<FutureData>()) };
        let _entered = Enter::new(future.runtime.clone());
        drop(future);
    });
}
unsafe extern "C" fn call<F, Fut>(context: *mut c_void, bytes: *const u8, len: usize) -> BytesFuture
where
    F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Reply> + Send + 'static,
{
    let context = unsafe { &*context.cast::<ContextData<F>>() };
    let function = context.function.clone();
    let input = if len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec()
    };
    // Invoke the application only inside poll's unwind guard, including panics
    // thrown synchronously before the callback returns its future.
    let future = FutureData {
        future: Box::pin(async move { function(input).await }),
        runtime: context.runtime.clone(),
    };
    BytesFuture {
        context: Box::into_raw(Box::new(future)).cast(),
        poll,
        release: release_future,
    }
}
unsafe extern "C" fn release_callback<F>(context: *mut c_void) {
    release_safely(|| drop(unsafe { Box::from_raw(context.cast::<ContextData<F>>()) }));
}
impl Callback {
    /// Register on the active engine runtime. Each call enters that runtime's
    /// host context so handlers can spawn tasks and create engine operations.
    pub fn new<F, Fut>(function: F) -> io::Result<Self>
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Vec<u8>> + Send + 'static,
    {
        Self::with_reply(move |input| {
            let future = function(input);
            async move { Reply::new(future.await) }
        })
    }
    pub fn with_reply<F, Fut>(function: F) -> io::Result<Self>
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Reply> + Send + 'static,
    {
        let runtime = current()?;
        let context = Box::new(ContextData {
            function: Arc::new(function),
            runtime: Arc::downgrade(&runtime),
        });
        let callback = AbiCallback {
            context: Box::into_raw(context).cast(),
            call: call::<F, Fut>,
            release: release_callback::<F>,
        };
        let id = unsafe { (api().callback_new)(callback) };
        if id == 0 {
            Err(last_error())
        } else {
            Ok(Self(Resource::new(5, id)))
        }
    }
    pub fn id(&self) -> u64 {
        self.0.id()
    }
}
