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
        stockholm().to_zoned(date(year, month, day).at(hour, minute, second, 0)).unwrap()
    }

    #[test]
    fn fixed_time_gap_search_uses_the_post_gap_instant() -> Result<(), CronError> {
        let cron = Cron::from_str("0 30 2 * * *")?;
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
}
