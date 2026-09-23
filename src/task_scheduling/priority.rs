//! Bounded priority admission with per-class caps and cancellation-safe permits.
use super::ConfigError;
use std::{
    collections::VecDeque,
    fmt,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};
use tokio::sync::oneshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorityAdmissionError {
    QueueFull,
    UnknownClass,
}
impl fmt::Display for PriorityAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "priority admission rejected: {self:?}")
    }
}
impl std::error::Error for PriorityAdmissionError {}

#[derive(Debug)]
struct Class {
    limit: usize,
    active: usize,
    waiting: VecDeque<oneshot::Sender<PriorityPermit>>,
}
#[derive(Debug)]
struct State {
    active: usize,
    classes: Vec<Class>,
}
impl State {
    fn prune(&mut self) {
        for class in &mut self.classes {
            class.waiting.retain(|waiter| !waiter.is_closed());
        }
    }
    fn queued(&self) -> usize {
        self.classes.iter().map(|class| class.waiting.len()).sum()
    }
}
#[derive(Debug)]
struct Inner {
    maximum: usize,
    queue_limit: usize,
    state: Mutex<State>,
}
impl Inner {
    fn dispatch(self: &Arc<Self>, state: &mut State) {
        state.prune();
        while state.active < self.maximum {
            let Some(index) = state
                .classes
                .iter()
                .position(|class| class.active < class.limit && !class.waiting.is_empty())
            else {
                break;
            };
            let sender = state.classes[index]
                .waiting
                .pop_front()
                .expect("eligible class has a waiter");
            state.active += 1;
            state.classes[index].active += 1;
            let permit = PriorityPermit {
                inner: self.clone(),
                class: index,
                armed: true,
            };
            if let Err(mut permit) = sender.send(permit) {
                // Avoid recursively acquiring this lock if cancellation won send.
                permit.armed = false;
                state.active -= 1;
                state.classes[index].active -= 1;
            }
        }
    }
}

/// Class index zero has highest priority; each class is FIFO. Saturated classes
/// do not block eligible lower classes. Zero class capacity deliberately disables
/// that class. Starvation of lower priorities is possible by design.
///
/// Admission enters a bounded queue before dispatch, so a zero queue limit rejects
/// even when execution capacity is free. No job, timer or background task is owned.
#[derive(Clone, Debug)]
pub struct PriorityCapacity {
    inner: Arc<Inner>,
}
impl PriorityCapacity {
    pub fn new(
        maximum: NonZeroUsize,
        class_limits: Vec<usize>,
        queue_limit: usize,
    ) -> Result<Self, ConfigError> {
        if class_limits.is_empty() || class_limits.iter().any(|limit| *limit > maximum.get()) {
            return Err(ConfigError(
                "priority classes require limits no greater than global capacity".into(),
            ));
        }
        Ok(Self {
            inner: Arc::new(Inner {
                maximum: maximum.get(),
                queue_limit,
                state: Mutex::new(State {
                    active: 0,
                    classes: class_limits
                        .into_iter()
                        .map(|limit| Class {
                            limit,
                            active: 0,
                            waiting: VecDeque::new(),
                        })
                        .collect(),
                }),
            }),
        })
    }
    pub async fn acquire(&self, class: usize) -> Result<PriorityPermit, PriorityAdmissionError> {
        let (sender, receiver) = oneshot::channel();
        {
            let mut state = self.inner.state.lock().expect("priority capacity poisoned");
            if class >= state.classes.len() {
                return Err(PriorityAdmissionError::UnknownClass);
            }
            state.prune();
            if state.queued() >= self.inner.queue_limit {
                return Err(PriorityAdmissionError::QueueFull);
            }
            state.classes[class].waiting.push_back(sender);
            self.inner.dispatch(&mut state);
        }
        // A granted permit lives in the channel until observed. Dropping this
        // future after grant drops the permit and restores capacity immediately.
        Ok(receiver
            .await
            .expect("admitted waiter retains its sender until grant"))
    }
    pub fn snapshot(&self) -> PrioritySnapshot {
        let mut state = self.inner.state.lock().expect("priority capacity poisoned");
        state.prune();
        PrioritySnapshot {
            active: state.active,
            queued: state.queued(),
            classes: state
                .classes
                .iter()
                .map(|c| (c.active, c.waiting.len()))
                .collect(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrioritySnapshot {
    pub active: usize,
    pub queued: usize,
    /// (active, queued), in class priority order.
    pub classes: Vec<(usize, usize)>,
}
#[derive(Debug)]
#[must_use = "keep the permit until the admitted operation actually ends"]
pub struct PriorityPermit {
    inner: Arc<Inner>,
    class: usize,
    armed: bool,
}
impl Drop for PriorityPermit {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let mut state = self.inner.state.lock().expect("priority capacity poisoned");
        state.active -= 1;
        state.classes[self.class].active -= 1;
        self.inner.dispatch(&mut state);
    }
}
