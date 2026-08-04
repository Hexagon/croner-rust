//! Tests for the `jiff` backend.
//!
//! The daylight saving time cases mirror the `chrono` tests in `src/lib.rs`, so
//! that both backends are held to the same behaviour.

#![cfg(feature = "jiff")]

use std::str::FromStr as _;

use croner::errors::CronError;
use croner::{Cron, Direction};
use jiff::civil::{date, DateTime};
use jiff::tz::TimeZone;
use jiff::Zoned;

/// Builds a zoned time, using the earlier instant when the wall clock time
/// happens twice.
fn zoned(tz: &TimeZone, y: i16, m: i8, d: i8, hh: i8, mm: i8, ss: i8) -> Zoned {
    tz.to_ambiguous_zoned(date(y, m, d).at(hh, mm, ss, 0))
        .earlier()
        .expect("a valid wall clock time")
}

/// Builds both instants of a wall clock time that happens twice.
fn zoned_pair(tz: &TimeZone, y: i16, m: i8, d: i8, hh: i8, mm: i8, ss: i8) -> (Zoned, Zoned) {
    let ambiguous = tz.to_ambiguous_zoned(date(y, m, d).at(hh, mm, ss, 0));
    (
        ambiguous.clone().earlier().expect("an earlier instant"),
        ambiguous.later().expect("a later instant"),
    )
}

fn stockholm() -> TimeZone {
    TimeZone::get("Europe/Stockholm").expect("the tzdb to know Europe/Stockholm")
}

#[test]
fn matches_a_zoned_time() -> Result<(), CronError> {
    let tz = stockholm();
    let cron = Cron::from_str("0 9 1 1 *")?;

    assert!(cron.is_time_matching(&zoned(&tz, 2023, 1, 1, 9, 0, 0))?);
    assert!(!cron.is_time_matching(&zoned(&tz, 2023, 1, 1, 10, 0, 0))?);
    Ok(())
}

#[test]
fn finds_the_next_and_previous_occurrence() -> Result<(), CronError> {
    let tz = stockholm();
    let cron = Cron::from_str("0 0 * * FRI")?;
    let start = zoned(&tz, 2024, 1, 3, 12, 0, 0); // A Wednesday

    assert_eq!(
        cron.find_next_occurrence(&start, false)?,
        zoned(&tz, 2024, 1, 5, 0, 0, 0)
    );
    assert_eq!(
        cron.find_previous_occurrence(&start, false)?,
        zoned(&tz, 2023, 12, 29, 0, 0, 0)
    );
    Ok(())
}

#[test]
fn iterates_forwards_and_backwards() -> Result<(), CronError> {
    let tz = stockholm();
    let cron = Cron::from_str("0 0 * * MON")?;
    let start = zoned(&tz, 2022, 2, 28, 23, 59, 0);

    let forward: Vec<Zoned> = cron.iter_after(start.clone()).take(3).collect();
    assert_eq!(
        forward,
        vec![
            zoned(&tz, 2022, 3, 7, 0, 0, 0),
            zoned(&tz, 2022, 3, 14, 0, 0, 0),
            zoned(&tz, 2022, 3, 21, 0, 0, 0),
        ]
    );

    let backward: Vec<Zoned> = cron.iter_before(start).take(2).collect();
    assert_eq!(
        backward,
        vec![
            zoned(&tz, 2022, 2, 28, 0, 0, 0),
            zoned(&tz, 2022, 2, 21, 0, 0, 0),
        ]
    );
    Ok(())
}

#[test]
fn works_with_an_unzoned_civil_datetime() -> Result<(), CronError> {
    let cron = Cron::from_str("0 0 L * *")?; // Last day of the month
    let start: DateTime = date(2024, 2, 1).at(0, 0, 0, 0);

    let next: DateTime = cron.find_next_occurrence(&start, false)?;
    assert_eq!(next, date(2024, 2, 29).at(0, 0, 0, 0));
    Ok(())
}

#[test]
fn honours_the_special_day_patterns() -> Result<(), CronError> {
    let tz = stockholm();

    // Last Friday of the year.
    let cron = Cron::from_str("0 0 * * FRI#L")?;
    assert!(cron.is_time_matching(&zoned(&tz, 2023, 12, 29, 0, 0, 0))?);

    // Closest weekday to the 15th. In June 2023 the 15th is a Thursday.
    let cron = Cron::from_str("0 0 15W * *")?;
    assert!(cron.is_time_matching(&zoned(&tz, 2023, 6, 15, 0, 0, 0))?);
    assert!(!cron.is_time_matching(&zoned(&tz, 2023, 6, 16, 0, 0, 0))?);
    Ok(())
}

// --- DST Gap (Spring Forward) ---

#[test]
fn dst_gap_fixed_time_job() -> Result<(), CronError> {
    // Europe/Stockholm: 2025-03-30 02:00:00 (CET) -> 03:00:00 (CEST).
    // The hour 02:00-02:59:59 does not exist.
    let tz = stockholm();
    let cron = Cron::from_str("0 30 2 * * *")?; // 02:30:00, inside the gap
    let start = zoned(&tz, 2025, 3, 30, 1, 59, 59);

    assert_eq!(
        cron.find_next_occurrence(&start, false)?,
        zoned(&tz, 2025, 3, 30, 3, 0, 0),
        "Fixed-time job in DST gap should run at the first valid time after the gap."
    );
    Ok(())
}

