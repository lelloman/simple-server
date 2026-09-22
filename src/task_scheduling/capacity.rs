//! Capacity for applications that retain their own scheduler and durable state.
use std::{collections::BTreeMap, fmt, num::NonZeroUsize, sync::Arc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Global and optional resource-pool execution limits. Clones share capacity.
///
/// The application bounds pending work and owns queue deadlines, cancellation,
/// overlap checks, and persistence. This type does not spawn or queue jobs itself.
#[derive(Clone, Debug)]
pub struct ExecutionCapacity {
    global: Arc<Semaphore>,
    pools: BTreeMap<String, Arc<Semaphore>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityError(pub String);
impl fmt::Display for CapacityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CapacityError {}

/// Keep this non-cloneable permit until execution actually ends, including
/// blocking work that continues after a cancellation request or runtime expiry.
#[derive(Debug)]
#[must_use = "dropping the permit releases execution capacity"]
pub struct ExecutionPermit {
    _global: OwnedSemaphorePermit,
    _pool: Option<OwnedSemaphorePermit>,
}

impl ExecutionCapacity {
    pub fn new(
        global: NonZeroUsize,
        pools: BTreeMap<String, NonZeroUsize>,
    ) -> Result<Self, CapacityError> {
        if global.get() > Semaphore::MAX_PERMITS
            || pools
                .values()
                .any(|limit| limit.get() > Semaphore::MAX_PERMITS)
        {
            return Err(CapacityError(
                "execution capacity exceeds Tokio's semaphore limit".into(),
            ));
        }
        if pools.keys().any(|name| name.is_empty()) {
            return Err(CapacityError(
                "resource pool names must not be empty".into(),
            ));
        }
        Ok(Self {
            global: Arc::new(Semaphore::new(global.get())),
            pools: pools
                .into_iter()
                .map(|(name, limit)| (name, Arc::new(Semaphore::new(limit.get()))))
                .collect(),
        })
    }

    /// Acquire the pool first, then global capacity. A saturated pool never
    /// reserves a global slot. A waiter may hold a pool slot while awaiting the
    /// global semaphore; this is intentional and preserves that acquisition order.
    ///
    /// Dropping this future releases any partially acquired capacity. Semaphore
    /// waits are FIFO, but cancelling and recreating a wait loses its queue place.
    pub async fn acquire(&self, pool: Option<&str>) -> Result<ExecutionPermit, CapacityError> {
        let pool = match pool {
            Some(name) => Some(
                self.pools
                    .get(name)
                    .ok_or_else(|| CapacityError(format!("unknown resource pool: {name}")))?
                    .clone(),
            ),
            None => None,
        };
        // Semaphores are private and never closed; cancellation is application-owned.
        let pool_permit = match pool {
            Some(pool) => Some(
                pool.acquire_owned()
                    .await
                    .expect("capacity semaphore is never closed"),
            ),
            None => None,
        };
        let global = self
            .global
            .clone()
            .acquire_owned()
            .await
            .expect("capacity semaphore is never closed");
        Ok(ExecutionPermit {
            _global: global,
            _pool: pool_permit,
        })
    }
}
