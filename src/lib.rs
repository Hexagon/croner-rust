pub mod pattern;
mod errors;
mod component;

use component::{ALL_BIT};
use errors::CronError;
use pattern::CronPattern;
use std::str::FromStr;

use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike};

// Scheduler module responsible for matching times against cron patterns
pub struct Cron {
    pub pattern: CronPattern, // Parsed cron pattern
}

impl Cron {
    // Constructor-like function to create a new Cron with a pattern
    pub fn parse(cron_string: &str) -> Result<Cron, CronError> {
        let pattern = CronPattern::new(cron_string)?;
        Ok(Cron { pattern })
    }

    // Checks if the provided datetime matches the cron pattern fields.
    pub fn is_time_matching<Tz: TimeZone>(&self, time: &DateTime<Tz>) -> Result<bool, CronError> {
        let second_matches = self.pattern.seconds.is_bit_set(time.second() as u8, ALL_BIT)?;
        let minute_matches = self.pattern.minutes.is_bit_set(time.minute() as u8, ALL_BIT)?;
        let hour_matches = self.pattern.hours.is_bit_set(time.hour() as u8, ALL_BIT)?;
        let month_matches = self.pattern.months.is_bit_set(time.month() as u8, ALL_BIT)?;
        let day_of_month_matches = self.pattern.day_match(time.year(), time.month(), time.day())?;
        Ok(second_matches && minute_matches && hour_matches && day_of_month_matches && month_matches)
    }

    pub fn find_next_occurrence<Tz: TimeZone>(
        &self,
        start_time: &DateTime<Tz>,
    ) -> Result<DateTime<Tz>, CronError> {
        let mut current_time = start_time
            .clone()
            .checked_add_signed(Duration::seconds(1))
            .ok_or(CronError::InvalidDate)?; // Start at the next second
        let tz = start_time.timezone(); // Capture the timezone

        'outer: loop {
            // Check if the current month matches the pattern
            if !self.pattern.month_match(current_time.month())? {
                let mut month = current_time.month();
                let mut year = current_time.year();

                loop {
                    month += 1;
                    if month > 12 {
                        month = 1;
                        year += 1;
                    }
                    if self.pattern.month_match(month)? {
                        break;
                    }
                    // Arbitrary limit to prevent infinite loops
                    // - If it is year 9998 and you have a problem with this - I'm sorry!
                    if year > 10000 {
                        return Err(CronError::TimeSearchLimitExceeded);
                    }
                }

                // If the month changes, we set the day to 1 and hours, minutes, seconds to 0
                current_time = tz.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
                continue 'outer;
            }

            // Check if the current day matches the pattern
            if !self.pattern.day_match(
                current_time.year(),
                current_time.month(),
                current_time.day(),
            )? {
                loop {
                    current_time = current_time
                        .checked_add_signed(Duration::days(1))
                        .ok_or(CronError::InvalidDate)?;

                    // Reset hours, minutes, and seconds to start of the day
                    current_time = tz.with_ymd_and_hms(
                        current_time.year(),
                        current_time.month(),
                        current_time.day(),
                        0,
                        0,
                        0
                    ).unwrap();

                    // If the day changes to the first of the next month, start 'outer loop again
                    if current_time.day() == 1 || self.pattern.day_match(
                        current_time.year(),
                        current_time.month(),
                        current_time.day(),
                    )? {
                        continue 'outer;
                    }
                }
            }


            // Check if the current hour matches the pattern
            if !self.pattern.hour_match(current_time.hour())? {
                current_time = current_time
                    .checked_add_signed(Duration::hours(1))
                    .ok_or(CronError::InvalidDate)?
                    .with_minute(0)
                    .ok_or(CronError::InvalidDate)?
                    .with_second(0)
                    .ok_or(CronError::InvalidDate)?;
                continue;
            }

            // Check if the current minute matches the pattern
            if !self.pattern.minute_match(current_time.minute())? {
                current_time = current_time
                    .checked_add_signed(Duration::minutes(1))
                    .ok_or(CronError::InvalidDate)?
                    .with_second(0)
                    .ok_or(CronError::InvalidDate)?; // Start at the next second
                continue;
            }

            // Check if the current second matches the pattern
            if self.pattern.second_match(current_time.second())? {
                // If we have a match, then return the current_time
                return Ok(current_time);
            }

            // Otherwise, add a second and check again
            current_time = current_time
                .checked_add_signed(Duration::seconds(1))
                .ok_or(CronError::InvalidDate)?;
        }
    }
}

impl FromStr for Cron {
    type Err = CronError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Cron::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};
    #[test]
    fn test_is_time_matching() -> Result<(), CronError> {
        // This pattern is meant to match first second of 9 am on the first day of January.
        let cron = Cron::parse("0 0 9 1 1 *")?;
        let time_matching = Local.with_ymd_and_hms(2023, 1, 1, 9, 0, 0).unwrap();
        let time_not_matching = Local.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);
        assert!(!cron.is_time_matching(&time_not_matching)?);

        Ok(())
    }

    #[test]
    fn test_last_day_of_february_non_leap_year() -> Result<(), CronError> {
 
        // This pattern is meant to match every second of 9 am on the last day of February in a non-leap year.
        let cron = Cron::parse("0 0 9 L 2 *")?;

        // February 28th, 2023 is the last day of February in a non-leap year.
        let time_matching = Local.with_ymd_and_hms(2023, 2, 28, 9, 0, 0).unwrap();
        let time_not_matching = Local.with_ymd_and_hms(2023, 2, 28, 10, 0, 0).unwrap();
        let time_not_matching_2 = Local.with_ymd_and_hms(2023, 2, 27, 9, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);
        assert!(!cron.is_time_matching(&time_not_matching)?);
        assert!(!cron.is_time_matching(&time_not_matching_2)?);

        Ok(())
    }

    #[test]
    fn test_last_day_of_february_leap_year() -> Result<(), CronError> {
        // This pattern is meant to match every second of 9 am on the last day of February in a leap year.
        let cron = Cron::parse("0 0 9 L 2 *")?;
        // February 29th, 2024 is the last day of February in a leap year.
        let time_matching = Local.with_ymd_and_hms(2024, 2, 29, 9, 0, 0).unwrap();
        let time_not_matching = Local.with_ymd_and_hms(2024, 2, 29, 10, 0, 0).unwrap();
        let time_not_matching_2 = Local.with_ymd_and_hms(2024, 2, 28, 9, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);
        assert!(!cron.is_time_matching(&time_not_matching)?);
        assert!(!cron.is_time_matching(&time_not_matching_2)?);

        Ok(())
    }

    #[test]
    fn test_last_friday_of_year() -> Result<(), CronError> {

        // This pattern is meant to match 0:00:00 last friday of current year
        let cron = Cron::parse("0 0 0 * * FRI#L")?;

        // February 29th, 2024 is the last day of February in a leap year.
        let time_matching = Local.with_ymd_and_hms(2023, 12, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);

        Ok(())
    }
}