//! # Croner
//!
//! Croner is a fully-featured, lightweight, and efficient Rust library designed for parsing and evaluating cron patterns.
//!
//! ## Features
//! - Parses a wide range of cron expressions, including extended formats.
//! - Evaluates cron patterns to calculate upcoming and previous execution times.
//! - Supports time zone-aware scheduling.
//! - Offers granularity up to seconds for precise task scheduling.
//! - Compatible with the `chrono` library for dealing with date and time in Rust.
//!
//! ## Crate Features
//! - `serde`: Enables [`serde::Serialize`](https://docs.rs/serde/1/serde/trait.Serialize.html) and
//!   [`serde::Deserialize`](https://docs.rs/serde/1/serde/trait.Deserialize.html) implementations for
//!   [`Cron`](struct.Cron.html). This feature is disabled by default.
//!
//! ## Example
//! The following example demonstrates how to use Croner to parse a cron expression and find the next and previous occurrences.
//!
//! ```rust
//! use chrono::Utc;
//! use croner::Cron;
//!
//! // Parse a cron expression to find occurrences at 00:00 on Friday
//! let cron = Cron::new("0 0 * * FRI").parse().expect("Successful parsing");
//! let now = Utc::now();
//!
//! // Get the next occurrence from the current time
//! let next = cron.find_next_occurrence(&now, false).unwrap();
//!
//! // Get the previous occurrence from the current time
//! let previous = cron.find_previous_occurrence(&now, false).unwrap();
//!
//! println!(
//!     "Pattern \"{}\" will match next at {}",
//!     cron.pattern.to_string(),
//!     next
//! );
//!
//! println!(
//!     "Pattern \"{}\" matched previously at {}",
//!     cron.pattern.to_string(),
//!     previous
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
//! | Field        | Required | Allowed values    | Allowed special characters | Remarks                                                                                         |
//! |--------------|----------|-------------------|----------------------------|-------------------------------------------------------------------------------------------------|
//! | Seconds      | Optional | 0-59              | * , - / ?                  |                                                                                                 |
//! | Minutes      | Yes      | 0-59              | * , - / ?                  |                                                                                                 |
//! | Hours        | Yes      | 0-23              | * , - / ?                  |                                                                                                 |
//! | Day of Month | Yes      | 1-31              | * , - / ? L W              |                                                                                                 |
//! | Month        | Yes      | 1-12 or JAN-DEC   | * , - / ?                  |                                                                                                 |
//! | Day of Week  | Yes      | 0-7 or SUN-MON    | * , - / ? # L              | 0 to 6 are Sunday to Saturday, 7 is Sunday, the same as 0. '#' is used to specify the nth weekday |
//!
//! For more information, refer to the full [README](https://github.com/hexagon/croner-rust).

pub mod errors;

mod component;
mod iterator;
mod pattern;

// Enum to specify the direction of time search, defined locally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Forward,
    Backward,
}
#[derive(PartialEq, Eq, Ord, PartialOrd, Hash, Clone, Copy, Debug)]
pub enum TimeComponent {
    Second = 1,
    Minute,
    Hour,
    Day,
    Month,
}
use errors::CronError;
pub use iterator::CronIterator;
use pattern::CronPattern;
use std::str::FromStr;

use chrono::{
    DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Timelike,
};

#[cfg(feature = "serde")]
use core::fmt;
#[cfg(feature = "serde")]
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize, Serializer,
};

const YEAR_UPPER_LIMIT: i32 = 5000;
const YEAR_LOWER_LIMIT: i32 = 1970;


