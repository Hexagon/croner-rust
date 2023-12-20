//! # Croner
//!
//! Croner is a fully-featured, lightweight, and efficient Rust library designed for parsing and evaluating cron patterns.
//!
//! ## Features
//! - Parses a wide range of cron expressions, including extended formats.
//! - Evaluates cron patterns to calculate upcoming execution times.
//! - Supports time zone-aware scheduling.
//! - Offers granularity up to seconds for precise task scheduling.
//! - Compatible with the `chrono` library for dealing with date and time in Rust.
//!
//! ## Example
//! The following example demonstrates how to use Croner to parse a cron expression and find the next occurrence of a specified time:
//!
//! ```rust
//! use chrono::Utc;
//! use croner::Cron;
//!
//! // Parse a cron expression to find the next occurrence at 00:00 on Friday
//! let cron = Cron::new("0 0 * * FRI").parse().expect("Successful parsing");
//!
//! // Get the next occurrence from the current time, excluding the current time
//! let next = cron.find_next_occurrence(&Utc::now(), false).unwrap();
//!
//! println!(
//!     "Pattern \"{}\" will match next at {}",
//!     cron.pattern.to_string(),
//!     next
//! );
//! ```
//!
//! In this example, `Cron::new("0 0 * * FRI")` creates a new Cron instance for the pattern that represents every Friday at midnight. The `find_next_occurrence` method calculates the next time this pattern will be true from the current moment.
//!
//! The `false` argument in `find_next_occurrence` specifies that the current time is not included in the calculation, ensuring that only future occurrences are considered.
//!
//! ## Getting Started
//! To start using Croner, add it to your project's `Cargo.toml` and follow the examples to integrate cron pattern parsing and scheduling into your application.
//!
//! ## Pattern
//!
//! The expressions used by Croner are very similar to those of Vixie Cron, but with
//! a few additions as outlined below:
//!
//! ```javascript
//! // ┌──────────────── (optional) second (0 - 59)
//! // │ ┌────────────── minute (0 - 59)
//! // │ │ ┌──────────── hour (0 - 23)
//! // │ │ │ ┌────────── day of month (1 - 31)
//! // │ │ │ │ ┌──────── month (1 - 12, JAN-DEC)
//! // │ │ │ │ │ ┌────── day of week (0 - 6, SUN-Mon)
//! // │ │ │ │ │ │       (0 to 6 are Sunday to Saturday; 7 is Sunday, the same as 0)
//! // │ │ │ │ │ │
//! // * * * * * *
//! ```
//!
//! | Field        | Required | Allowed values  | Allowed special characters | Remarks                                                                                                         |
//! | ------------ | -------- | --------------- | -------------------------- | --------------------------------------------------------------------------------------------------------------- |
//! | Seconds      | Optional | 0-59            | * , - / ?                  |                                                                                                                 |
//! | Minutes      | Yes      | 0-59            | * , - / ?                  |                                                                                                                 |
//! | Hours        | Yes      | 0-23            | * , - / ?                  |                                                                                                                 |
//! | Day of Month | Yes      | 1-31            | * , - / ? L W              |                                                                                                                 |
//! | Month        | Yes      | 1-12 or JAN-DEC | * , - / ?                  |                                                                                                                 |
//! | Day of Week  | Yes      | 0-7 or SUN-MON  | * , - / ? # L              | 0 to 6 are Sunday to Saturday, 7 is Sunday, the same as 0. '#' is used to specify the nth occurrence of a weekday |
//!
//! For more information, refer to the full [README](https://github.com/hexagon/croner-rust).

mod component;
mod errors;
mod iterator;
mod pattern;

use errors::CronError;
use iterator::CronIterator;
use pattern::CronPattern;
use std::str::FromStr;

use chrono::{
    DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike,
};

const YEAR_UPPER_LIMIT: i32 = 5000;

enum TimeComponent {
    Second = 1,
    Minute,
    Hour,
    Day,
    Month,
    Year,
}

// The Cron struct represents a cron schedule and provides methods to parse cron strings,
// check if a datetime matches the cron pattern, and find the next occurrence.
#[derive(Clone)]
pub struct Cron {
    pub pattern: CronPattern, // Parsed cron pattern
}
impl Cron {
    // Constructor to create a new instance of Cron with default settings
    pub fn new(cron_string: &str) -> Self {
        Self {
            pattern: CronPattern::new(cron_string),
        }
    }

