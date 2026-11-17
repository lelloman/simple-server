//! A self-contained UTC cron evaluator.
//!
//! Expressions have six fields (second, minute, hour, day of month, month,
//! day of week) and an optional seventh year field. Each field accepts `*`,
//! values, `a-b` ranges, `/step` periods and comma-separated lists. Months and
//! days of the week also accept English names (`jan`, `january`, `mon`, ...)
//! in any case, and the two day fields accept `?` as a synonym for `*`. Days of
//! the week are numbered 1 (Sunday) to 7 (Saturday) and years 1970 to 2100.
//! An occurrence must match the day of month *and* the day of the week.
//!
//! These rules match the `cron` 0.15 crate this module replaces, without its
//! calendar dependency.

use std::time::{Duration, SystemTime};

/// One parsed field: the allowed values within `min..=max`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Field {
    min: u32,
    allowed: Vec<bool>,
}

impl Field {
    fn contains(&self, value: u32) -> bool {
        value
            .checked_sub(self.min)
            .and_then(|index| self.allowed.get(index as usize))
            .copied()
            .unwrap_or(false)
    }

    /// The smallest allowed value at or above `value`.
    fn next_from(&self, value: u32) -> Option<u32> {
        let start = value.saturating_sub(self.min) as usize;
        self.allowed
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, allowed)| **allowed)
            .map(|(index, _)| self.min + index as u32)
    }
}

/// The rules for one field position.
struct Unit {
    name: &'static str,
    min: u32,
    max: u32,
    names: &'static [(&'static str, u32)],
    any: bool,
}

const MONTH_NAMES: &[(&str, u32)] = &[
    ("jan", 1),
    ("january", 1),
    ("feb", 2),
    ("february", 2),
    ("mar", 3),
    ("march", 3),
    ("apr", 4),
    ("april", 4),
    ("may", 5),
    ("jun", 6),
    ("june", 6),
    ("jul", 7),
    ("july", 7),
    ("aug", 8),
    ("august", 8),
    ("sep", 9),
    ("september", 9),
    ("oct", 10),
    ("october", 10),
    ("nov", 11),
    ("november", 11),
    ("dec", 12),
    ("december", 12),
];

const WEEKDAY_NAMES: &[(&str, u32)] = &[
    ("sun", 1),
    ("sunday", 1),
    ("mon", 2),
    ("monday", 2),
    ("tue", 3),
    ("tues", 3),
    ("tuesday", 3),
    ("wed", 4),
    ("wednesday", 4),
    ("thu", 5),
    ("thurs", 5),
    ("thursday", 5),
    ("fri", 6),
    ("friday", 6),
    ("sat", 7),
    ("saturday", 7),
];

const UNITS: [Unit; 7] = [
    Unit {
        name: "seconds",
        min: 0,
        max: 59,
        names: &[],
        any: false,
    },
    Unit {
        name: "minutes",
        min: 0,
        max: 59,
        names: &[],
        any: false,
    },
    Unit {
        name: "hours",
        min: 0,
        max: 23,
        names: &[],
        any: false,
    },
    Unit {
        name: "days of month",
        min: 1,
        max: 31,
        names: &[],
        any: true,
    },
    Unit {
        name: "months",
        min: 1,
        max: 12,
        names: MONTH_NAMES,
        any: false,
    },
    Unit {
        name: "days of week",
        min: 1,
        max: 7,
        names: WEEKDAY_NAMES,
        any: true,
    },
    Unit {
        name: "years",
        min: 1970,
        max: 2100,
        names: &[],
        any: false,
    },
];

impl Unit {
    fn number(&self, text: &str) -> Result<u32, String> {
        text.parse::<u32>()
            .map_err(|_| format!("invalid {} value '{text}'", self.name))
    }

    fn named(&self, text: &str) -> Result<u32, String> {
        if self.names.is_empty() {
            return Err(format!("{} do not support names ('{text}')", self.name));
        }
        let lower = text.to_ascii_lowercase();
        self.names
            .iter()
            .find(|(name, _)| *name == lower)
            .map(|(_, value)| *value)
            .ok_or_else(|| format!("'{text}' is not a valid name for {}", self.name))
    }

