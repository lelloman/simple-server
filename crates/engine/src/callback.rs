use crate::{export_wake, operations};
use simple_server_abi::{Buffer, BytesFuture, Callback, PENDING, READY};
use std::{
    collections::HashMap,
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    sync::{Arc, Mutex, OnceLock},
    task::{Context, Poll},
};

pub struct ForeignCallback(Callback);
// The ABI requires concurrent calls and a Send + Sync callback context.
unsafe impl Send for ForeignCallback {}
unsafe impl Sync for ForeignCallback {}
impl Drop for ForeignCallback {
    fn drop(&mut self) {
        unsafe { (self.0.release)(self.0.context) };
    }
}
pub struct ForeignFuture(BytesFuture);
// The ABI requires movable futures, with serialized poll/release.
unsafe impl Send for ForeignFuture {}
impl Future for ForeignFuture {
    type Output = Result<Vec<u8>, String>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut output = MaybeUninit::uninit();
        match unsafe { (self.0.poll)(self.0.context, export_wake(cx.waker()), output.as_mut_ptr()) }
        {
            PENDING => Poll::Pending,
            READY => {
                struct Owned(Buffer);
                impl Drop for Owned {
                    fn drop(&mut self) {
                        unsafe { (self.0.release)(self.0.context) };
                    }
                }
                let output = Owned(unsafe { output.assume_init() });
                let bytes = if output.0.len == 0 {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(output.0.data, output.0.len) }.to_vec()
                };
                Poll::Ready(Ok(bytes))
            }
            _ => Poll::Ready(Err("application callback panicked".into())),
        }
    }
}
impl Drop for ForeignFuture {
    fn drop(&mut self) {
        unsafe { (self.0.release)(self.0.context) };
    }
}
impl ForeignCallback {
    pub fn call(&self, bytes: &[u8]) -> ForeignFuture {
        ForeignFuture(unsafe { (self.0.call)(self.0.context, bytes.as_ptr(), bytes.len()) })
    }
}
static CALLBACKS: OnceLock<Mutex<HashMap<u64, Arc<ForeignCallback>>>> = OnceLock::new();
fn callbacks() -> &'static Mutex<HashMap<u64, Arc<ForeignCallback>>> {
    CALLBACKS.get_or_init(Default::default)
}
pub fn insert(callback: Callback) -> u64 {
    let callback = Arc::new(ForeignCallback(callback));
    let id = operations::id();
    callbacks().lock().unwrap().insert(id, callback);
    id
}
pub fn get(id: u64) -> Result<Arc<ForeignCallback>, String> {
    callbacks()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| "unknown callback".into())
}
pub fn remove(id: u64) {
    let callback = callbacks().lock().unwrap().remove(&id);
    // Release application values outside the registry lock: their destructors
    // may release other resources or register additional callbacks.
    drop(callback);
}