    // Tries to parse a given cron string into a Cron instance.
    pub fn parse(&mut self) -> Result<Cron, CronError> {
        self.pattern.parse()?;
        Ok(self.clone())
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
    /// use chrono::Utc;
    ///
    /// // Parse cron expression
    /// let cron: Cron = Cron::new("* * * * *").parse().expect("Couldn't parse cron string");
    ///
    /// // Compare to time now
    /// let time = Utc::now();
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
        // Convert to NaiveDateTime
        let naive_time = time.naive_local();

        // Use NaiveDateTime for the comparisons
        Ok(self.pattern.second_match(naive_time.second())?
            && self.pattern.minute_match(naive_time.minute())?
            && self.pattern.hour_match(naive_time.hour())?
            && self
                .pattern
                .day_match(naive_time.year(), naive_time.month(), naive_time.day())?
            && self.pattern.month_match(naive_time.month())?)
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
    /// use chrono::Utc;
    /// use croner::Cron;
    ///
    /// // Parse cron expression
    /// let cron: Cron = Cron::new("0 18 * * * 5").with_seconds_required().parse().expect("Success");
    ///
    /// // Get next match
    /// let time = Utc::now();
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
    ) -> Result<DateTime<Tz>, CronError>
    where
        Tz: TimeZone,
    {
        let mut naive_time = start_time.naive_local();
        let originaltimezone = start_time.timezone();

        if !inclusive {
            naive_time = naive_time
                .checked_add_signed(chrono::Duration::seconds(1))
                .ok_or(CronError::InvalidTime)?;
        }

        loop {
            let mut updated = false;

            updated |= self.find_next_matching_month(&mut naive_time)?;
            updated |= self.find_next_matching_day(&mut naive_time)?;
            updated |= self.find_next_matching_hour(&mut naive_time)?;
            updated |= self.find_next_matching_minute(&mut naive_time)?;
            updated |= self.find_next_matching_second(&mut naive_time)?;

            if updated {
                continue;
            }

            // Convert back to original timezone
            let tz_datetime_result = from_naive(naive_time, &originaltimezone)?;

            // Check for match
            if self.is_time_matching(&tz_datetime_result)? {
                return Ok(tz_datetime_result);
            } else {
                return Err(CronError::TimeSearchLimitExceeded);
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
    /// use chrono::Utc;
    /// use croner::Cron;
    ///
    /// // Parse cron expression
    /// let cron = Cron::new("* * * * *").parse().expect("Couldn't parse cron string");
    ///
    /// // Compare to time now
    /// let time = Utc::now();
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
    /// use chrono::Utc;
    /// use croner::Cron;
    ///
    /// // Parse cron expression
    /// let cron = Cron::new("* * * * *").parse().expect("Couldn't parse cron string");
    ///
    /// // Compare to time now
    /// let time = Utc::now();
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
    pub fn iter_after<Tz: TimeZone>(&self, start_after: DateTime<Tz>) -> CronIterator<Tz>
    where
        Tz: TimeZone,
    {
        let start_from = start_after
            .checked_add_signed(Duration::seconds(1))
            .expect("Invalid date encountered when adding one second");
        CronIterator::new(self.clone(), start_from)
    }

    // Internal functions to check for the next matching month/day/hour/minute/second and return the updated time.
    fn find_next_matching_month(
        &self,
        current_time: &mut NaiveDateTime,
    ) -> Result<bool, CronError> {
        let mut incremented = false;
        while !self.pattern.month_match(current_time.month())? {
            increment_time_component(current_time, TimeComponent::Month)?;
            incremented = true;
        }
        Ok(incremented)
    }

    fn find_next_matching_day(&self, current_time: &mut NaiveDateTime) -> Result<bool, CronError> {
        let mut incremented = false;
        while !self.pattern.day_match(
            current_time.year(),
            current_time.month(),
            current_time.day(),
        )? {
            increment_time_component(current_time, TimeComponent::Day)?;
            incremented = true;
        }

        Ok(incremented)
    }

    fn find_next_matching_hour(&self, current_time: &mut NaiveDateTime) -> Result<bool, CronError> {
        let mut incremented = false;
        let next_hour_result = self.pattern.next_hour_match(current_time.hour());

        match next_hour_result {
            Ok(Some(next_match)) if next_match != current_time.hour() => {
                set_time_component(current_time, TimeComponent::Hour, next_match)?;
            }
            Ok(None) => {
                increment_time_component(current_time, TimeComponent::Day)?;
                incremented = true;
            }
            Err(e) => return Err(e), // Propagate any CronError
            _ => {}                  // No action needed if the current hour already matches
        }
        Ok(incremented)
    }

    fn find_next_matching_minute(
        &self,
        current_time: &mut NaiveDateTime,
    ) -> Result<bool, CronError> {
        let mut incremented = false;
        let next_minute_result = self.pattern.next_minute_match(current_time.minute());

        match next_minute_result {
            Ok(Some(next_match)) if next_match != current_time.minute() => {
                incremented = true;
                set_time_component(current_time, TimeComponent::Minute, next_match)?;
            }
            Ok(None) => {
                incremented = true;
                increment_time_component(current_time, TimeComponent::Hour)?;
            }
            Err(e) => return Err(e), // Propagate the CronError
            _ => {}                  // No action needed if the current minute matches
        }
        Ok(incremented)
    }

    fn find_next_matching_second(
        &self,
        current_time: &mut NaiveDateTime,
    ) -> Result<bool, CronError> {
        let mut incremented = false;
        let next_second_result = self.pattern.next_second_match(current_time.second());

        match next_second_result {
            Ok(Some(next_match)) => {
                // If a matching second is found, set it and mark as incremented.
                set_time_component(current_time, TimeComponent::Second, next_match)?;
            }
            Ok(None) => {
                // If no match is found in the current minute, increment the minute.
                increment_time_component(current_time, TimeComponent::Minute)?;
                incremented = true;
            }
            Err(e) => {
                // Propagate any errors encountered during the match process.
                return Err(e);
            }
        }
        Ok(incremented)
    }

    pub fn with_dom_and_dow(&mut self) -> &mut Self {
        self.pattern.with_dom_and_dow();
        self
    }

    pub fn with_seconds_optional(&mut self) -> &mut Self {
        self.pattern.with_seconds_optional();
        self
    }

    pub fn with_seconds_required(&mut self) -> &mut Self {
        self.pattern.with_seconds_required();
        self
    }

    pub fn with_alternative_weekdays(&mut self) -> &mut Self {
        self.pattern.with_alternative_weekdays();
        self
    }
}

// Enables creating a Cron instance from a string slice, returning a CronError if parsing fails.
impl FromStr for Cron {
    type Err = CronError;

    fn from_str(cron_string: &str) -> Result<Cron, CronError> {
        let res = Cron::new(cron_string);
        Ok(res)
    }
}

// Recursive function to handle setting the time and managing overflows.
#[allow(clippy::too_many_arguments)]
fn set_time(
    current_time: &mut NaiveDateTime,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    component: TimeComponent,
) -> Result<(), CronError> {
    // First, try creating a NaiveDate and NaiveTime
    match (
        NaiveDate::from_ymd_opt(year, month, day),
        NaiveTime::from_hms_opt(hour, minute, second),
    ) {
        (Some(date), Some(time)) => {
            // Combine date and time into NaiveDateTime
            *current_time = date.and_time(time);
            Ok(())
        }
        _ => {
            // Handle invalid date or overflow by incrementing the next higher component.
            match component {
                TimeComponent::Second => set_time(
                    current_time,
                    year,
                    month,
                    day,
                    hour,
                    minute + 1,
                    0,
                    TimeComponent::Minute,
                ),
                TimeComponent::Minute => set_time(
                    current_time,
                    year,
                    month,
                    day,
                    hour + 1,
                    0,
                    0,
                    TimeComponent::Hour,
                ),
                TimeComponent::Hour => set_time(
                    current_time,
                    year,
                    month,
                    day + 1,
                    0,
                    0,
                    0,
                    TimeComponent::Day,
                ),
                TimeComponent::Day => set_time(
                    current_time,
                    year,
                    month + 1,
                    1,
                    0,
                    0,
                    0,
                    TimeComponent::Month,
                ),
                TimeComponent::Month => {
                    set_time(current_time, year + 1, 1, 1, 0, 0, 0, TimeComponent::Year)
                }
                TimeComponent::Year => Err(CronError::InvalidDate),
            }
        }
    }
}

fn set_time_component(
    current_time: &mut NaiveDateTime,
    component: TimeComponent,
    set_to: u32,
) -> Result<(), CronError> {
    // Extract all parts
    let (year, month, day, hour, minute, _second) = (
        current_time.year(),
        current_time.month(),
        current_time.day(),
        current_time.hour(),
        current_time.minute(),
        current_time.second(),
    );

    match component {
        TimeComponent::Year => set_time(current_time, set_to as i32, 0, 0, 0, 0, 0, component),
        TimeComponent::Month => set_time(current_time, year, set_to, 0, 0, 0, 0, component),
        TimeComponent::Day => set_time(current_time, year, month, set_to, 0, 0, 0, component),
        TimeComponent::Hour => set_time(current_time, year, month, day, set_to, 0, 0, component),
        TimeComponent::Minute => {
            set_time(current_time, year, month, day, hour, set_to, 0, component)
        }
        TimeComponent::Second => set_time(
            current_time,
            year,
            month,
            day,
            hour,
            minute,
            set_to,
            component,
        ),
    }
}

// Convert `NaiveDateTime` back to `DateTime<Tz>`
pub fn from_naive<Tz: TimeZone>(
    naive_time: NaiveDateTime,
    timezone: &Tz,
) -> Result<DateTime<Tz>, CronError> {
    match timezone.from_local_datetime(&naive_time) {
        chrono::LocalResult::Single(dt) => Ok(dt),
        _ => Err(CronError::InvalidTime),
    }
}

fn increment_time_component(
    current_time: &mut NaiveDateTime,
    component: TimeComponent,
) -> Result<(), CronError> {
    // Check for time overflow
    if current_time.year() >= YEAR_UPPER_LIMIT {
        return Err(CronError::TimeSearchLimitExceeded);
    }

    // Extract all parts
    let (year, month, day, hour, minute, second) = (
        current_time.year(),
        current_time.month(),
        current_time.day(),
        current_time.hour(),
        current_time.minute(),
        current_time.second(),
    );

    // Increment the component and try to set the new time.
    match component {
        TimeComponent::Year => set_time(current_time, year + 1, 1, 1, 0, 0, 0, component),
        TimeComponent::Month => set_time(current_time, year, month + 1, 1, 0, 0, 0, component),
        TimeComponent::Day => set_time(current_time, year, month, day + 1, 0, 0, 0, component),
        TimeComponent::Hour => set_time(current_time, year, month, day, hour + 1, 0, 0, component),
        TimeComponent::Minute => set_time(
            current_time,
            year,
            month,
            day,
            hour,
            minute + 1,
            0,
            component,
        ),
        TimeComponent::Second => set_time(
            current_time,
            year,
            month,
            day,
            hour,
            minute,
            second + 1,
            component,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};
    #[test]
    fn test_is_time_matching() -> Result<(), CronError> {
        // This pattern is meant to match first second of 9 am on the first day of January.
        let cron = Cron::new("0 9 1 1 *").parse()?;
        let time_matching = Local.with_ymd_and_hms(2023, 1, 1, 9, 0, 0).unwrap();
        let time_not_matching = Local.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);
        assert!(!cron.is_time_matching(&time_not_matching)?);

        Ok(())
    }

    #[test]
    fn test_last_day_of_february_non_leap_year() -> Result<(), CronError> {
        // This pattern is meant to match every second of 9 am on the last day of February in a non-leap year.
        let cron = Cron::new("0 9 L 2 *").parse()?;

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
        let cron = Cron::new("0 9 L 2 *").parse()?;

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
        let cron = Cron::new("0 0 * * FRI#L").parse()?;

        // February 29th, 2024 is the last day of February in a leap year.
        let time_matching = Local.with_ymd_and_hms(2023, 12, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);

        Ok(())
    }

    #[test]
    fn test_last_friday_of_year_alternative_alpha_syntax() -> Result<(), CronError> {
        // This pattern is meant to match 0:00:00 last friday of current year
        let cron = Cron::new("0 0 * * FRIl").parse()?;

        // February 29th, 2024 is the last day of February in a leap year.
        let time_matching = Local.with_ymd_and_hms(2023, 12, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);

        Ok(())
    }

    #[test]
    fn test_last_friday_of_year_alternative_number_syntax() -> Result<(), CronError> {
        // This pattern is meant to match 0:00:00 last friday of current year
        let cron = Cron::new("0 0 * * 5L").parse()?;

        // February 29th, 2024 is the last day of February in a leap year.
        let time_matching = Local.with_ymd_and_hms(2023, 12, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);

        Ok(())
    }

    #[test]
    fn test_find_next_occurrence() -> Result<(), CronError> {
        // This pattern is meant to match every minute at 30 seconds past the minute.
        let cron = Cron::new("* * * * * *").with_seconds_optional().parse()?;

        // Set the start time to a known value.
        let start_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 0, 29).unwrap();
        // Calculate the next occurrence from the start time.
        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;

        // Verify that the next occurrence is at the expected time.
        let expected_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 0, 30).unwrap();
        assert_eq!(next_occurrence, expected_time);

        Ok(())
    }

    #[test]
    fn test_find_next_minute() -> Result<(), CronError> {
        let cron = Cron::new("* * * * *").parse()?;

        // Set the start time to a known value.
        let start_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 0, 29).unwrap();
        // Calculate the next occurrence from the start time.
        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;

        // Verify that the next occurrence is at the expected time.
        let expected_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 1, 0).unwrap();
        assert_eq!(next_occurrence, expected_time);