    fn validate(&self, value: u32) -> Result<u32, String> {
        if (self.min..=self.max).contains(&value) {
            Ok(value)
        } else {
            Err(format!(
                "{} must be between {} and {} ('{value}' specified)",
                self.name, self.min, self.max
            ))
        }
    }

    fn is_wildcard(&self, text: &str) -> bool {
        text == "*" || (self.any && text == "?")
    }

    /// A range body: `*`, `a-b` with numbers, or `a-b` with names.
    fn range(&self, text: &str) -> Result<Option<(u32, u32)>, String> {
        if self.is_wildcard(text) {
            return Ok(Some((self.min, self.max)));
        }
        let Some((start, end)) = text.split_once('-') else {
            return Ok(None);
        };
        let numeric =
            |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
        let alphabetic =
            |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_alphabetic());
        let (start, end) = if numeric(start) && numeric(end) {
            (self.number(start)?, self.number(end)?)
        } else if alphabetic(start) && alphabetic(end) {
            (self.named(start)?, self.named(end)?)
        } else {
            return Err(format!("invalid {} range '{text}'", self.name));
        };
        match (self.validate(start), self.validate(end)) {
            (Ok(start), Ok(end)) if start <= end => Ok(Some((start, end))),
            _ => Err(format!("invalid {} range '{text}'", self.name)),
        }
    }

    fn parse(&self, text: &str) -> Result<Field, String> {
        let mut allowed = vec![false; (self.max - self.min + 1) as usize];
        for item in text.split(',') {
            let mut mark = |value: u32| allowed[(value - self.min) as usize] = true;
            if let Some((base, step)) = item.split_once('/') {
                let step = self.number(step)?;
                if step == 0 || step > self.max {
                    return Err(format!(
                        "{} step must be between 1 and {} ('{step}' specified)",
                        self.name, self.max
                    ));
                }
                let (start, end) = match self.range(base)? {
                    Some(bounds) => bounds,
                    None if !base.is_empty() && base.bytes().all(|byte| byte.is_ascii_digit()) => {
                        (self.validate(self.number(base)?)?, self.max)
                    }
                    None => return Err(format!("invalid {} period '{item}'", self.name)),
                };
                (start..=end).step_by(step as usize).for_each(&mut mark);
            } else if let Some((start, end)) = self.range(item)? {
                (start..=end).for_each(&mut mark);
            } else if !item.is_empty() && item.bytes().all(|byte| byte.is_ascii_digit()) {
                mark(self.validate(self.number(item)?)?);
            } else if !item.is_empty() && item.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                mark(self.named(item)?);
            } else {
                return Err(format!("invalid {} value '{item}'", self.name));
            }
        }
        Ok(Field {
            min: self.min,
            allowed,
        })
    }
}

/// A parsed cron expression evaluated in UTC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Cron {
    fields: [Field; 7],
}

impl Cron {
    pub(crate) fn parse(expression: &str) -> Result<Self, String> {
        let parts = expression.split_whitespace().collect::<Vec<_>>();
        if !matches!(parts.len(), 6 | 7) {
            return Err("cron requires six or seven fields".into());
        }
        let mut fields = Vec::with_capacity(7);
        for (index, unit) in UNITS.iter().enumerate() {
            fields.push(match parts.get(index) {
                Some(text) => unit.parse(text)?,
                None => unit.parse("*")?,
            });
        }
        let fields: [Field; 7] = fields.try_into().expect("seven units");
        Ok(Self { fields })
    }

