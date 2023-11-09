pub mod pattern;

mod component;
mod errors;

use errors::CronError;
use pattern::CronPattern;
use std::str::FromStr;

use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike};

pub struct CronIterator<Tz>
where
    Tz: TimeZone,
    Tz::Offset: std::fmt::Display,
{
    cron: Cron,
    current_time: DateTime<Tz>,
}

impl<Tz> CronIterator<Tz>
where
    Tz: TimeZone,
    Tz::Offset: std::fmt::Display,
{
    fn new(cron: Cron, start_time: DateTime<Tz>) -> Self {
        CronIterator {
            cron,
            current_time: start_time,
        }
    }
}

impl<Tz> Iterator for CronIterator<Tz>
where
    Tz: TimeZone,
    Tz::Offset: std::fmt::Display,
{
    type Item = DateTime<Tz>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.cron.find_next_occurrence(&self.current_time, true) {
            Ok(next_time) => {
                // Check if we can add one second without overflow
                if let Some(updated_time) =
                    next_time.clone().checked_add_signed(Duration::seconds(1))
                {
                    self.current_time = updated_time;
                    Some(next_time)
                } else {
                    // If we hit an overflow, stop the iteration
                    None
                }
            }
            Err(_) => None, // Stop the iteration if we cannot find the next occurrence
        }
    }
}

// The Cron struct represents a cron schedule and provides methods to parse cron strings,
// check if a datetime matches the cron pattern, and find the next occurrence.
#[derive(Clone)]
pub struct Cron {
    pub pattern: CronPattern, // Parsed cron pattern
}
impl Cron {
    // Tries to parse a given cron string into a Cron instance.
    pub fn parse(cron_string: &str) -> Result<Cron, CronError> {
        let pattern = CronPattern::new(cron_string)?;
        Ok(Cron { pattern })
    }

    /// Evaluates if a given `DateTime` matches the cron pattern associated with this instance.
    ///
    /// The function checks each cron field (seconds, minutes, hours, day of month, month) against
    /// the provided `DateTime` to determine if it aligns with the cron pattern. Each field is
    /// checked for a match, and all fields must match for the entire pattern to be considered
    /// a match.
    ///
    /// # Parameters
    ///
    /// - `time`: A reference to the `DateTime<Tz>` to be checked against the cron pattern.
    ///
    /// # Returns
    ///
    /// - `Ok(bool)`: `true` if `time` matches the cron pattern, `false` otherwise.
    /// - `Err(CronError)`: An error if there is a problem checking any of the pattern fields
    ///   against the provided `DateTime`.
    ///
    /// # Errors
    ///
    /// This method may return `CronError` if an error occurs during the evaluation of the
    /// cron pattern fields. Errors can occur due to invalid bit operations or invalid dates.
    ///
    /// # Examples
    ///
    /// ```
    /// use croner::Cron;
    /// use chrono::Local;
    ///
    /// // Parse cron expression
    /// let cron: Cron = "0 * * * * *".parse().expect("Couldn't parse cron string");
    ///
    /// // Compare to time now
    /// let time = Local::now();
    /// let matches_all = cron.is_time_matching(&time).unwrap();
    ///
    /// // Output results
    /// println!("Time is: {}", time);
    /// println!(
    ///     "Pattern \"{}\" does {} time {}",
    ///     cron.pattern.to_string(),
    ///     if matches_all { "match" } else { "not match" },
    ///     time
    /// );
    /// ```
    pub fn is_time_matching<Tz: TimeZone>(&self, time: &DateTime<Tz>) -> Result<bool, CronError> {
        Ok(self.pattern.second_match(time.second())?
            && self.pattern.minute_match(time.minute())?
            && self.pattern.hour_match(time.hour())?
            && self
                .pattern
                .day_match(time.year(), time.month(), time.day())?
            && self.pattern.month_match(time.month())?)
    }

