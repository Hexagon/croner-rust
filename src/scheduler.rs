// scheduler.rs

use crate::errors::CronError;
use crate::pattern::CronPattern;
use crate::component::LAST_BIT;
use chrono::{NaiveDate, DateTime, Datelike, Local, Timelike, Duration};

// Scheduler module responsible for matching times against cron patterns
pub struct CronScheduler;

impl CronScheduler {
    // Checks if the provided datetime matches the cron pattern fields.
    pub fn is_time_matching(
        cron_pattern: &CronPattern,
        time: &DateTime<Local>,
    ) -> Result<bool, CronError> {
        let last_day = if cron_pattern.days.is_special_bit_set(LAST_BIT) {
            // Call the method and handle the Result
            match Self::last_day_of_month(time.year(), time.month()) {
                Ok(day) => day,
                Err(e) => return Err(e),
            }
        } else {
            // Placeholder for non-LAST_BIT patterns, assuming you want to return 0 in this case
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
