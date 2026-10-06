//! Optional context for applications retaining Tokio-backed database/client work.
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
#[derive(Clone, Default)]
pub(super) struct HostRuntime {
    #[cfg(feature = "engine-tracing")]
    dispatcher: tracing::Dispatch,
    #[cfg(feature = "engine-tokio")]
    handle: Option<tokio::runtime::Handle>,
}
impl HostRuntime {
    pub fn capture() -> Self {
        Self {
            #[cfg(feature = "engine-tracing")]
            dispatcher: tracing::dispatcher::get_default(Clone::clone),
            #[cfg(feature = "engine-tokio")]
            handle: tokio::runtime::Handle::try_current().ok(),
        }
    }
    pub fn own<T>(&self, value: T) -> Owned<T> {
        Owned {
            value: Some(value),
            context: self.clone(),
        }
    }
    pub fn scope<F: Future>(&self, future: F) -> Scoped<F> {
        Scoped {
            future: Some(Box::pin(future)),
            context: self.clone(),
        }
    }
}
pub(super) struct Scoped<F> {
    future: Option<Pin<Box<F>>>,
    context: HostRuntime,
}
impl<F: Future> Future for Scoped<F> {
    type Output = F::Output;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let context = self.context.clone();
        #[cfg(feature = "engine-tracing")]
        let _tracing = tracing::dispatcher::set_default(&context.dispatcher);
        #[cfg(feature = "engine-tokio")]
        let _guard = context.handle.as_ref().map(|handle| handle.enter());
        #[cfg(not(any(feature = "engine-tokio", feature = "engine-tracing")))]
        let _ = context;
        self.future
            .as_mut()
            .expect("future alive")
            .as_mut()
            .poll(cx)
    }
}
impl<F> Drop for Scoped<F> {
    fn drop(&mut self) {
        #[cfg(feature = "engine-tracing")]
        let _tracing = tracing::dispatcher::set_default(&self.context.dispatcher);
        #[cfg(feature = "engine-tokio")]
        let _guard = self.context.handle.as_ref().map(|handle| handle.enter());
        drop(self.future.take());
    }
}

// Callback registrations can outlive their last polled future. Their captured
// values must also be destroyed inside the application runtime context.
pub(super) struct Owned<T> {
    value: Option<T>,
    context: HostRuntime,
}
impl<T> std::ops::Deref for Owned<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.value.as_ref().expect("value alive")
    }
}
impl<T> Drop for Owned<T> {
    fn drop(&mut self) {
        #[cfg(feature = "engine-tracing")]
        let _tracing = tracing::dispatcher::set_default(&self.context.dispatcher);
        #[cfg(feature = "engine-tokio")]
        let _guard = self.context.handle.as_ref().map(|handle| handle.enter());
        #[cfg(not(any(feature = "engine-tokio", feature = "engine-tracing")))]
        let _ = &self.context;
        drop(self.value.take());
    }
}
