use super::ConfigError;
use chrono::{DateTime, Utc};
use std::{
    str::FromStr,
    time::{Duration, SystemTime},
};

/// Parsed UTC cron. Six fields (seconds first), with an optional seventh year.
#[derive(Clone, Debug)]
pub struct CronSchedule(Box<cron::Schedule>);
impl CronSchedule {
    pub fn parse(expression: &str) -> Result<Self, ConfigError> {
        if !matches!(expression.split_whitespace().count(), 6 | 7) {
            return Err(ConfigError("cron requires six or seven fields".into()));
        }
        cron::Schedule::from_str(expression)
            .map(|schedule| Self(Box::new(schedule)))
            .map_err(|e| ConfigError(e.to_string()))
    }
    pub fn next_after(&self, time: SystemTime) -> Option<SystemTime> {
        let epoch = DateTime::<Utc>::UNIX_EPOCH;
        let date = match time.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(delta) => epoch.checked_add_signed(chrono::Duration::from_std(delta).ok()?)?,
            Err(delta) => {
                epoch.checked_sub_signed(chrono::Duration::from_std(delta.duration()).ok()?)?
            }
        };
        self.0.after(&date).next().map(Into::into)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FirstRun {
    Immediately,
    #[default]
    AfterInterval,
}
#[derive(Clone, Debug)]
pub enum Schedule {
    FixedRate {
        every: Duration,
        first: FirstRun,
    },
    FixedDelay {
        every: Duration,
        jitter: Duration,
        first: FirstRun,
    },
    Cron(CronSchedule),
}
impl Schedule {
    pub fn validate(&self) -> Result<(), ConfigError> {
        match self {
            Self::FixedRate { every, .. } | Self::FixedDelay { every, .. } if every.is_zero() => {
                Err(ConfigError("interval must be nonzero".into()))
            }
            Self::FixedDelay { every, jitter, .. } if every.checked_add(*jitter).is_none() => {
                Err(ConfigError("interval plus jitter overflows".into()))
            }
            _ => Ok(()),
        }
    }
    /// Pure schedule calculation. `jitter` is a caller-supplied sample in [0, 1].
    pub fn first_after(&self, now: SystemTime, jitter: f64) -> Option<SystemTime> {
        match self {
            Self::FixedRate {
                first: FirstRun::Immediately,
                ..
            }
            | Self::FixedDelay {
                first: FirstRun::Immediately,
                ..
            } => Some(now),
            Self::Cron(cron) => cron.next_after(now),
            _ => self.after_completion(now, jitter),
        }
    }
    /// Fixed-delay recurrence; manual triggers do not call this helper implicitly.
    pub fn after_completion(&self, now: SystemTime, jitter: f64) -> Option<SystemTime> {
        match self {
            Self::FixedRate { every, .. } => now.checked_add(*every),
            Self::FixedDelay {
                every,
                jitter: maximum,
                ..
            } => {
                let sample = if jitter.is_finite() {
                    jitter.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                now.checked_add(
                    every.checked_add(Duration::new(
                        ((maximum.as_nanos() as f64 * sample) as u128)
                            .min(maximum.as_nanos())
                            .div_euclid(1_000_000_000) as u64,
                        (((maximum.as_nanos() as f64 * sample) as u128).min(maximum.as_nanos())
                            % 1_000_000_000) as u32,
                    ))?,
                )
            }
            Self::Cron(cron) => cron.next_after(now),
        }
    }
    /// Skip missed occurrences; never emit a backlog of historical ticks.
    pub fn after_tick(&self, previous: SystemTime, now: SystemTime) -> Option<SystemTime> {
        match self {
            Self::FixedRate { every, .. } => {
                if every.is_zero() {
                    return None;
                }
                let elapsed = now.duration_since(previous).unwrap_or_default().as_nanos();
                let jumps = elapsed / every.as_nanos() + 1;
                let nanos = every.as_nanos().checked_mul(jumps)?;
                previous.checked_add(Duration::new(
                    (nanos / 1_000_000_000).try_into().ok()?,
                    (nanos % 1_000_000_000) as u32,
                ))
            }
            Self::Cron(cron) => cron.next_after(now),
            Self::FixedDelay { .. } => None,
        }
    }
}