    /// The first occurrence strictly after `time`, at whole-second precision.
    pub(crate) fn next_after(&self, time: SystemTime) -> Option<SystemTime> {
        let seconds = match time.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(delta) => i64::try_from(delta.as_secs()).ok()?,
            Err(before) => {
                let delta = before.duration();
                let whole = i64::try_from(delta.as_secs()).ok()?;
                // A fractional second before the epoch floors to the next lower second.
                if delta.subsec_nanos() > 0 {
                    -whole - 1
                } else {
                    -whole
                }
            }
        };
        let next = self.next_from(seconds.checked_add(1)?)?;
        SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(u64::try_from(next).ok()?))
    }

    /// The first matching whole second at or after `start` (seconds since the epoch).
    fn next_from(&self, start: i64) -> Option<i64> {
        let [seconds, minutes, hours, days, months, weekdays, years] = &self.fields;
        let (mut year, mut month, mut day) = civil_from_days(start.div_euclid(86_400));
        let mut clock = start.rem_euclid(86_400) as u32;
        let (mut hour, mut minute, mut second) = (clock / 3_600, clock / 60 % 60, clock % 60);
        if year < 1970 {
            (year, month, day, hour, minute, second) = (1970, 1, 1, 0, 0, 0);
        }
        loop {
            if year > 2100 {
                return None;
            }
            let year_u32 = year as u32;
            if !years.contains(year_u32) {
                year = i64::from(years.next_from(year_u32)?);
                (month, day, hour, minute, second) = (1, 1, 0, 0, 0);
                continue;
            }
            let Some(next_month) = months.next_from(month) else {
                (year, month, day, hour, minute, second) = (year + 1, 1, 1, 0, 0, 0);
                continue;
            };
            if next_month != month {
                (month, day, hour, minute, second) = (next_month, 1, 0, 0, 0);
            }
            let Some(next_day) = days
                .next_from(day)
                .filter(|candidate| *candidate <= days_in_month(year, month))
            else {
                (month, day, hour, minute, second) = (month + 1, 1, 0, 0, 0);
                if month > 12 {
                    (year, month) = (year + 1, 1);
                }
                continue;
            };
            if next_day != day {
                (day, hour, minute, second) = (next_day, 0, 0, 0);
            }
            let ordinal = days_from_civil(year, month, day);
            if !weekdays.contains(weekday_from_sunday(ordinal)) {
                (day, hour, minute, second) = (day + 1, 0, 0, 0);
                if day > days_in_month(year, month) {
                    (month, day) = (month + 1, 1);
                    if month > 12 {
                        (year, month) = (year + 1, 1);
                    }
                }
                continue;
            }
            let Some(next_hour) = hours.next_from(hour) else {
                (day, hour, minute, second) = (day + 1, 0, 0, 0);
                if day > days_in_month(year, month) {
                    (month, day) = (month + 1, 1);
                    if month > 12 {
                        (year, month) = (year + 1, 1);
                    }
                }
                continue;
            };
            if next_hour != hour {
                (hour, minute, second) = (next_hour, 0, 0);
            }
            let Some(next_minute) = minutes.next_from(minute) else {
                (hour, minute, second) = (hour + 1, 0, 0);
                if hour > 23 {
                    (day, hour) = (day + 1, 0);
                    if day > days_in_month(year, month) {
                        (month, day) = (month + 1, 1);
                        if month > 12 {
                            (year, month) = (year + 1, 1);
                        }
                    }
                }
                continue;
            };
            if next_minute != minute {
                (minute, second) = (next_minute, 0);
            }
            let Some(next_second) = seconds.next_from(second) else {
                (minute, second) = (minute + 1, 0);
                if minute > 59 {
                    (hour, minute) = (hour + 1, 0);
                    if hour > 23 {
                        (day, hour) = (day + 1, 0);
                        if day > days_in_month(year, month) {
                            (month, day) = (month + 1, 1);
                            if month > 12 {
                                (year, month) = (year + 1, 1);
                            }
                        }
                    }
                }
                continue;
            };
            clock = hour * 3_600 + minute * 60 + next_second;
            return Some(ordinal * 86_400 + i64::from(clock));
        }
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The proleptic Gregorian date of a day count since 1970-01-01.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// 1 for Sunday through 7 for Saturday; 1970-01-01 was a Thursday.
fn weekday_from_sunday(days: i64) -> u32 {
    (days + 4).rem_euclid(7) as u32 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> SystemTime {
        if seconds >= 0 {
            SystemTime::UNIX_EPOCH + Duration::from_secs(seconds as u64)
        } else {
            SystemTime::UNIX_EPOCH - Duration::from_secs(seconds.unsigned_abs())
        }
    }

    fn epoch_seconds(time: SystemTime) -> i64 {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[test]
    fn calendar_round_trips_across_eras_and_leap_days() {
        for days in (-800_000..800_000).step_by(37) {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(days_from_civil(year, month, day), days);
            assert!(day >= 1 && day <= days_in_month(year, month));
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(weekday_from_sunday(0), 5);
        assert_eq!(weekday_from_sunday(days_from_civil(2026, 11, 15)), 1);
    }

    #[test]
    fn rejects_malformed_expressions() {
        for expression in [
            "* * * * *",
            "* * * * * * * *",
            "60 * * * * *",
            "* * 24 * * *",
            "* * * 0 * *",
            "* * * * 13 *",
            "* * * * * 8",
            "* * * * * * 1969",
            "*/0 * * * * *",
            "*/61 * * * * *",
            "5-1 * * * * *",
            "? * * * * *",
            "* * * * jan-mon *",
            "* * * * * funday",
            "mon * * * * *",
            "1,,2 * * * * *",
            "a-1 * * * * *",
            "-1 * * * * *",
            "mon/2 * * * * *",
        ] {
            assert!(Cron::parse(expression).is_err(), "{expression}");
        }
    }

    #[test]
    fn finds_the_next_occurrence_in_utc() {
        let cases = [
            (
                "0 0 * * * *",
                "2026-11-17T10:15:30Z",
                "2026-11-17T11:00:00Z",
            ),
            (
                "*/15 * * * * *",
                "2026-11-17T10:15:59Z",
                "2026-11-17T10:16:00Z",
            ),
            (
                "0 30 9 * * mon-fri",
                "2026-11-20T10:00:00Z",
                "2026-11-23T09:30:00Z",
            ),
            (
                "0 0 0 29 feb ?",
                "2026-01-01T00:00:00Z",
                "2028-02-29T00:00:00Z",
            ),
            (
                "0 0 0 31 * *",
                "2026-04-01T00:00:00Z",
                "2026-05-31T00:00:00Z",
            ),
            (
                "0 0 0 13 * fri",
                "2026-01-01T00:00:00Z",
                "2026-02-13T00:00:00Z",
            ),
            (
                "0 0 12 1 jan * 2030",
                "2026-01-01T00:00:00Z",
                "2030-01-01T12:00:00Z",
            ),
            (
                "59 59 23 31 dec * 2026",
                "2026-12-31T23:59:58Z",
                "2026-12-31T23:59:59Z",
            ),
        ];
        for (expression, after, expected) in cases {
            let cron = Cron::parse(expression).unwrap();
            let next = cron.next_after(at(parse_utc(after))).unwrap();
            assert_eq!(
                epoch_seconds(next),
                parse_utc(expected),
                "{expression} after {after}"
            );
        }
        assert!(
            Cron::parse("0 0 0 1 1 * 2026")
                .unwrap()
                .next_after(at(parse_utc("2026-06-01T00:00:00Z")))
                .is_none()
        );
        assert!(
            Cron::parse("0 0 0 30 feb *")
                .unwrap()
                .next_after(at(0))
                .is_none()
        );
    }

    #[test]
    fn fractions_and_pre_epoch_times_floor_to_the_second() {
        let cron = Cron::parse("* * * * * *").unwrap();
        let half = SystemTime::UNIX_EPOCH + Duration::from_millis(10_500);
        assert_eq!(epoch_seconds(cron.next_after(half).unwrap()), 11);
        assert_eq!(epoch_seconds(cron.next_after(at(-5)).unwrap()), 0);
        let before = SystemTime::UNIX_EPOCH - Duration::from_millis(500);
        assert_eq!(epoch_seconds(cron.next_after(before).unwrap()), 0);
    }

    fn parse_utc(text: &str) -> i64 {
        let number = |range: std::ops::Range<usize>| text[range].parse::<u32>().unwrap();
        days_from_civil(i64::from(number(0..4)), number(5..7), number(8..10)) * 86_400
            + i64::from(number(11..13) * 3_600 + number(14..16) * 60 + number(17..19))
    }
}