    /// Finds the next occurrence of a scheduled date and time that matches the cron pattern,
    /// starting from a given `start_time`. If `inclusive` is `true`, the search includes the
    /// `start_time`; otherwise, it starts from the next second.
    ///
    /// This method performs a search through time, beginning at `start_time`, to find the
    /// next date and time that aligns with the cron pattern defined within the `Cron` instance.
    /// The search respects cron fields (seconds, minutes, hours, day of month, month, day of week)
    /// and iterates through time until a match is found or an error occurs.
    ///
    /// # Parameters
    ///
    /// - `start_time`: A reference to a `DateTime<Tz>` indicating the start time for the search.
    /// - `inclusive`: A `bool` that specifies whether the search should include `start_time` itself.
    ///
    /// # Returns
    ///
    /// - `Ok(DateTime<Tz>)`: The next occurrence that matches the cron pattern.
    /// - `Err(CronError)`: An error if the next occurrence cannot be found within a reasonable
    ///   limit, if any of the date/time manipulations result in an invalid date, or if the
    ///   cron pattern match fails.
    ///
    /// # Errors
    ///
    /// - `CronError::InvalidTime`: If the start time provided is invalid or adjustments to the
    ///   time result in an invalid date/time.
    /// - `CronError::TimeSearchLimitExceeded`: If the search exceeds a reasonable time limit.
    ///   This prevents infinite loops in case of patterns that cannot be matched.
    /// - Other errors as defined by the `CronError` enum may occur if the pattern match fails
    ///   at any stage of the search.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Local;
    /// use croner::Cron;
    ///
    /// // Parse cron expression
    /// let cron: Cron = "0 18 * * * 5".parse().expect("Couldn't parse cron string");
    ///
    /// // Get next match
    /// let time = Local::now();
    /// let next = cron.find_next_occurrence(&time, false).unwrap();
    ///
    /// println!(
    ///     "Pattern \"{}\" will match next time at {}",
    ///     cron.pattern.to_string(),
    ///     next
    /// );
    /// ```
    pub fn find_next_occurrence<Tz: TimeZone>(
        &self,
        start_time: &DateTime<Tz>,
        inclusive: bool,
    ) -> Result<DateTime<Tz>, CronError> {
        let mut current_time = if inclusive {
            start_time.clone()
        } else {
            self.increment_time(start_time, Duration::seconds(1))?
        };
        loop {
            current_time = self.find_next_matching_month(&current_time)?;
            current_time = self.find_next_matching_day(&current_time)?;
            current_time = self.find_next_matching_hour(&current_time)?;
            current_time = self.find_next_matching_minute(&current_time)?;
            current_time = self.find_next_matching_second(&current_time)?;
            if self.is_time_matching(&current_time)? {
                return Ok(current_time);
            }
        }
    }

    /// Creates a `CronIterator` starting from the specified time.
    ///
    /// This function will create an iterator that yields dates and times that
    /// match a cron schedule, beginning at `start_from`. The iterator will
    /// begin at the specified start time if it matches.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Local;
    /// use croner::Cron;
    ///
    /// // Parse cron expression
    /// let cron: Cron = "* * * * * *".parse().expect("Couldn't parse cron string");
    ///
    /// // Compare to time now
    /// let time = Local::now();
    ///
    /// // Get next 5 matches using iter_from
    /// println!("Finding matches of pattern '{}' starting from {}:", cron.pattern.to_string(), time);
    ///
    /// for time in cron.clone().iter_from(time).take(5) {
    ///     println!("{}", time);
    /// }
    /// ```
    ///
    /// # Parameters
    ///
    /// - `start_from`: A `DateTime<Tz>` that represents the starting point for the iterator.
    ///
    /// # Returns
    ///
    /// Returns a `CronIterator<Tz>` that can be used to iterate over scheduled times.
    pub fn iter_from<Tz>(&self, start_from: DateTime<Tz>) -> CronIterator<Tz>
    where
        Tz: TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        CronIterator::new(self.clone(), start_from)
    }

    /// Creates a `CronIterator` starting after the specified time.
    ///
    /// This function will create an iterator that yields dates and times that
    /// match a cron schedule, beginning after `start_after`. The iterator will
    /// not yield the specified start time; it will yield times that come
    /// after it according to the cron schedule.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Local;
    /// use croner::Cron;
    ///
    /// // Parse cron expression
    /// let cron: Cron = "* * * * * *".parse().expect("Couldn't parse cron string");
    ///
    /// // Compare to time now
    /// let time = Local::now();
    ///
    /// // Get next 5 matches using iter_from
    /// println!("Finding matches of pattern '{}' starting from {}:", cron.pattern.to_string(), time);
    ///
    /// for time in cron.clone().iter_after(time).take(5) {
    ///     println!("{}", time);
    /// }
    ///  
    /// ```
    ///
    /// # Parameters
    ///
    /// - `start_after`: A `DateTime<Tz>` that represents the starting point for the iterator.
    ///
    /// # Returns
    ///
    /// Returns a `CronIterator<Tz>` that can be used to iterate over scheduled times.
    pub fn iter_after<Tz>(&self, start_after: DateTime<Tz>) -> CronIterator<Tz>
    where
        Tz: TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        let start_from = start_after
            .checked_add_signed(Duration::seconds(1))
            .expect("Invalid date encountered when adding one second");
        CronIterator::new(self.clone(), start_from)
    }

    // Internal function to increment time with a set duration, and give CronError::InvalidTime on failure
    fn increment_time<Tz: TimeZone>(
        &self,
        time: &DateTime<Tz>,
        duration: Duration,
    ) -> Result<DateTime<Tz>, CronError> {
        time.clone()
            .checked_add_signed(duration)
            .ok_or(CronError::InvalidTime)
    }

    // Internal functions to check for the next matching month/day/hour/minute/second and return the updated time.
    fn find_next_matching_month<Tz: TimeZone>(
        &self,
        current_time: &DateTime<Tz>,
    ) -> Result<DateTime<Tz>, CronError> {
        let tz = current_time.timezone();
        let mut year = current_time.year();
        let mut month = current_time.month();
        let mut day = current_time.day();
        let mut hour = current_time.hour();
        let mut minute = current_time.minute();
        let mut second = current_time.second();
        while !self.pattern.month_match(month)? {
            month += 1;
            day = 1;
            hour = 0;
            minute = 0;
            second = 0;
            if month > 12 {
                month = 1;
                year += 1;
                if year > 9999 {
                    return Err(CronError::TimeSearchLimitExceeded);
                }
            }
        }
        tz.with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or(CronError::InvalidDate)
    }
    fn find_next_matching_day<Tz: TimeZone>(
        &self,
        current_time: &DateTime<Tz>,
    ) -> Result<DateTime<Tz>, CronError> {
        let tz = current_time.timezone();
        let mut date = current_time.clone();

        while !self
            .pattern
            .day_match(date.year(), date.month(), date.day())?
        {
            date = self.increment_time(&date, Duration::days(1))?;
            // When the minute changes, reset time
            date = tz
                .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
                .single()
                .ok_or(CronError::InvalidTime)?;
        }

        Ok(date)
    }
    fn find_next_matching_hour<Tz: TimeZone>(
        &self,
        current_time: &DateTime<Tz>,
    ) -> Result<DateTime<Tz>, CronError> {
        let tz = current_time.timezone();
        let mut time = current_time.clone();

        while !self.pattern.hour_match(time.hour())? {
            time = self.increment_time(&time, Duration::hours(1))?;
            // When the hour changes, reset minutes and seconds to 0
            time = tz
                .with_ymd_and_hms(time.year(), time.month(), time.day(), time.hour(), 0, 0)
                .single()
                .ok_or(CronError::InvalidTime)?;
        }

        Ok(time)
    }
    fn find_next_matching_minute<Tz: TimeZone>(
        &self,
        current_time: &DateTime<Tz>,
    ) -> Result<DateTime<Tz>, CronError> {
        let tz = current_time.timezone();
        let mut time = current_time.clone();

        while !self.pattern.minute_match(time.minute())? {
            time = self.increment_time(&time, Duration::minutes(1))?;
            // When the minute changes, reset seconds to 0
            time = tz
                .with_ymd_and_hms(
                    time.year(),
                    time.month(),
                    time.day(),
                    time.hour(),
                    time.minute(),
                    0,
                )
                .single()
                .ok_or(CronError::InvalidTime)?;
        }

        Ok(time)
    }
    fn find_next_matching_second<Tz: TimeZone>(
        &self,
        current_time: &DateTime<Tz>,
    ) -> Result<DateTime<Tz>, CronError> {
        let mut time = current_time.clone();

        while !self.pattern.second_match(time.second())? {
            time = self.increment_time(&time, Duration::seconds(1))?;
        }

        Ok(time)
    }
}

