//! Cancellation trees without an executor dependency.
use crate::engine_lifecycle::Shutdown;
use std::sync::Arc;
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    own: Shutdown,
    parent: Option<Arc<Self>>,
}
impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn child_token(&self) -> Self {
        Self {
            own: Shutdown::new(),
            parent: Some(Arc::new(self.clone())),
        }
    }
    pub fn cancel(&self) {
        self.own.request()
    }
    pub fn is_cancelled(&self) -> bool {
        self.own.is_requested() || self.parent.as_ref().is_some_and(|p| p.is_cancelled())
    }
    pub async fn cancelled(&self) {
        if let Some(parent) = &self.parent {
            let _ = futures_util::future::select(
                Box::pin(self.own.requested()),
                Box::pin(parent.cancelled()),
            )
            .await;
        } else {
            self.own.requested().await;
        }
    }
}
