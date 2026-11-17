#![cfg(feature = "task-scheduling")]

//! The built-in cron evaluator must accept, reject and schedule exactly like the
//! `cron` 0.15 crate it replaced. This compares both over generated
//! expressions and instants; the crate is only a development dependency.

use std::str::FromStr;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use simple_server::task_scheduling::CronSchedule;

/// A small deterministic generator (SplitMix64) so failures reproduce.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len() as u64) as usize]
    }
}

const MONTHS: &[&str] = &[
    "jan", "FEB", "Mar", "april", "may", "jun", "July", "aug", "sep", "oct", "nov", "december",
    "smarch",
];
const DAYS: &[&str] = &[
    "sun",
    "MON",
    "tue",
    "tues",
    "Wednesday",
    "thu",
    "thurs",
    "fri",
    "sat",
    "funday",
];

/// One field, sometimes slightly out of range so rejection is compared too.
fn field(random: &mut Random, min: u64, max: u64, names: &[&str], any: bool) -> String {
    let span = max - min + 1;
    // Mostly valid values, occasionally one past either end.
    let mut value = |random: &mut Random| {
        if random.below(25) == 0 {
            if random.below(2) == 0 {
                min.saturating_sub(1)
            } else {
                max + 1
            }
        } else {
            min + random.below(span)
        }
    };
    let single = |random: &mut Random, value: &mut dyn FnMut(&mut Random) -> u64| -> String {
        match random.below(9) {
            0 => "*".to_owned(),
            1 if any => "?".to_owned(),
            2 => {
                let (a, b) = (value(random), value(random));
                format!("{a}-{b}")
            }
            3 => format!("*/{}", 1 + random.below(max.min(40) + 2)),
            4 => format!("{}/{}", value(random), 1 + random.below(max.min(40) + 2)),
            5 => {
                let (a, b) = (value(random), value(random));
                format!("{}-{}/{}", a.min(b), a.max(b), 1 + random.below(12))
            }
            6 if !names.is_empty() => random.pick(names).to_owned(),
            7 if !names.is_empty() => format!("{}-{}", random.pick(names), random.pick(names)),
            8 if any => format!("?/{}", 1 + random.below(10)),
            _ => value(random).to_string(),
        }
    };
    let count = if random.below(4) == 0 {
        2 + random.below(3)
    } else {
        1
    };
    (0..count)
        .map(|_| single(random, &mut value))
        .collect::<Vec<_>>()
        .join(",")
}

fn expression(random: &mut Random) -> String {
    let mut fields = vec![
        field(random, 0, 59, &[], false),
        field(random, 0, 59, &[], false),
        field(random, 0, 23, &[], false),
        field(random, 1, 31, &[], true),
        field(random, 1, 12, MONTHS, false),
        field(random, 1, 7, DAYS, true),
    ];
    if random.below(4) == 0 {
        let start = 1_969 + random.below(140);
        fields.push(if random.below(2) == 0 {
            start.to_string()
        } else {
            format!("{start}-{}", start + random.below(10))
        });
    }
    fields.join(" ")
}

fn reference_next(schedule: &cron::Schedule, time: SystemTime) -> Option<SystemTime> {
    let date: DateTime<Utc> = time.into();
    schedule.after(&date).next().map(Into::into)
}

#[test]
fn matches_the_cron_crate_on_generated_expressions_and_instants() {
    let mut random = Random(0x5EED_CA11);
    let (mut accepted, mut rejected, mut compared) = (0, 0, 0);
    for _ in 0..4_000 {
        let expression = expression(&mut random);
        let ours = CronSchedule::parse(&expression);
        let reference = cron::Schedule::from_str(&expression);
        assert_eq!(
            ours.is_ok(),
            reference.is_ok(),
            "acceptance differs for '{expression}': {:?}",
            ours.err()
        );
        let (Ok(ours), Ok(reference)) = (ours, reference) else {
            rejected += 1;
            continue;
        };
        accepted += 1;
        for _ in 0..6 {
            // 1960 to 2110, with sub-second offsets.
            let seconds = random.below(4_733_000_000) as i64 - 315_619_200;
            let nanos = if random.below(3) == 0 {
                random.below(1_000_000_000) as u32
            } else {
                0
            };
            let time = if seconds >= 0 {
                SystemTime::UNIX_EPOCH + Duration::new(seconds as u64, nanos)
            } else {
                SystemTime::UNIX_EPOCH - Duration::from_secs(seconds.unsigned_abs())
                    + Duration::new(0, nanos)
            };
            assert_eq!(
                ours.next_after(time),
                reference_next(&reference, time),
                "next after {time:?} differs for '{expression}'"
            );
            compared += 1;
        }
    }
    assert!(
        accepted > 1_000 && rejected > 100 && compared > 6_000,
        "{accepted} {rejected} {compared}"
    );
}

#[test]
fn matches_the_cron_crate_on_common_schedules() {
    let expressions = [
        "* * * * * *",
        "0 * * * * *",
        "0 0 * * * *",
        "0 */5 * * * *",
        "0 30 2 * * *",
        "0 0 0 * * mon",
        "0 0 9-17 * * mon-fri",
        "0 0 0 1 * *",
        "0 0 0 1 1 *",
        "0 0 0 29 2 *",
        "0 0 0 31 * *",
        "0 15,45 */3 * * sat,sun",
        "30 0 0 13 * fri",
        "0 0 12 ? jan-mar ?",
        "0 0 0 1 1 * 2099-2100",
    ];
    let start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_225_600); // 2026-01-01 UTC
    for expression in expressions {
        let ours = CronSchedule::parse(expression).unwrap();
        let reference = cron::Schedule::from_str(expression).unwrap();
        let (mut mine, mut theirs) = (start, start);
        for step in 0..200 {
            let (next_mine, next_theirs) =
                (ours.next_after(mine), reference_next(&reference, theirs));
            assert_eq!(next_mine, next_theirs, "'{expression}' step {step}");
            let Some(next) = next_mine else { break };
            (mine, theirs) = (next, next);
        }
    }
}