// The Cron struct represents a cron schedule and provides methods to parse cron strings,
// check if a datetime matches the cron pattern, and find the next occurrence.
#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
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

    /// Evaluates if a given `DateTime` matches the cron pattern.
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
        let naive_time = time.naive_local();
        Ok(self.pattern.second_match(naive_time.second())?
            && self.pattern.minute_match(naive_time.minute())?
            && self.pattern.hour_match(naive_time.hour())?
            && self
                .pattern
                .day_match(naive_time.year(), naive_time.month(), naive_time.day())?
            && self.pattern.month_match(naive_time.month())?)
    }

    /// Finds the next occurrence of a scheduled time that matches the cron pattern.
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
    ) -> Result<DateTime<Tz>, CronError> {
        self.find_occurrence(start_time, inclusive, Direction::Forward)
    }

    /// Finds the previous occurrence of a scheduled time that matches the cron pattern.
    pub fn find_previous_occurrence<Tz: TimeZone>(
        &self,
        start_time: &DateTime<Tz>,
        inclusive: bool,
    ) -> Result<DateTime<Tz>, CronError> {
        self.find_occurrence(start_time, inclusive, Direction::Backward)
    }

    /// The main generic search function.
    /// The main generic search function.
    fn find_occurrence<Tz: TimeZone>(
        &self,
        start_time: &DateTime<Tz>,
        inclusive: bool,
        direction: Direction,
    ) -> Result<DateTime<Tz>, CronError> {
        let mut naive_time = start_time.naive_local();
        let timezone = start_time.timezone();

        if !inclusive {
            let adjustment = match direction {
                Direction::Forward => Duration::seconds(1),
                Direction::Backward => Duration::seconds(-1),
            };
            naive_time = naive_time
                .checked_add_signed(adjustment)
                .ok_or(CronError::InvalidTime)?;
        }

        loop {
            let mut updated = false;
            updated |= self.find_matching_date_component(&mut naive_time, direction, TimeComponent::Month)?;
            updated |= self.find_matching_date_component(&mut naive_time, direction, TimeComponent::Day)?;
            updated |= self.find_matching_granular_component(&mut naive_time, direction, TimeComponent::Hour)?;
            updated |= self.find_matching_granular_component(&mut naive_time, direction, TimeComponent::Minute)?;
            updated |= self.find_matching_granular_component(&mut naive_time, direction, TimeComponent::Second)?;

            if updated {
                continue;
            }

            let (tz_datetime, was_adjusted) = from_naive(naive_time, &timezone)?;
            if self.is_time_matching(&tz_datetime)? || was_adjusted {
                return Ok(tz_datetime);
            } else {
                return Err(CronError::TimeSearchLimitExceeded);
            }
        }
    }

    /// Creates a `CronIterator` starting from the specified time.
    ///
    /// The search can be performed forwards or backwards in time.
    ///
    /// # Arguments
    ///
    /// * `start_from` - A `DateTime<Tz>` that represents the starting point for the iterator.
    /// * `direction` - A `Direction` to specify the search direction.
    ///
    /// # Returns
    ///
    /// Returns a `CronIterator<Tz>` that can be used to iterate over scheduled times.
    pub fn iter_from<Tz: TimeZone>(
        &self,
        start_from: DateTime<Tz>,
        direction: Direction,
    ) -> CronIterator<Tz> {
        CronIterator::new(
            self.clone(),
            start_from,
            true,
            direction,
        )
    }

    /// Creates a `CronIterator` starting after the specified time, in forward direction.
    ///
    /// # Arguments
    ///
    /// * `start_after` - A `DateTime<Tz>` that represents the starting point for the iterator.
    ///
    /// # Returns
    ///
    /// Returns a `CronIterator<Tz>` that can be used to iterate over scheduled times.
    pub fn iter_after<Tz: TimeZone>(
        &self,
        start_after: DateTime<Tz>
    ) -> CronIterator<Tz> {
        CronIterator::new(
            self.clone(),
            start_after,
            false,
            Direction::Forward,
        )
    }

    /// Creates a `CronIterator` starting before the specified time, in backwards direction.
    ///
    /// # Arguments
    ///
    /// * `start_before` - A `DateTime<Tz>` that represents the starting point for the iterator.
    ///
    /// # Returns
    ///
    /// Returns a `CronIterator<Tz>` that can be used to iterate over scheduled times.
    pub fn iter_before<Tz: TimeZone>(
        &self,
        start_before: DateTime<Tz>
    ) -> CronIterator<Tz> {
        CronIterator::new(
            self.clone(),
            start_before,
            false,
            Direction::Backward,
        )
    }

    // TIME MANIPULATION FUNCTIONS

    /// Sets a time component and resets lower-order ones based on direction.
    fn set_time_component(
        current_time: &mut NaiveDateTime,
        component: TimeComponent,
        value: u32,
        direction: Direction,
    ) -> Result<(), CronError> {
        let mut new_time = *current_time;

        new_time = match component {
            TimeComponent::Second => new_time.with_second(value).ok_or(CronError::InvalidTime)?,
            TimeComponent::Minute => new_time.with_minute(value).ok_or(CronError::InvalidTime)?,
            TimeComponent::Hour => new_time.with_hour(value).ok_or(CronError::InvalidTime)?,
            _ => return Err(CronError::InvalidTime),
        };

        match direction {
            Direction::Forward => {
                if component >= TimeComponent::Hour {
                    new_time = new_time.with_minute(0).unwrap();
                }
                if component >= TimeComponent::Minute {
                    new_time = new_time.with_second(0).unwrap();
                }
            }
            Direction::Backward => {
                if component >= TimeComponent::Hour {
                    new_time = new_time.with_minute(59).unwrap();
                }
                if component >= TimeComponent::Minute {
                    new_time = new_time.with_second(59).unwrap();
                }
            }
        }

        *current_time = new_time;
        Ok(())
    }

    /// Adjusts a time component up or down, resetting lower-order ones.
    fn adjust_time_component(
        current_time: &mut NaiveDateTime,
        component: TimeComponent,
        direction: Direction,
    ) -> Result<(), CronError> {
        let limit = match direction {
            Direction::Forward => YEAR_UPPER_LIMIT,
            Direction::Backward => YEAR_LOWER_LIMIT,
        };

        if current_time.year() == limit {
            return Err(CronError::TimeSearchLimitExceeded);
        }

        match direction {
            Direction::Forward => {
                let duration = match component {
                    TimeComponent::Minute => Duration::minutes(1),
                    TimeComponent::Hour => Duration::hours(1),
                    TimeComponent::Day => Duration::days(1),
                    TimeComponent::Month => {
                        let mut year = current_time.year();
                        let mut month = current_time.month() + 1;
                        if month > 12 {
                            year += 1;
                            month = 1;
                        }
                        *current_time = NaiveDate::from_ymd_opt(year, month, 1)
                            .ok_or(CronError::InvalidDate)?
                            .and_hms_opt(0, 0, 0)
                            .ok_or(CronError::InvalidTime)?;
                        return Ok(());
                    }
                    _ => return Err(CronError::InvalidTime),
                };
                *current_time = current_time
                    .checked_add_signed(duration)
                    .ok_or(CronError::InvalidTime)?;
                if component >= TimeComponent::Day {
                    *current_time = current_time.with_hour(0).unwrap();
                }
                if component >= TimeComponent::Hour {
                    *current_time = current_time.with_minute(0).unwrap();
                }
                if component >= TimeComponent::Minute {
                    *current_time = current_time.with_second(0).unwrap();
                }
            }
            Direction::Backward => {
                let duration = match component {
                    TimeComponent::Minute => Duration::minutes(1),
                    TimeComponent::Hour => Duration::hours(1),
                    TimeComponent::Day => Duration::days(1),
                    TimeComponent::Month => {
                        let next_month_first_day = NaiveDate::from_ymd_opt(
                            current_time.year(),
                            current_time.month(),
                            1,
                        )
                        .ok_or(CronError::InvalidDate)?;
                        *current_time = (next_month_first_day - Duration::days(1))
                            .and_hms_opt(23, 59, 59)
                            .ok_or(CronError::InvalidTime)?;
                        return Ok(());
                    }
                    _ => return Err(CronError::InvalidTime),
                };
                *current_time = current_time
                    .checked_sub_signed(duration)
                    .ok_or(CronError::InvalidTime)?;
                if component >= TimeComponent::Day {
                    *current_time = current_time.with_hour(23).unwrap();
                }
                if component >= TimeComponent::Hour {
                    *current_time = current_time.with_minute(59).unwrap();
                }
                if component >= TimeComponent::Minute {
                    *current_time = current_time.with_second(59).unwrap();
                }
            }
        }
        Ok(())
    }
    fn find_matching_date_component(
        &self,
        current_time: &mut NaiveDateTime,
        direction: Direction,
        component: TimeComponent,
    ) -> Result<bool, CronError> {
        let mut changed = false;
        // Loop until the component matches the pattern
        while !(match component {
            TimeComponent::Month => self.pattern.month_match(current_time.month()),
            TimeComponent::Day => self.pattern.day_match(
                current_time.year(),
                current_time.month(),
                current_time.day(),
            ),
            _ => Ok(true), // Should not happen for other components, but this is safe
        })? {
            Self::adjust_time_component(current_time, component, direction)?;
            changed = true;
        }
        Ok(changed)
    }

    /// Consolidated helper for time-based components (Hour, Minute, Second).
    fn find_matching_granular_component(
        &self,
        current_time: &mut NaiveDateTime,
        direction: Direction,
        component: TimeComponent,
    ) -> Result<bool, CronError> {
        let mut changed = false;
        let (current_value, next_larger_component) = match component {
            TimeComponent::Hour => (current_time.hour(), TimeComponent::Day),
            TimeComponent::Minute => (current_time.minute(), TimeComponent::Hour),
            TimeComponent::Second => (current_time.second(), TimeComponent::Minute),
            _ => return Err(CronError::InvalidTime),
        };

        let match_result = self
            .pattern
            .find_match_in_component(current_value, component, direction)?;

        match match_result {
            Some(match_value) => {
                if match_value != current_value {
                    Self::set_time_component(current_time, component, match_value, direction)?;
                }
            }
            None => {
                Self::adjust_time_component(current_time, next_larger_component, direction)?;
                changed = true;
            }
        }
        Ok(changed)
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

    pub fn as_str(&self) -> &str {
        self.pattern.as_str()
    }
}

