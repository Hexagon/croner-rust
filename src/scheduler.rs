use crate::component::LAST_BIT;
use crate::errors::CronError;
use crate::pattern::CronPattern;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Timelike};

// Scheduler module responsible for matching times against cron patterns
pub struct CronScheduler;

impl CronScheduler {
    // Checks if the provided datetime matches the cron pattern fields.
    pub fn is_time_matching(
        cron_pattern: &CronPattern,
        time: &DateTime<Local>,
    ) -> Result<bool, CronError> {
        let last_day = if cron_pattern.days.is_special_bit_set(LAST_BIT) {
            match Self::last_day_of_month(time.year(), time.month()) {
                Ok(day) => day,
                Err(e) => return Err(e),
            }
        } else {
            0
        };

        Ok(cron_pattern.seconds.is_bit_set(time.second() as u8)
            && cron_pattern.minutes.is_bit_set(time.minute() as u8)
            && cron_pattern.hours.is_bit_set(time.hour() as u8)
            && (cron_pattern.days.is_bit_set(time.day() as u8)
                || (cron_pattern.days.is_special_bit_set(LAST_BIT) && time.day() == last_day))
            && cron_pattern.months.is_bit_set(time.month() as u8)
            && cron_pattern
                .days_of_week
                .is_bit_set(time.weekday().number_from_sunday() as u8 - 1))
    }

    // Helper function to find the last day of a given month

    // Helper function to find the last day of a given month
    fn last_day_of_month(year: i32, month: u32) -> Result<u32, CronError> {
        if month == 0 || month > 12 {
            return Err(CronError::InvalidDate);
        }

        // Create a date that should be the first day of the next month
        let next_month_year = if month == 12 { year + 1 } else { year };
        let next_month = if month == 12 { 1 } else { month + 1 };

        let next_month_date = NaiveDate::from_ymd_opt(next_month_year, next_month, 1)
            .ok_or(CronError::InvalidDate)?;

        // Subtract one day to get the last day of the given month
        let last_day_date = next_month_date
            .checked_sub_signed(Duration::days(1))
            .ok_or(CronError::InvalidDate)?;

        // Return only the day
        Ok(last_day_date.day())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_is_time_matching() -> Result<(), CronError> {
        // This pattern is meant to match every second of 9 am on the first day of January.
        let pattern = CronPattern::new("0 0 9 1 1 *")?;
        let time_matching = Local.with_ymd_and_hms(2023, 1, 1, 9, 0, 0).unwrap();
        let time_not_matching = Local.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();

        assert!(CronScheduler::is_time_matching(&pattern, &time_matching)?);
        assert!(!CronScheduler::is_time_matching(
            &pattern,
            &time_not_matching
        )?);

        Ok(())
    }

    #[test]
    fn test_last_day_of_month() -> Result<(), CronError> {
        // Check the last day of February for a non-leap year
        assert_eq!(CronScheduler::last_day_of_month(2021, 2)?, 28);

        // Check the last day of February for a leap year
        assert_eq!(CronScheduler::last_day_of_month(2020, 2)?, 29);

        // Check for an invalid month (0 or greater than 12)
        assert!(CronScheduler::last_day_of_month(2023, 0).is_err());
        assert!(CronScheduler::last_day_of_month(2023, 13).is_err());

        Ok(())
    }

    #[test]
    fn test_last_day_of_february_non_leap_year() -> Result<(), CronError> {
        // This pattern is meant to match every second of 9 am on the last day of February in a non-leap year.
        let pattern = CronPattern::new("0 0 9 L 2 *")?;
        // February 28th, 2023 is the last day of February in a non-leap year.
        let time_matching = Local.with_ymd_and_hms(2023, 2, 28, 9, 0, 0).unwrap();
        let time_not_matching = Local.with_ymd_and_hms(2023, 2, 28, 10, 0, 0).unwrap();
        let time_not_matching_2 = Local.with_ymd_and_hms(2023, 2, 27, 9, 0, 0).unwrap();

        assert!(CronScheduler::is_time_matching(&pattern, &time_matching)?);
        assert!(!CronScheduler::is_time_matching(
            &pattern,
            &time_not_matching
        )?);
        assert!(!CronScheduler::is_time_matching(
            &pattern,
            &time_not_matching_2
        )?);

        Ok(())
    }

    #[test]
    fn test_last_day_of_february_leap_year() -> Result<(), CronError> {
        // This pattern is meant to match every second of 9 am on the last day of February in a leap year.
        let pattern = CronPattern::new("0 0 9 L 2 *")?;
        // February 29th, 2024 is the last day of February in a leap year.
        let time_matching = Local.with_ymd_and_hms(2024, 2, 29, 9, 0, 0).unwrap();
        let time_not_matching = Local.with_ymd_and_hms(2024, 2, 29, 10, 0, 0).unwrap();
        let time_not_matching_2 = Local.with_ymd_and_hms(2024, 2, 28, 9, 0, 0).unwrap();

        assert!(CronScheduler::is_time_matching(&pattern, &time_matching)?);
        assert!(!CronScheduler::is_time_matching(
            &pattern,
            &time_not_matching
        )?);
        assert!(!CronScheduler::is_time_matching(
            &pattern,
            &time_not_matching_2
        )?);

        Ok(())
    }
}
