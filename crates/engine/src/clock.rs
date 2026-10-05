use simple_server_abi::{INVALID, MonotonicInstant, READY};
use std::{
    ffi::c_void,
    sync::OnceLock,
    time::{Duration, Instant},
};

fn origin() -> Instant {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    *ORIGIN.get_or_init(Instant::now)
}
pub fn decode(value: MonotonicInstant) -> Option<tokio::time::Instant> {
    if value.nanoseconds >= 1_000_000_000 {
        return None;
    }
    let nanos = i128::from(value.seconds) * 1_000_000_000 + i128::from(value.nanoseconds);
    let magnitude = nanos.unsigned_abs();
    let duration = Duration::new(
        (magnitude / 1_000_000_000).try_into().ok()?,
        (magnitude % 1_000_000_000) as u32,
    );
    let origin = origin();
    if nanos < 0 {
        origin.checked_sub(duration)
    } else {
        origin.checked_add(duration)
    }
    .map(tokio::time::Instant::from_std)
}
fn encode(value: Instant) -> Result<MonotonicInstant, String> {
    let origin = origin();
    let nanos = if value >= origin {
        value.duration_since(origin).as_nanos() as i128
    } else {
        -(origin.duration_since(value).as_nanos() as i128)
    };
    Ok(MonotonicInstant {
        seconds: nanos
            .div_euclid(1_000_000_000)
            .try_into()
            .map_err(|_| "clock timestamp out of range")?,
        nanoseconds: nanos.rem_euclid(1_000_000_000) as u32,
    })
}
pub unsafe extern "C" fn now(runtime: *mut c_void, output: *mut MonotonicInstant) -> i32 {
    crate::guarded(INVALID, || {
        let now = if runtime.is_null() {
            Instant::now()
        } else {
            let runtime = unsafe { &*runtime.cast::<tokio::runtime::Runtime>() };
            let _enter = runtime.enter();
            tokio::time::Instant::now().into_std()
        };
        unsafe { output.write(encode(now)?) };
        Ok(READY)
    })
}
pub unsafe extern "C" fn valid(value: MonotonicInstant) -> u8 {
    u8::from(decode(value).is_some())
}