impl std::fmt::Display for Cron {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pattern)
    }
}

// Enables creating a Cron instance from a string slice, returning a CronError if parsing fails.
impl FromStr for Cron {
    type Err = CronError;

    fn from_str(cron_string: &str) -> Result<Cron, CronError> {
        Cron::new(cron_string).parse()
    }
}

#[cfg(feature = "serde")]
impl Serialize for Cron {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.pattern.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Cron {
    fn deserialize<D>(deserializer: D) -> Result<Cron, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct CronVisitor;

        impl Visitor<'_> for CronVisitor {
            type Value = Cron;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid cron pattern")
            }

            fn visit_str<E>(self, value: &str) -> Result<Cron, E>
            where
                E: de::Error,
            {
                Cron::new(value).parse().map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(CronVisitor)
    }
}

// Convert `NaiveDateTime` back to `DateTime<Tz>`
pub fn from_naive<Tz: TimeZone>(
    naive_time: NaiveDateTime,
    timezone: &Tz,
) -> Result<(DateTime<Tz>, bool), CronError> {
    match timezone.from_local_datetime(&naive_time) {
        chrono::LocalResult::Single(dt) => Ok((dt, false)),
        chrono::LocalResult::Ambiguous(dt1, _) => Ok((dt1, false)),
        chrono::LocalResult::None => {
            // Handle DST gap by searching nearby
            for i in 0..3600 {
                let adjusted = naive_time + Duration::seconds(i + 1);
                if let chrono::LocalResult::Single(dt) = timezone.from_local_datetime(&adjusted) {
                    return Ok((dt, true));
                }
            }
            Err(CronError::InvalidTime)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::hash::{DefaultHasher, Hash, Hasher as _};

    use super::*;
    use chrono::{Local, TimeZone};
    use rstest::rstest;
    #[cfg(feature = "serde")]
    use serde_test::{assert_de_tokens_error, assert_tokens, Token};
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
        let expected_dates = [
            Local.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 12, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 17, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 24, 0, 0, 0).unwrap(),
        ];

        // Iterate over the expected dates, checking each one
        for (idx, current_date) in cron.clone().iter_from(start_date, Direction::Forward).take(5) {
            assert_eq!(expected_dates[idx], current_date);
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
        let expected_dates = [
            Local.with_ymd_and_hms(2027, 12, 31, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2032, 12, 31, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2038, 12, 31, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2049, 12, 31, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2055, 12, 31, 0, 0, 0).unwrap(),
        ];

        // Iterate over the expected dates, checking each one
        let mut idx = 0;
        for current_date in cron.clone().iter_from(start_date, Direction::Forward).take(5) {
            assert_eq!(expected_dates[idx], current_date);
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
            "0 0 5,L * 7#2",
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

    // Unusual cron pattern found online, perfect for testing
    #[test]
    fn test_unusual_cron_expression_end_month_start_month_mon() -> Result<(), CronError> {
        use chrono::TimeZone;

        // Parse the cron expression with specified options
        let cron = Cron::new("0 0 */31,1-7 */1 MON").parse()?;

        // Define the start date for the test
        let start_date = Local.with_ymd_and_hms(2023, 12, 24, 0, 0, 0).unwrap();

        // Define the expected matching dates
        let expected_dates = vec![
            Local.with_ymd_and_hms(2023, 12, 25, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 4, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 5, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 6, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 7, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 22, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 1, 29, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap(),
        ];

        // Iterate over the expected dates, checking each one
        let mut idx = 0;
        for current_date in cron.iter_from(start_date, Direction::Forward).take(expected_dates.len()) {
            assert_eq!(expected_dates[idx], current_date);
            idx += 1;
        }

        assert_eq!(idx, 13);

        Ok(())
    }

    // Unusual cron pattern found online, perfect for testing, with dom_and_dow
    #[test]
    fn test_unusual_cron_expression_end_month_start_month_mon_dom_and_dow() -> Result<(), CronError>
    {
        use chrono::TimeZone;

        // Parse the cron expression with specified options
        let cron = Cron::new("0 0 */31,1-7 */1 MON")
            .with_dom_and_dow()
            .with_seconds_optional() // Just to differ as much from the non dom-and-dow test
            .parse()?;

        // Define the start date for the test
        let start_date = Local.with_ymd_and_hms(2023, 12, 24, 0, 0, 0).unwrap();

        // Define the expected matching dates
        let expected_dates = [
            Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 2, 5, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 3, 4, 0, 0, 0).unwrap(),
        ];

        // Iterate over the expected dates, checking each one
        let mut idx = 0;
        for current_date in cron.iter_from(start_date, Direction::Forward).take(expected_dates.len()) {
            assert_eq!(expected_dates[idx], current_date);
            idx += 1;
        }

        assert_eq!(idx, 3);

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
        let expected_dates = [
            Local.with_ymd_and_hms(2024, 3, 29, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2030, 3, 29, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2036, 2, 29, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2041, 3, 29, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2047, 3, 29, 0, 0, 0).unwrap(),
        ];

        // Iterate over the expected dates, checking each one
        let mut idx = 0;
        for current_date in cron.iter_from(start_date, Direction::Forward).take(5) {
            assert_eq!(expected_dates[idx], current_date);
            idx += 1;
        }

        assert_eq!(idx, 5);

        Ok(())
    }

    #[test]
    fn test_cron_expression_second_sunday_using_seven() -> Result<(), CronError> {
        use chrono::TimeZone;

        // Parse the cron expression with specified options
        let cron = Cron::new("0 0 0 * * 7#2").with_seconds_optional().parse()?;

        // Define the start date for the test
        let start_date = Local.with_ymd_and_hms(2024, 10, 1, 0, 0, 0).unwrap();

        // Define the expected matching dates
        let expected_dates = [
            Local.with_ymd_and_hms(2024, 10, 13, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 11, 10, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2024, 12, 8, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2025, 1, 12, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2025, 2, 9, 0, 0, 0).unwrap(),
        ];

        // Iterate over the expected dates, checking each one
        let mut idx = 0;
        for current_date in cron.iter_from(start_date, Direction::Forward).take(5) {
            assert_eq!(expected_dates[idx], current_date);
            idx += 1;
        }

        assert_eq!(idx, 5);

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

    /// Utility function used in hashing test
    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    #[rstest]
    // Frequency & Nicknames
    #[case("@hourly", "@daily", false)]
    #[case("@daily", "@weekly", false)]
    #[case("@weekly", "@monthly", false)]
    #[case("@monthly", "@yearly", false)]
    #[case("* * * * *", "@hourly", false)]
    #[case("@annually", "@yearly", true)]
    // Optional Seconds Field (5 vs 6 fields)
    #[case("* * * * * *", "* * * * *", false)]
    #[case("0 12 * * *", "30 0 12 * * *", false)]
    #[case("0 0 * * * *", "@hourly", false)]
    // Field Specificity (Earlier vs. Later)
    #[case("5 * * * * *", "10 * * * * *", false)]
    #[case("15 * * * *", "45 * * * *", false)]
    #[case("* * 8 * *", "* * 18 * *", false)]
    #[case("* * * 1 *", "* * * 6 *", false)]
    #[case("* * * JAN *", "* * * JUL *", false)]
    #[case("* * * * 0", "* * * * 3", false)]
    #[case("* * * * SUN", "* * * * WED", false)]
    #[case("* * * * 7", "* * * * 1", false)]
    // Ranges (`-`)
    #[case("0-29 * * * *", "30-59 * * * *", false)]
    #[case("* * 1-11 * *", "* * 12-23 * *", false)]
    #[case("* * * JAN-JUN *", "* * * JUL-DEC *", false)]
    #[case("* * * * MON-WED", "* * * * THU-SAT", false)]
    #[case("* * * * *", "0-5 * * * *", false)]
    // Steps (`/`)
    #[case("*/15 * * * *", "*/30 * * * *", false)]
    #[case("0/10 * * * *", "5/10 * * * *", false)]
    #[case("* * 1-10/2 * *", "* * 1-10/3 * *", false)]
    #[case("* * * * *", "*/2 * * * *", false)]
    // Lists (`,`)
    #[case("0,10,20 * * * *", "30,40,50 * * * *", false)]
    #[case("* * * * MON,WED,FRI", "* * * * TUE,THU,SAT", false)]
    // Equivalency & Wildcards
    #[case("? ? ? ? ? ?", "* * * * * *", true)]
    #[case("0,15,30,45 * * * *", "*/15 * * * *", true)]
    #[case("@monthly", "0 0 1 * *", true)]
    #[case("* * * * 1,3,5", "* * * * MON,WED,FRI", true)]
    #[case("* * * mar *", "* * * 3 *", true)]
    // #[case("0 0 1-7 * 1", "0 0 * * 1#1", true)]
    // #[case("0 0 8-14 * MON", "0 0 * * MON#2", true)]
    // Day-of-Month vs. Day-of-Week
    #[case("0 0 * * 1", "0 0 15 * *", false)]
    #[case("0 0 1 * *", "0 0 1 * 1", false)]
    // Special Character `L` (Last)
    #[case("* * 1 * *", "* * L * *", false)]
    #[case("* * L FEB *", "* * L MAR *", false)]
    #[case("* * * * 1#L", "* * * * 2#L", false)]
    #[case("* * * * 4#L", "* * * * FRI#L", false)]
    // Special Character `W` (Weekday)
    #[case("* * 1W * *", "* * 1 * *", false)]
    #[case("* * 15W * *", "* * 16W * *", false)]
    // Special Character `#` (Nth Weekday)
    #[case("* * * * 1#2", "* * * * 1#1", false)]
    #[case("* * * * TUE#4", "* * * * TUE#2", false)]
    #[case("* * * * 5#1", "* * * * FRI#1", true)]
    #[case("* * * * MON#1", "* * * * TUE#1", false)]
    // Complex Combinations
    #[case("0 10 * * MON#2", "0 10 1-7 * MON", false)]
    #[case("*/10 8-10 * JAN,DEC 1-5", "0 12 * * 6", false)]
    fn test_comparison_and_hash(
        #[case] pattern_1: &str,
        #[case] pattern_2: &str,
        #[case] equal: bool,
    ) {
        eprintln!("Parsing {pattern_1}");
        let cron_1 = Cron::new(pattern_1).parse().unwrap_or_else(|err| {
            eprintln!(
                "Initial parse attempt failed ({err}). Trying again but with allowed seconds."
            );
            Cron::new(pattern_1)
                .with_seconds_required()
                .parse()
                .unwrap()
        });

        eprintln!("Parsing {pattern_2}");
        let cron_2 = Cron::new(pattern_2).parse().unwrap_or_else(|err| {
            eprintln!(
                "Initial parse attempt failed ({err}). Trying again but with allowed seconds."
            );
            Cron::new(pattern_2)
                .with_seconds_required()
                .parse()
                .unwrap()
        });

        assert_eq!(
            cron_1 == cron_2,
            equal,
            "Equality relation between both patterns is not {equal}"
        );
        assert_eq!(
            calculate_hash(&cron_1) == calculate_hash(&cron_2),
            equal,
            "Hashes don't respect quality relation"
        );

        if !equal {
            assert!(
                cron_1 > cron_2,
                "Ordering between first an second pattern is wrong"
            );
        }

        #[expect(clippy::eq_op, reason = "Want to check Eq is correctly implemented")]
        {
            assert!(
                cron_1 == cron_1,
                "Eq implementation is incorrect for first patter"
            );
            assert!(
                cron_2 == cron_2,
                "Eq implementation is incorrect for second patter"
            );
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_tokens() {
        let cron = Cron::new("0 0 * * *")
            .parse()
            .expect("should be valid pattern");
        assert_tokens(&cron.to_string(), &[Token::Str("0 0 * * *")]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_shorthand_serde_tokens() {
        let expressions = [
            ("@daily", "0 0 * * *"),
            ("0 12 * * MON", "0 12 * * 1"),
            ("*/15 9-17 * * MON-FRI", "*/15 9-17 * * 1-5"),
        ];
        for (shorthand, expected) in expressions.iter() {
            let cron = Cron::new(shorthand)
                .parse()
                .expect("should be valid pattern");
            assert_tokens(&cron.to_string(), &[Token::Str(expected)]);
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_invalid_serde_tokens() {
        assert_de_tokens_error::<Cron>(
            &[Token::Str("Invalid cron pattern")],
            "Invalid pattern: Pattern must consist of five or six fields (minute, hour, day, month, day of week, and optional second)."
        );
    }

    #[test]
    fn test_find_previous_occurrence() -> Result<(), CronError> {
        let cron = Cron::new("* * * * *").parse()?;
        let start_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 1, 30).unwrap();
        let prev_occurrence = cron.find_previous_occurrence(&start_time, false)?;
        let expected_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 1, 0).unwrap();
        assert_eq!(prev_occurrence, expected_time);
        Ok(())
    }

    #[test]
    fn test_find_previous_occurrence_inclusive() -> Result<(), CronError> {
        let cron = Cron::new("* * * * *").parse()?;
        let start_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 1, 0).unwrap();
        let prev_occurrence = cron.find_previous_occurrence(&start_time, true)?;
        assert_eq!(prev_occurrence, start_time);
        Ok(())
    }

    #[test]
    fn test_wrap_year_backwards() -> Result<(), CronError> {
        let cron = Cron::new("0 0 1 1 *").parse()?; // Jan 1st, 00:00
        let start_time = Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 1).unwrap();
        let prev_occurrence = cron.find_previous_occurrence(&start_time, false)?;
        let expected_time = Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(prev_occurrence, expected_time);

        let start_time_2 = Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let prev_occurrence_2 = cron.find_previous_occurrence(&start_time_2, false)?;
        let expected_time_2 = Local.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(prev_occurrence_2, expected_time_2);
        Ok(())
    }
}