#[test]
fn dst_gap_interval_job() -> Result<(), CronError> {
    let tz = stockholm();
    let start = zoned(&tz, 2025, 3, 30, 1, 59, 59);

    // Every 5 minutes: the runs inside the gap are skipped.
    let cron = Cron::from_str("0 */5 * * * *")?;
    assert_eq!(
        cron.find_next_occurrence(&start, false)?,
        zoned(&tz, 2025, 3, 30, 3, 0, 0)
    );

    // Every second: the next second is the first one after the gap.
    let cron = Cron::from_str("* * * * * *")?;
    assert_eq!(
        cron.find_next_occurrence(&start, false)?,
        zoned(&tz, 2025, 3, 30, 3, 0, 0)
    );
    Ok(())
}

// --- DST Overlap (Fall Back) ---

#[test]
fn dst_overlap_fixed_time_job_runs_once() -> Result<(), CronError> {
    // Europe/Stockholm: 2025-10-26 03:00:00 (CEST) -> 02:00:00 (CET).
    // The hour 02:00-02:59:59 happens twice.
    let tz = stockholm();
    let cron = Cron::from_str("0 30 2 * * *")?; // 02:30:00
    let start = zoned(&tz, 2025, 10, 26, 0, 0, 0);

    let (earlier, later) = zoned_pair(&tz, 2025, 10, 26, 2, 30, 0);
    assert_ne!(earlier, later, "02:30 should be ambiguous on this date");

    let mut iter = cron.iter_after(start);
    assert_eq!(
        iter.next().unwrap(),
        earlier,
        "A fixed-time job should run at the first of the two occurrences."
    );
    assert_ne!(
        iter.next().unwrap(),
        later,
        "A fixed-time job should not run again at the second occurrence."
    );
    Ok(())
}

#[test]
fn dst_overlap_interval_job_runs_twice() -> Result<(), CronError> {
    let tz = stockholm();
    let cron = Cron::from_str("0 0 */2 * * *")?; // Every two hours
    let start = zoned(&tz, 2025, 10, 26, 0, 0, 0);

    let (earlier, later) = zoned_pair(&tz, 2025, 10, 26, 2, 0, 0);

    let runs: Vec<Zoned> = cron.iter_from(start, Direction::Forward).take(4).collect();
    assert_eq!(runs[0], zoned(&tz, 2025, 10, 26, 0, 0, 0));
    assert_eq!(runs[1], earlier, "First 02:00, in CEST");
    assert_eq!(runs[2], later, "Second 02:00, in CET");
    assert_eq!(runs[3], zoned(&tz, 2025, 10, 26, 4, 0, 0));
    Ok(())
}

#[test]
fn dst_overlap_interval_job_covers_every_minute_twice() -> Result<(), CronError> {
    let tz = stockholm();
    let cron = Cron::from_str("0 * 2 * * *")?; // Every minute of the 02:00 hour
    let start = zoned(&tz, 2025, 10, 26, 1, 59, 59);

    let runs: Vec<Zoned> = cron.iter_after(start).take(120).collect();
    assert_eq!(runs.len(), 120);

    for minute in 0..60 {
        let (earlier, later) = zoned_pair(&tz, 2025, 10, 26, 2, minute as i8, 0);
        assert_eq!(
            runs[minute * 2],
            earlier,
            "Minute {minute}: CEST occurrence"
        );
        assert_eq!(
            runs[minute * 2 + 1],
            later,
            "Minute {minute}: CET occurrence"
        );
    }
    Ok(())
}

/// Both backends must produce the same instants for the same schedule.
#[cfg(feature = "chrono")]
mod parity {
    use super::*;

    use chrono::TimeZone as _;
    use chrono_tz::Tz;

    const PATTERNS: &[&str] = &[
        "0 0 * * *",
        "*/7 * * * *",
        "0 30 2 * * *",
        "0 0 */2 * * *",
        "* * * * * *",
        "0 0 L * *",
        "0 0 15W * *",
        "0 12 * * FRI#L",
        "0 0 29 2 *",
    ];

    /// Start times that sit just before a daylight saving time change, plus a
    /// plain winter date.
    const STARTS: &[(i32, u32, u32, u32, u32, u32)] = &[
        (2025, 3, 30, 1, 59, 59),  // Just before the spring forward gap
        (2025, 10, 26, 1, 59, 59), // Just before the fall back overlap
        (2024, 2, 27, 12, 0, 0),   // A leap year, no transition nearby
        (2025, 12, 31, 23, 59, 59),
    ];

    const ZONES: &[&str] = &[
        "Europe/Stockholm",
        "America/New_York",
        "Australia/Lord_Howe", // Uses a 30 minute DST shift
        "UTC",
    ];

    #[test]
    fn chrono_and_jiff_agree_on_every_occurrence() {
        for zone in ZONES {
            let chrono_tz: Tz = zone.parse().expect("a known time zone");
            let jiff_tz = TimeZone::get(zone).expect("a known time zone");

            for (year, month, day, hour, minute, second) in STARTS {
                let chrono_start = chrono_tz
                    .with_ymd_and_hms(*year, *month, *day, *hour, *minute, *second)
                    .earliest()
                    .expect("a valid start time");
                let jiff_start = zoned(
                    &jiff_tz,
                    *year as i16,
                    *month as i8,
                    *day as i8,
                    *hour as i8,
                    *minute as i8,
                    *second as i8,
                );

                for pattern in PATTERNS {
                    let cron = Cron::from_str(pattern).expect("a valid pattern");

                    for direction in [Direction::Forward, Direction::Backward] {
                        let from_chrono: Vec<i64> = cron
                            .iter_from(chrono_start, direction)
                            .take(50)
                            .map(|dt| dt.timestamp())
                            .collect();
                        let from_jiff: Vec<i64> = cron
                            .iter_from(jiff_start.clone(), direction)
                            .take(50)
                            .map(|z| z.timestamp().as_second())
                            .collect();

                        assert_eq!(
                            from_chrono, from_jiff,
                            "pattern {pattern:?} in {zone} from {chrono_start} going {direction:?}"
                        );
                    }
                }
            }
        }
    }
}
