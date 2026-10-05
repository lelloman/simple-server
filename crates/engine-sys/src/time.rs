use crate::{Operation, api, current};
use simple_server_abi::{MonotonicInstant, READY};
use std::{
    future::{Future, poll_fn},
    mem::MaybeUninit,
    ops,
    pin::{Pin, pin},
    task::{Context, Poll},
    time::Duration,
};

/// An engine-local monotonic timestamp. Paused runtimes use their virtual clock.
/// Outside an engine callback, `now` reads the real monotonic clock. Values are
/// process-local and must not be persisted or compared across clock domains.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Instant(i128);
impl Instant {
    pub fn now() -> Self {
        let runtime = current().ok();
        let pointer = runtime
            .as_ref()
            .map_or(std::ptr::null_mut(), |runtime| runtime.0.as_ptr());
        let mut output = MaybeUninit::uninit();
        let result = unsafe { (api().clock_now)(pointer, output.as_mut_ptr()) };
        assert_eq!(
            result,
            READY,
            "engine clock failed: {}",
            crate::last_error()
        );
        let value = unsafe { output.assume_init() };
        Self(i128::from(value.seconds) * 1_000_000_000 + i128::from(value.nanoseconds))
    }
    fn wire(self) -> Option<MonotonicInstant> {
        Some(MonotonicInstant {
            seconds: self.0.div_euclid(1_000_000_000).try_into().ok()?,
            nanoseconds: self.0.rem_euclid(1_000_000_000) as u32,
        })
    }
    fn checked(value: i128) -> Option<Self> {
        let instant = Self(value);
        let stamp = instant.wire()?;
        (unsafe { (api().clock_valid)(stamp) } != 0).then_some(instant)
    }
    pub fn checked_add(self, duration: Duration) -> Option<Self> {
        Self::checked(self.0.checked_add(duration.as_nanos() as i128)?)
    }
    pub fn checked_sub(self, duration: Duration) -> Option<Self> {
        Self::checked(self.0.checked_sub(duration.as_nanos() as i128)?)
    }
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        let nanos = self.0.checked_sub(earlier.0)?;
        if nanos < 0 {
            return None;
        }
        Some(Duration::new(
            (nanos / 1_000_000_000).try_into().ok()?,
            (nanos % 1_000_000_000) as u32,
        ))
    }
    pub fn duration_since(self, earlier: Self) -> Duration {
        self.checked_duration_since(earlier).unwrap_or_default()
    }
    pub fn saturating_duration_since(self, earlier: Self) -> Duration {
        self.duration_since(earlier)
    }
    pub fn elapsed(self) -> Duration {
        Self::now().duration_since(self)
    }
}
impl ops::Add<Duration> for Instant {
    type Output = Self;
    fn add(self, rhs: Duration) -> Self {
        self.checked_add(rhs).expect("engine instant overflow")
    }
}
impl ops::Sub<Duration> for Instant {
    type Output = Self;
    fn sub(self, rhs: Duration) -> Self {
        self.checked_sub(rhs).expect("engine instant overflow")
    }
}
impl ops::Sub for Instant {
    type Output = Duration;
    fn sub(self, rhs: Self) -> Duration {
        self.duration_since(rhs)
    }
}
impl ops::AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}
impl ops::SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

/// An engine timer whose deadline is fixed at construction, not at first poll.
pub struct Sleep {
    operation: Operation,
    deadline: Instant,
    complete: bool,
}
impl Sleep {
    pub fn deadline(&self) -> Instant {
        self.deadline
    }
}
impl Future for Sleep {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.complete {
            return Poll::Ready(());
        }
        match Pin::new(&mut self.operation).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                result.expect("engine timer failed");
                self.complete = true;
                Poll::Ready(())
            }
        }
    }
}
pub fn sleep(duration: Duration) -> Sleep {
    sleep_until(Instant::now() + duration)
}
pub fn sleep_until(deadline: Instant) -> Sleep {
    let stamp = deadline.wire().expect("invalid engine deadline");
    let command = format!(
        "{{\"op\":\"sleep_until\",\"seconds\":{},\"nanoseconds\":{}}}",
        stamp.seconds, stamp.nanoseconds
    );
    Sleep {
        operation: Operation::new(command.as_bytes()).expect("sleep requires an engine runtime"),
        deadline,
        complete: false,
    }
}
pub fn timeout<F: Future>(
    duration: Duration,
    future: F,
) -> impl Future<Output = Result<F::Output, crate::Elapsed>> {
    timeout_at(Instant::now() + duration, future)
}
pub fn timeout_at<F: Future>(
    deadline: Instant,
    future: F,
) -> impl Future<Output = Result<F::Output, crate::Elapsed>> {
    let timer = sleep_until(deadline);
    async move {
        let mut future = pin!(future);
        let mut timer = pin!(timer);
        poll_fn(|cx| {
            if let Poll::Ready(value) = future.as_mut().poll(cx) {
                return Poll::Ready(Ok(value));
            }
            timer.as_mut().poll(cx).map(|()| Err(crate::Elapsed))
        })
        .await
    }
}