        Ok(())
    }

    #[test]
    fn test_wrap_month_and_year() -> Result<(), CronError> {
        // This pattern is meant to match every minute at 30 seconds past the minute.
        let cron = Cron::new("0 0 15 * * *").with_seconds_optional().parse()?;

        // Set the start time to a known value.
        let start_time = Local.with_ymd_and_hms(2023, 12, 31, 16, 0, 0).unwrap();
        // Calculate the next occurrence from the start time.
        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;

        // Verify that the next occurrence is at the expected time.
        let expected_time = Local.with_ymd_and_hms(2024, 1, 1, 15, 0, 0).unwrap();
        assert_eq!(next_occurrence, expected_time);

        Ok(())
    }

    #[test]
    fn test_weekday_pattern_correct_weekdays() -> Result<(), CronError> {
        let schedule = Cron::new("0 0 0 * * 5,6").with_seconds_optional().parse()?;
        let start_time = Local
            .with_ymd_and_hms(2022, 2, 17, 0, 0, 0)
            .single()
            .unwrap();
        let mut next_runs = Vec::new();

        for next in schedule.iter_after(start_time).take(6) {
            next_runs.push(next);
        }

        assert_eq!(next_runs[0].year(), 2022);
        assert_eq!(next_runs[0].month(), 2);
        assert_eq!(next_runs[0].day(), 18);

        assert_eq!(next_runs[1].day(), 19);
        assert_eq!(next_runs[2].day(), 25);
        assert_eq!(next_runs[3].day(), 26);

        assert_eq!(next_runs[4].month(), 3);
        assert_eq!(next_runs[4].day(), 4);
        assert_eq!(next_runs[5].day(), 5);

        Ok(())
    }

    #[test]
    fn test_weekday_pattern_combined_with_day_of_month() -> Result<(), CronError> {
        let schedule = Cron::new("59 59 23 2 * 6")
            .with_seconds_optional()
            .parse()?;
        let start_time = Local
            .with_ymd_and_hms(2022, 1, 31, 0, 0, 0)
            .single()
            .unwrap();
        let mut next_runs = Vec::new();

        for next in schedule.iter_after(start_time).take(6) {
            next_runs.push(next);
        }

        assert_eq!(next_runs[0].year(), 2022);
        assert_eq!(next_runs[0].month(), 2);
        assert_eq!(next_runs[0].day(), 2);

        assert_eq!(next_runs[1].month(), 2);
        assert_eq!(next_runs[1].day(), 5);

        assert_eq!(next_runs[2].month(), 2);
        assert_eq!(next_runs[2].day(), 12);

        assert_eq!(next_runs[3].month(), 2);
        assert_eq!(next_runs[3].day(), 19);

        assert_eq!(next_runs[4].month(), 2);
        assert_eq!(next_runs[4].day(), 26);

        assert_eq!(next_runs[5].month(), 3);
        assert_eq!(next_runs[5].day(), 2);

        Ok(())
    }

    #[test]
    fn test_weekday_pattern_alone() -> Result<(), CronError> {
        let schedule = Cron::new("15 9 * * mon").parse()?;
        let start_time = Local
            .with_ymd_and_hms(2022, 2, 28, 23, 59, 0)
            .single()
            .unwrap();
        let mut next_runs = Vec::new();

        for next in schedule.iter_after(start_time).take(3) {
            next_runs.push(next);
        }

        assert_eq!(next_runs[0].year(), 2022);
        assert_eq!(next_runs[0].month(), 3);
        assert_eq!(next_runs[0].day(), 7);
        assert_eq!(next_runs[0].hour(), 9);
        assert_eq!(next_runs[0].minute(), 15);

        assert_eq!(next_runs[1].day(), 14);
        assert_eq!(next_runs[1].hour(), 9);
        assert_eq!(next_runs[1].minute(), 15);

        assert_eq!(next_runs[2].day(), 21);
        assert_eq!(next_runs[2].hour(), 9);
        assert_eq!(next_runs[2].minute(), 15);

        Ok(())
    }

    #[test]
    fn test_cron_expression_13w_wed() -> Result<(), CronError> {
        // Parse the cron expression
        let cron = Cron::new("0 0 13W * WED").parse()?;

        // Define the start date for the test
        let start_date = Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        // Define the expected matching dates
        let expected_dates = vec![
            Local.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 12, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 17, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 24, 0, 0, 0).unwrap(),
        ];

        // Iterate over the expected dates, checking each one
        let mut idx = 0;
        for current_date in cron.clone().iter_from(start_date).take(5) {
            assert_eq!(expected_dates[idx], current_date);
            idx = idx + 1;
        }

        Ok(())
    }

    #[test]
    fn test_cron_expression_31dec_fri() -> Result<(), CronError> {
        // Parse the cron expression
        let cron = Cron::new("0 0 0 31 12 FRI")
            .with_seconds_required()
            .with_dom_and_dow()
            .parse()?;

        // Define the start date for the test
        let start_date = Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        // Define the expected matching dates
        let expected_dates = vec![
            Local.with_ymd_and_hms(2027, 12, 31, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2032, 12, 31, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2038, 12, 31, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2049, 12, 31, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2055, 12, 31, 0, 0, 0).unwrap(),
        ];

        // Iterate over the expected dates, checking each one
        let mut idx = 0;
        for current_date in cron.clone().iter_from(start_date).take(5) {
            assert_eq!(expected_dates[idx], current_date);
            idx = idx + 1;
        }

        Ok(())
    }

    #[test]
    fn test_cron_parse_invalid_expressions() {
        let invalid_expressions = vec![
            "* * *",
            "invalid",
            "123",
            "0 0 * * * * *",
            "* * * *",
            "* 60 * * * *",
            "-1 59 * * * *",
            "1- 59 * * * *",
            "0 0 0 5L * *",
            "0 0 0 5#L * *",
        ];
        for expr in invalid_expressions {
            assert!(Cron::new(expr).with_seconds_optional().parse().is_err());
        }
    }

    #[test]
    fn test_cron_parse_valid_expressions() {
        let valid_expressions = vec![
            "* * * * *",
            "0 0 * * *",
            "*/10 * * * *",
            "0 0 1 1 *",
            "0 12 * * MON",
            "0 0   * * 1",
            "0 0 1 1,7 * ",
            "00 00 01 * SUN  ",
            "0 0 1-7 * SUN",
            "5-10/2 * * * *",
            "0 0-23/2 * * *",
            "0 12 15-21 * 1-FRI",
            "0 0 29 2 *",
            "0 0 31 * *",
            "*/15 9-17 * * MON-FRI",
            "0 12 * JAN-JUN *",
            "0 0 1,15,L * SUN#L",
            "0 0 2,1 1-6/2 *",
            "0 0 5,L * 5L",
        ];
        for expr in valid_expressions {
            assert!(Cron::new(expr).parse().is_ok());
        }
    }

    #[test]
    fn test_is_time_matching_different_time_zones() -> Result<(), CronError> {
        use chrono::FixedOffset;

        let cron = Cron::new("0 12 * * *").parse()?;
        let time_east_matching = FixedOffset::east_opt(3600)
            .expect("Success")
            .with_ymd_and_hms(2023, 1, 1, 12, 0, 0)
            .unwrap(); // UTC+1
        let time_west_matching = FixedOffset::west_opt(3600)
            .expect("Success")
            .with_ymd_and_hms(2023, 1, 1, 12, 0, 0)
            .unwrap(); // UTC-1

        assert!(cron.is_time_matching(&time_east_matching)?);
        assert!(cron.is_time_matching(&time_west_matching)?);

        Ok(())
    }

    #[test]
    fn test_find_next_occurrence_edge_case_inclusive() -> Result<(), CronError> {
        let cron = Cron::new("59 59 23 * * *")
            .with_seconds_required()
            .parse()?;
        let start_time = Local.with_ymd_and_hms(2023, 3, 14, 23, 59, 59).unwrap();
        let next_occurrence = cron.find_next_occurrence(&start_time, true)?;
        let expected_time = Local.with_ymd_and_hms(2023, 3, 14, 23, 59, 59).unwrap();
        assert_eq!(next_occurrence, expected_time);
        Ok(())
    }

    #[test]
    fn test_find_next_occurrence_edge_case_exclusive() -> Result<(), CronError> {
        let cron = Cron::new("59 59 23 * * *")
            .with_seconds_optional()
            .parse()?;
        let start_time = Local.with_ymd_and_hms(2023, 3, 14, 23, 59, 59).unwrap();
        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;
        let expected_time = Local.with_ymd_and_hms(2023, 3, 15, 23, 59, 59).unwrap();
        assert_eq!(next_occurrence, expected_time);
        Ok(())
    }

    #[test]
    fn test_cron_iterator_large_time_jumps() -> Result<(), CronError> {
        let cron = Cron::new("0 0 * * *").parse()?;
        let start_time = Local.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let mut iterator = cron.iter_after(start_time);
        let next_run = iterator.nth(365 * 5 + 1); // Jump 5 years ahead
        let expected_time = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(next_run, Some(expected_time));
        Ok(())
    }

    #[test]
    fn test_handling_different_month_lengths() -> Result<(), CronError> {
        let cron = Cron::new("0 0 L * *").parse()?; // Last day of the month
        let feb_non_leap_year = Local.with_ymd_and_hms(2023, 2, 1, 0, 0, 0).unwrap();
        let feb_leap_year = Local.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();
        let april = Local.with_ymd_and_hms(2023, 4, 1, 0, 0, 0).unwrap();

        assert_eq!(
            cron.find_next_occurrence(&feb_non_leap_year, false)?,
            Local.with_ymd_and_hms(2023, 2, 28, 0, 0, 0).unwrap()
        );
        assert_eq!(
            cron.find_next_occurrence(&feb_leap_year, false)?,
            Local.with_ymd_and_hms(2024, 2, 29, 0, 0, 0).unwrap()
        );
        assert_eq!(
            cron.find_next_occurrence(&april, false)?,
            Local.with_ymd_and_hms(2023, 4, 30, 0, 0, 0).unwrap()
        );

        Ok(())
    }

    #[test]
    fn test_cron_iterator_non_standard_intervals() -> Result<(), CronError> {
        let cron = Cron::new("*/29 */13 * * * *")
            .with_seconds_optional()
            .parse()?;
        let start_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let mut iterator = cron.iter_after(start_time);
        let first_run = iterator.next().unwrap();
        let second_run = iterator.next().unwrap();

        assert_eq!(first_run.hour() % 13, 0);
        assert_eq!(first_run.minute() % 29, 0);
        assert_eq!(second_run.hour() % 13, 0);
        assert_eq!(second_run.minute() % 29, 0);

        Ok(())
    }

    #[test]
    fn test_cron_iterator_non_standard_intervals_with_offset() -> Result<(), CronError> {
        let cron = Cron::new("7/29 2/13 * * *").parse()?;
        let start_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let mut iterator = cron.iter_after(start_time);

        let first_run = iterator.next().unwrap();
        // Expect the first run to be at 02:07 (2 hours and 7 minutes after midnight)
        assert_eq!(first_run.hour(), 2);
        assert_eq!(first_run.minute(), 7);

        let second_run = iterator.next().unwrap();
        // Expect the second run to be at 02:36 (29 minutes after the first run)
        assert_eq!(second_run.hour(), 2);
        assert_eq!(second_run.minute(), 36);

        Ok(())
    }

    #[test]
    fn test_cron_expression_29feb_march_fri() -> Result<(), CronError> {
        use chrono::TimeZone;

        // Parse the cron expression with specified options
        let cron = Cron::new("0 0 29 2-3 FRI")
            .with_dom_and_dow()
            .with_seconds_optional()
            .parse()?;

        // Define the start date for the test
        let start_date = Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        // Define the expected matching dates
        let expected_dates = vec![
            Local.with_ymd_and_hms(2024, 3, 29, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2030, 3, 29, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2036, 2, 29, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2041, 3, 29, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2047, 3, 29, 0, 0, 0).unwrap(),
        ];

        // Iterate over the expected dates, checking each one
        let mut idx = 0;
        for current_date in cron.iter_from(start_date).take(5) {
            assert_eq!(expected_dates[idx], current_date);
            idx += 1;
        }

        Ok(())
    }

    #[test]
    fn test_specific_and_wildcard_entries() -> Result<(), CronError> {
        let cron = Cron::new("15 */2 * 3,5 FRI").parse()?;
        let matching_time = Local.with_ymd_and_hms(2023, 3, 3, 2, 15, 0).unwrap();
        let non_matching_time = Local.with_ymd_and_hms(2023, 3, 3, 3, 15, 0).unwrap();

        assert!(cron.is_time_matching(&matching_time)?);
        assert!(!cron.is_time_matching(&non_matching_time)?);

        Ok(())
    }

    #[test]
    fn test_month_weekday_edge_cases() -> Result<(), CronError> {
        let cron = Cron::new("0 0 * 2-3 SUN").parse()?;

        let matching_time = Local.with_ymd_and_hms(2023, 2, 5, 0, 0, 0).unwrap();
        let non_matching_time = Local.with_ymd_and_hms(2023, 2, 5, 0, 0, 1).unwrap();

        assert!(cron.is_time_matching(&matching_time)?);
        assert!(!cron.is_time_matching(&non_matching_time)?);

        Ok(())
    }

    #[test]
    fn test_leap_year() -> Result<(), CronError> {
        let cron = Cron::new("0 0 29 2 *").parse()?;
        let leap_year_matching = Local.with_ymd_and_hms(2024, 2, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&leap_year_matching)?);

        Ok(())
    }

    #[test]
    fn test_time_overflow() -> Result<(), CronError> {
        let cron_match = Cron::new("59 59 23 31 12 *")
            .with_seconds_optional()
            .parse()?;
        let cron_next = Cron::new("0 0 0 1 1 *").with_seconds_optional().parse()?;
        let time_matching = Local.with_ymd_and_hms(2023, 12, 31, 23, 59, 59).unwrap();
        let next_day = Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let next_match = Local.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();

        let is_matching = cron_match.is_time_matching(&time_matching)?;
        let next_occurrence = cron_next.find_next_occurrence(&time_matching, false)?;
        let next_match_occurrence = cron_match.find_next_occurrence(&time_matching, false)?;

        assert!(is_matching);
        assert_eq!(next_occurrence, next_day);
        assert_eq!(next_match_occurrence, next_match);

        Ok(())
    }

    #[test]
    fn test_yearly_recurrence() -> Result<(), CronError> {
        let cron = Cron::new("0 0 1 1 *").parse()?;
        let matching_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let non_matching_time = Local.with_ymd_and_hms(2023, 1, 2, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&matching_time)?);
        assert!(!cron.is_time_matching(&non_matching_time)?);

        Ok(())
    }
}
