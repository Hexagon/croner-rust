//! Regression tests for issue #67: a fixed-time job scheduled in the hour
//! before a spring-forward gap must not also run at the first instant after
//! the gap. Only a wall clock time the pattern schedules inside the gap is
//! moved past it.

#![cfg(any(feature = "chrono", feature = "jiff"))]

use croner::{Cron, Direction};
use std::str::FromStr;

#[cfg(feature = "chrono")]
mod chrono_tests {
    use super::*;
    use chrono::{DateTime, TimeZone};
    use chrono_tz::Tz;

    fn at(tz: &str, y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Tz> {
        let tz: Tz = tz.parse().expect("known zone");
        tz.with_ymd_and_hms(y, m, d, h, mi, 0).unwrap()
    }

    fn next_three(cron: &Cron, mut t: DateTime<Tz>) -> Vec<DateTime<Tz>> {
        (0..3)
            .map(|_| {
                t = cron.find_next_occurrence(&t, false).unwrap();
                t
            })
            .collect()
    }

    #[test]
    fn daily_midnight_runs_once_on_spring_forward_day() {
        let cron = Cron::from_str("0 0 * * *").unwrap();
        let london = "Europe/London";
        assert_eq!(
            next_three(&cron, at(london, 2026, 3, 28, 12, 0)),
            vec![
                at(london, 2026, 3, 29, 0, 0),
                at(london, 2026, 3, 30, 0, 0),
                at(london, 2026, 3, 31, 0, 0),
            ]
        );
    }

    #[test]
    fn start_in_hour_before_gap_moves_to_next_day() {
        let cron = Cron::from_str("0 0 * * *").unwrap();
        let london = "Europe/London";
        assert_eq!(
            cron.find_next_occurrence(&at(london, 2026, 3, 29, 0, 45), false)
                .unwrap(),
            at(london, 2026, 3, 30, 0, 0)
        );
    }

    #[test]
    fn job_in_hour_before_gap_with_seconds_runs_once() {
        let cron = Cron::from_str("0 0 1 * * *").unwrap();
        let la = "America/Los_Angeles";
        assert_eq!(
            next_three(&cron, at(la, 2026, 3, 7, 12, 0)),
            vec![
                at(la, 2026, 3, 8, 1, 0),
                at(la, 2026, 3, 9, 1, 0),
                at(la, 2026, 3, 10, 1, 0),
            ]
        );
    }

    #[test]
    fn backward_search_skips_post_gap_instant() {
        let cron = Cron::from_str("0 0 * * *").unwrap();
        let london = "Europe/London";
        assert_eq!(
            cron.find_previous_occurrence(&at(london, 2026, 3, 29, 2, 30), false)
                .unwrap(),
            at(london, 2026, 3, 29, 0, 0)
        );
        let prev: Vec<_> = cron
            .iter_from(at(london, 2026, 3, 30, 0, 0), Direction::Backward)
            .take(3)
            .collect();
        assert_eq!(
            prev,
            vec![
                at(london, 2026, 3, 30, 0, 0),
                at(london, 2026, 3, 29, 0, 0),
                at(london, 2026, 3, 28, 0, 0),
            ]
        );
    }

    #[test]
    fn job_scheduled_inside_gap_still_runs_after_it() {
        let cron = Cron::from_str("0 30 1 * * *").unwrap();
        let london = "Europe/London";
        assert_eq!(
            cron.find_next_occurrence(&at(london, 2026, 3, 29, 0, 45), false)
                .unwrap(),
            at(london, 2026, 3, 29, 2, 0)
        );
    }
}

#[cfg(feature = "jiff")]
mod jiff_tests {
    use super::*;
    use jiff::{civil::date, tz::TimeZone, Zoned};

    fn at(tz: &str, y: i16, m: i8, d: i8, h: i8, mi: i8) -> Zoned {
        date(y, m, d)
            .at(h, mi, 0, 0)
            .to_zoned(TimeZone::get(tz).unwrap())
            .unwrap()
    }

    fn next_three(cron: &Cron, mut t: Zoned) -> Vec<Zoned> {
        (0..3)
            .map(|_| {
                t = cron.find_next_occurrence(&t, false).unwrap();
                t.clone()
            })
            .collect()
    }

    #[test]
    fn daily_midnight_runs_once_on_spring_forward_day() {
        let cron = Cron::from_str("0 0 * * *").unwrap();
        let london = "Europe/London";
        assert_eq!(
            next_three(&cron, at(london, 2026, 3, 28, 12, 0)),
            vec![
                at(london, 2026, 3, 29, 0, 0),
                at(london, 2026, 3, 30, 0, 0),
                at(london, 2026, 3, 31, 0, 0),
            ]
        );
    }

    #[test]
    fn job_in_hour_before_gap_with_seconds_runs_once() {
        let cron = Cron::from_str("0 0 1 * * *").unwrap();
        let la = "America/Los_Angeles";
        assert_eq!(
            next_three(&cron, at(la, 2026, 3, 7, 12, 0)),
            vec![
                at(la, 2026, 3, 8, 1, 0),
                at(la, 2026, 3, 9, 1, 0),
                at(la, 2026, 3, 10, 1, 0),
            ]
        );
    }

    #[test]
    fn job_scheduled_inside_gap_still_runs_after_it() {
        let cron = Cron::from_str("0 30 1 * * *").unwrap();
        let london = "Europe/London";
        assert_eq!(
            cron.find_next_occurrence(&at(london, 2026, 3, 29, 0, 45), false)
                .unwrap(),
            at(london, 2026, 3, 29, 2, 0)
        );
    }
}