// Enables creating a Cron instance from a string slice, returning a CronError if parsing fails.
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

    #[test]
    fn test_find_next_occurrence() -> Result<(), CronError> {
        // This pattern is meant to match every minute at 30 seconds past the minute.
        let cron = Cron::parse("* * * * * *")?;

        // Set the start time to a known value.
        let start_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 0, 29).unwrap();
        // Calculate the next occurrence from the start time.
        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;

        // Verify that the next occurrence is at the expected time.
        let expected_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 0, 30).unwrap();
        println!("{} {} {}", start_time, next_occurrence, expected_time);
        assert_eq!(next_occurrence, expected_time);

        Ok(())
    }

    #[test]
    fn test_find_next_minute() -> Result<(), CronError> {
        // This pattern is meant to match every minute at 30 seconds past the minute.
        let cron = Cron::parse("0 * * * * *")?;

        // Set the start time to a known value.
        let start_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 0, 29).unwrap();
        // Calculate the next occurrence from the start time.
        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;

        // Verify that the next occurrence is at the expected time.
        let expected_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 1, 0).unwrap();
        println!("{} {} {}", start_time, next_occurrence, expected_time);
        assert_eq!(next_occurrence, expected_time);

        Ok(())
    }

    #[test]
    fn test_wrap_month_and_year() -> Result<(), CronError> {
        // This pattern is meant to match every minute at 30 seconds past the minute.
        let cron = Cron::parse("0 0 15 * * *")?;

        // Set the start time to a known value.
        let start_time = Local.with_ymd_and_hms(2023, 12, 31, 16, 0, 0).unwrap();
        // Calculate the next occurrence from the start time.
        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;

        // Verify that the next occurrence is at the expected time.
        let expected_time = Local.with_ymd_and_hms(2024, 1, 1, 15, 0, 0).unwrap();
        println!("{} {} {}", start_time, next_occurrence, expected_time);
        assert_eq!(next_occurrence, expected_time);

        Ok(())
    }
}
