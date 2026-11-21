//! Regression tests for issue #84: a fixed-time job whose wall clock time
//! falls in a gap longer than two hours must still be found. `Pacific/Apia`
//! skipped the whole of 30 December 2011: 2011-12-29 23:59:59 -10:00 was
//! followed by 2011-12-31 00:00:00 +14:00.

#![cfg(any(feature = "chrono", feature = "jiff"))]

use croner::Cron;
use std::str::FromStr;

const APIA: &str = "Pacific/Apia";

#[cfg(feature = "chrono")]
mod chrono_tests {
    use super::*;
    use chrono::{DateTime, TimeZone};
    use chrono_tz::Tz;

    fn at(y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Tz> {
        let tz: Tz = APIA.parse().expect("known zone");
        tz.with_ymd_and_hms(y, m, d, h, mi, 0).unwrap()
    }

    fn next(pattern: &str, from: DateTime<Tz>) -> DateTime<Tz> {
        Cron::from_str(pattern)
            .unwrap()
            .find_next_occurrence(&from, false)
            .unwrap()
    }

    fn previous(pattern: &str, from: DateTime<Tz>) -> DateTime<Tz> {
        Cron::from_str(pattern)
            .unwrap()
            .find_previous_occurrence(&from, false)
            .unwrap()
    }

    #[test]
    fn daily_job_in_skipped_day_runs_after_the_gap() {
        let after_gap = at(2011, 12, 31, 0, 0);
        assert_eq!(next("0 12 * * *", at(2011, 12, 29, 23, 30)), after_gap);
        assert_eq!(previous("0 12 * * *", at(2011, 12, 31, 0, 30)), after_gap);
    }

    #[test]
    fn daily_midnight_job_crosses_the_gap() {
        let after_gap = at(2011, 12, 31, 0, 0);
        assert_eq!(next("0 0 * * *", at(2011, 12, 29, 23, 30)), after_gap);
        assert_eq!(previous("0 0 * * *", at(2011, 12, 31, 0, 30)), after_gap);
    }

    #[test]
    fn hourly_job_crosses_the_gap() {
        let after_gap = at(2011, 12, 31, 0, 0);
        assert_eq!(next("0 * * * *", at(2011, 12, 29, 23, 30)), after_gap);
        assert_eq!(previous("0 * * * *", at(2011, 12, 31, 0, 30)), after_gap);
    }

    #[test]
    fn job_restricted_to_the_skipped_day_skips_it() {
        assert_eq!(
            next("0 12 30 * *", at(2011, 12, 29, 23, 30)),
            at(2012, 1, 30, 12, 0)
        );
        assert_eq!(
            previous("0 12 30 * *", at(2011, 12, 31, 0, 30)),
            at(2011, 11, 30, 12, 0)
        );
    }

    #[test]
    fn backward_search_from_after_the_gap_reaches_the_day_before() {
        // 2011-12-31 00:00 is the moved 30 December run; the one before it
        // is the regular 29 December run.
        assert_eq!(
            previous("0 12 * * *", at(2011, 12, 31, 0, 0)),
            at(2011, 12, 29, 12, 0)
        );
    }

    #[test]
    fn iteration_runs_through_the_gap() {
        let cron = Cron::from_str("0 12 * * *").unwrap();
        assert_eq!(
            cron.iter_after(at(2011, 12, 28, 13, 0))
                .take(3)
                .collect::<Vec<_>>(),
            vec![
                at(2011, 12, 29, 12, 0),
                at(2011, 12, 31, 0, 0),
                at(2011, 12, 31, 12, 0),
            ]
        );
    }
}

#[cfg(feature = "jiff")]
mod jiff_tests {
    use super::*;
    use jiff::{civil::date, tz::TimeZone, ZonedDateTime};

    fn at(y: i16, m: i8, d: i8, h: i8, mi: i8) -> ZonedDateTime {
        date(y, m, d)
            .at(h, mi, 0, 0)
            .to_zoned_datetime(TimeZone::get(APIA).unwrap())
            .unwrap()
    }

    fn next(pattern: &str, from: &ZonedDateTime) -> ZonedDateTime {
        Cron::from_str(pattern)
            .unwrap()
            .find_next_occurrence(from, false)
            .unwrap()
    }

    fn previous(pattern: &str, from: &ZonedDateTime) -> ZonedDateTime {
        Cron::from_str(pattern)
            .unwrap()
            .find_previous_occurrence(from, false)
            .unwrap()
    }

    #[test]
    fn daily_job_in_skipped_day_runs_after_the_gap() {
        let after_gap = at(2011, 12, 31, 0, 0);
        assert_eq!(next("0 12 * * *", &at(2011, 12, 29, 23, 30)), after_gap);
        assert_eq!(previous("0 12 * * *", &at(2011, 12, 31, 0, 30)), after_gap);
    }

    #[test]
    fn daily_midnight_job_crosses_the_gap() {
        let after_gap = at(2011, 12, 31, 0, 0);
        assert_eq!(next("0 0 * * *", &at(2011, 12, 29, 23, 30)), after_gap);
        assert_eq!(previous("0 0 * * *", &at(2011, 12, 31, 0, 30)), after_gap);
    }

    #[test]
    fn job_restricted_to_the_skipped_day_skips_it() {
        assert_eq!(
            next("0 12 30 * *", &at(2011, 12, 29, 23, 30)),
            at(2012, 1, 30, 12, 0)
        );
        assert_eq!(
            previous("0 12 30 * *", &at(2011, 12, 31, 0, 30)),
            at(2011, 11, 30, 12, 0)
        );
    }

    #[test]
    fn backward_search_from_after_the_gap_reaches_the_day_before() {
        assert_eq!(
            previous("0 12 * * *", &at(2011, 12, 31, 0, 0)),
            at(2011, 12, 29, 12, 0)
        );
    }
}
