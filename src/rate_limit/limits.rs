/// One ordered snapshot constraint. Building a check performs only arithmetic;
/// database reads, transactions, reservations and side effects stay caller-owned.
#[derive(Clone, Debug)]
pub struct LimitCheck<K> {
    key: K,
    allowed: bool,
}
impl<K> LimitCheck<K> {
    /// Count admission requires current usage strictly below the configured
    /// limit. Signed values preserve applications with signed database counts.
    pub fn below(key: K, used: i128, limit: i128) -> Self {
        Self {
            key,
            allowed: used < limit,
        }
    }
    /// Projected byte/resource usage may equal the limit. Each addition saturates
    /// in the caller's usize domain, including any independently reserved space.
    pub fn projected(key: K, used: usize, cost: usize, reserve: usize, limit: usize) -> Self {
        Self {
            key,
            allowed: used.saturating_add(cost).saturating_add(reserve) <= limit,
        }
    }
}

/// Return the first failing constraint in declaration order. This preserves
/// quota-vs-capacity error precedence without owning storage or charging state.
pub fn evaluate_limits<K: Clone>(checks: &[LimitCheck<K>]) -> Result<(), K> {
    match checks.iter().find(|check| !check.allowed) {
        Some(check) => Err(check.key.clone()),
        None => Ok(()),
    }
}
