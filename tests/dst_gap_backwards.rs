#![cfg(any(feature = "chrono", feature = "jiff"))]

use croner::Cron;
use std::str::FromStr;

#[cfg(feature = "chrono")]
mod chrono_tests {
    use chrono::{DateTime, TimeZone};
    use chrono_tz::Tz;
    use croner::errors::CronError;

    use super::*;

    fn stockholm() -> Tz {
        "Europe/Stockholm".parse().expect("known zone")
    }

    fn zoned(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> DateTime<Tz> {
        stockholm()
            .with_ymd_and_hms(year, month, day, hour, minute, second)
            .unwrap()
    }

    #[test]
    fn fixed_time_gap_search_uses_the_post_gap_instant() -> Result<(), CronError> {
        let cron = Cron::from_str("0 30 2 * * *")?;
        // 02:30 never occurs on this date, so the fixed-time run is carried by
        // the first real instant after the gap.
        let gap_run = zoned(2025, 3, 30, 3, 0, 0);

        assert_eq!(
            cron.find_previous_occurrence(&zoned(2025, 3, 30, 4, 0, 0), false)?,
            gap_run
        );
        assert_eq!(
            cron.find_previous_occurrence(&gap_run, false)?,
            zoned(2025, 3, 29, 2, 30, 0)
        );
        Ok(())
    }

    #[test]
    fn backward_iteration_crosses_the_gap_without_inventing_times() -> Result<(), CronError> {
        let cron = Cron::from_str("0 * * * * *")?;

        assert_eq!(
            cron.iter_before(zoned(2025, 3, 30, 3, 2, 0))
                .take(4)
                .collect::<Vec<_>>(),
            vec![
                zoned(2025, 3, 30, 3, 1, 0),
                zoned(2025, 3, 30, 3, 0, 0),
                zoned(2025, 3, 30, 1, 59, 0),
                zoned(2025, 3, 30, 1, 58, 0),
            ]
        );
        Ok(())
    }

    #[test]
    fn a_gap_across_midnight_cannot_trap_a_backward_search() -> Result<(), CronError> {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let toronto: Tz = "America/Toronto".parse().expect("known zone");
            let cron = Cron::from_str("0 45 23 30 3 *").expect("valid pattern");
            let backward = cron.find_previous_occurrence(
                &toronto.with_ymd_and_hms(1919, 4, 15, 12, 0, 0).unwrap(),
                false,
            );
            let forward = cron.find_next_occurrence(
                &toronto.with_ymd_and_hms(1919, 3, 25, 12, 0, 0).unwrap(),
                false,
            );
            sender.send((backward, forward)).ok();
        });

        let (backward, forward) = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("a search trapped in the midnight-crossing gap");

        let toronto: Tz = "America/Toronto".parse().expect("known zone");
        assert_eq!(
            backward?,
            toronto.with_ymd_and_hms(1918, 3, 30, 23, 45, 0).unwrap()
        );
        assert_eq!(
            forward?,
            toronto.with_ymd_and_hms(1920, 3, 30, 23, 45, 0).unwrap()
        );
        Ok(())
    }
}

#[cfg(feature = "jiff")]
mod jiff_tests {
    use croner::errors::CronError;
    use jiff::civil::date;
    use jiff::tz::TimeZone;
    use jiff::Zoned;

    use super::*;

    fn stockholm() -> TimeZone {
        TimeZone::get("Europe/Stockholm").expect("known zone")
    }

    fn zoned(year: i16, month: i8, day: i8, hour: i8, minute: i8, second: i8) -> Zoned {
        stockholm()
            .to_zoned(date(year, month, day).at(hour, minute, second, 0))
            .unwrap()
    }

    #[test]
    fn fixed_time_gap_search_uses_the_post_gap_instant() -> Result<(), CronError> {
        let cron = Cron::from_str("0 30 2 * * *")?;
        // 02:30 never occurs on this date, so the fixed-time run is carried by
        // the first real instant after the gap.
        let gap_run = zoned(2025, 3, 30, 3, 0, 0);

        assert_eq!(
            cron.find_previous_occurrence(&zoned(2025, 3, 30, 4, 0, 0), false)?,
            gap_run
        );
        assert_eq!(
            cron.find_previous_occurrence(&gap_run, false)?,
            zoned(2025, 3, 29, 2, 30, 0)
        );
        Ok(())
    }

    #[test]
    fn backward_iteration_crosses_the_gap_without_inventing_times() -> Result<(), CronError> {
        let cron = Cron::from_str("0 * * * * *")?;

        assert_eq!(
            cron.iter_before(zoned(2025, 3, 30, 3, 2, 0))
                .take(4)
                .collect::<Vec<_>>(),
            vec![
                zoned(2025, 3, 30, 3, 1, 0),
                zoned(2025, 3, 30, 3, 0, 0),
                zoned(2025, 3, 30, 1, 59, 0),
                zoned(2025, 3, 30, 1, 58, 0),
            ]
        );
        Ok(())
    }

    #[test]
    fn a_gap_across_midnight_cannot_trap_a_backward_search() -> Result<(), CronError> {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let toronto = TimeZone::get("America/Toronto").expect("known zone");
            let cron = Cron::from_str("0 45 23 30 3 *").expect("valid pattern");
            let backward = cron.find_previous_occurrence(
                &toronto
                    .to_zoned(date(1919, 4, 15).at(12, 0, 0, 0))
                    .expect("valid instant"),
                false,
            );
            let forward = cron.find_next_occurrence(
                &toronto
                    .to_zoned(date(1919, 3, 25).at(12, 0, 0, 0))
                    .expect("valid instant"),
                false,
            );
            sender.send((backward, forward)).ok();
        });

        let (backward, forward) = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("a search trapped in the midnight-crossing gap");

        let toronto = TimeZone::get("America/Toronto").expect("known zone");
        assert_eq!(
            backward?,
            toronto
                .to_zoned(date(1918, 3, 30).at(23, 45, 0, 0))
                .expect("valid instant")
        );
        assert_eq!(
            forward?,
            toronto
                .to_zoned(date(1920, 3, 30).at(23, 45, 0, 0))
                .expect("valid instant")
        );
        Ok(())
    }
}
