//! # Croner
//!
//! Croner is a fully-featured, lightweight, and efficient Rust library designed for parsing and evaluating cron patterns.
//!
//! ## Features
//! - Parses a wide range of cron expressions, including extended formats.
//! - Generates human-readable descriptions of cron patterns.
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
//! use std::str::FromStr as _;
//!
//! use chrono::Utc;
//! use croner::Cron;
//!
//! // Parse a cron expression to find occurrences at 00:00 on Friday
//! let cron = Cron::from_str("0 0 * * FRI").expect("Successful parsing");
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
//! In this example, `Cron::from_str("0 0 * * FRI")` creates a new Cron instance for the pattern that represents every Friday at midnight. The `find_next_occurrence` method calculates the next time this pattern will be true from the current moment.
//!
//! The `false` argument in `find_next_occurrence` specifies that the current time is not included in the calculation, ensuring that only future occurrences are considered.
//!
//! ## Describing a Pattern
//! Croner can also generate a human-readable, English description of a cron pattern. This is highly useful for displaying schedule information in a UI or for debugging complex patterns.
//!
//! The .describe() method returns a String detailing what the schedule means.
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
pub mod parser;
pub mod describe;

mod component;
mod iterator;
mod pattern;

// Enum to specify the direction of time search
#[derive(Clone, Copy, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
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
    Year
}

/// Categorizes a cron pattern as either a Fixed-Time Job or an Interval/Wildcard Job.
/// This is used to apply specific Daylight Saving Time (DST) transition rules.
#[derive(Debug, PartialEq, Eq)]
pub enum JobType {
    FixedTime,
    IntervalWildcard,
}

use errors::CronError;
pub use iterator::CronIterator;
use parser::CronParser;
use pattern::CronPattern;
use std::str::FromStr;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Timelike};

#[cfg(feature = "serde")]
use core::fmt;
#[cfg(feature = "serde")]
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize, Serializer,
};

/// Safeguard to prevent infinite loops when searching for future
/// occurrences of a cron pattern that may never match. It ensures that the search
/// function will eventually terminate and return an error instead of running indefinitely.
pub const YEAR_UPPER_LIMIT: i32 = 5000;

/// Sets the lower year limit to 1 AD/CE.
/// This is a pragmatic choice to avoid the complexities of year 0 (1 BCE) and pre-CE
/// dates, which involve different calendar systems and are outside the scope of a
/// modern scheduling library.
pub const YEAR_LOWER_LIMIT: i32 = 1;

// The Cron struct represents a cron schedule and provides methods to parse cron strings,
// check if a datetime matches the cron pattern, and find the next occurrence.
#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct Cron {
    pub pattern: CronPattern, // Parsed cron pattern
}

impl FromStr for Cron {
    type Err = CronError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CronParser::new().parse(s)
    }
}

impl Cron {
    /// Evaluates if a given `DateTime` matches the cron pattern.
    ///
    /// The function checks each cron field (seconds, minutes, hours, day of month, month and 
    /// year) against the provided `DateTime` to determine if it aligns with the cron pattern. 
    /// Each field is checked for a match, and all fields must match for the entire pattern 
    /// to be considered a match.
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
    /// use std::str::FromStr as _;
    ///
    /// use croner::Cron;
    /// use chrono::Utc;
    ///
    /// // Parse cron expression
    /// let cron: Cron = Cron::from_str("* * * * *").expect("Couldn't parse cron string");
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
            && self.pattern.month_match(naive_time.month())?
            && self.pattern.year_match(naive_time.year())?) // Add year match check
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
    /// use croner::{Cron, parser::{Seconds, CronParser}};
    ///
    /// // Parse cron expression
    /// let cron: Cron = CronParser::builder().seconds(Seconds::Required).build().parse("0 18 * * * 5").expect("Success");
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
            .map(|(dt, _)| dt)
    }

    /// Finds the previous occurrence of a scheduled time that matches the cron pattern.
    pub fn find_previous_occurrence<Tz: TimeZone>(
        &self,
        start_time: &DateTime<Tz>,
        inclusive: bool,
    ) -> Result<DateTime<Tz>, CronError> {
        self.find_occurrence(start_time, inclusive, Direction::Backward)
            .map(|(dt, _)| dt) // Take only the first element (DateTime<Tz>)
    }

    /// The main generic search function.
    /// Returns (found_datetime, optional_second_ambiguous_datetime_if_any)
    fn find_occurrence<Tz: TimeZone>(
        &self,
        start_time: &DateTime<Tz>,
        inclusive: bool,
        direction: Direction,
    ) -> Result<(DateTime<Tz>, Option<DateTime<Tz>>), CronError> {
        let mut naive_time = start_time.naive_local();
        let timezone = start_time.timezone();
        let job_type = self.determine_job_type();

        let initial_adjusted_naive_time = if !inclusive {
            let adjustment = match direction {
                Direction::Forward => Duration::seconds(1),
                Direction::Backward => Duration::seconds(-1),
            };
            naive_time
                .checked_add_signed(adjustment)
                .ok_or(CronError::InvalidTime)?
        } else {
            naive_time
        };

        naive_time = initial_adjusted_naive_time;

        let mut iterations = 0;
        const MAX_SEARCH_ITERATIONS: u32 = 366 * 24 * 60 * 60;

        loop {
            iterations += 1;
            if iterations > MAX_SEARCH_ITERATIONS {
                return Err(CronError::TimeSearchLimitExceeded);
            }

            let mut changed_component_in_this_pass = false;

            changed_component_in_this_pass |= self.find_matching_date_component(&mut naive_time, direction, TimeComponent::Year)?;
            if !changed_component_in_this_pass {
                changed_component_in_this_pass |= self.find_matching_date_component(&mut naive_time, direction, TimeComponent::Month)?;
            }
            if !changed_component_in_this_pass {
                changed_component_in_this_pass |= self.find_matching_date_component(&mut naive_time, direction, TimeComponent::Day)?;
            }

            if changed_component_in_this_pass {
                match direction {
                    Direction::Forward => naive_time = naive_time.with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap(),
                    Direction::Backward => naive_time = naive_time.with_hour(23).unwrap().with_minute(59).unwrap().with_second(59).unwrap(),
                }
            }

            let mut time_component_adjusted_in_this_pass = false;
            time_component_adjusted_in_this_pass |= self.find_matching_granular_component(&mut naive_time, direction, TimeComponent::Hour)?;
            if !time_component_adjusted_in_this_pass {
                time_component_adjusted_in_this_pass |= self.find_matching_granular_component(&mut naive_time, direction, TimeComponent::Minute)?;
            }
            if !time_component_adjusted_in_this_pass {
                self.find_matching_granular_component(&mut naive_time, direction, TimeComponent::Second)?;
            }

            match from_naive(naive_time, &timezone) {
                chrono::LocalResult::Single(dt) => {
                    if self.is_time_matching(&dt)? {
                        return Ok((dt, None)); // Single match, no second ambiguous time
                    }
                    naive_time = naive_time.checked_add_signed(match direction {
                        Direction::Forward => Duration::seconds(1),
                        Direction::Backward => Duration::seconds(-1),
                    }).ok_or(CronError::InvalidTime)?;
                }
                chrono::LocalResult::Ambiguous(_dt1, _dt2) => {
                    // DST Overlap (Fall Back)
                    let first_occurrence_dt = timezone.from_local_datetime(&naive_time).earliest().unwrap();
                    let second_occurrence_dt = timezone.from_local_datetime(&naive_time).latest().unwrap();

                    if job_type == JobType::FixedTime {
                        // Fixed-Time Job: Execute only once, at its first occurrence (earliest in the ambiguous pair).
                        if self.is_time_matching(&first_occurrence_dt)? {
                            return Ok((first_occurrence_dt, None)); // Return only the first, no second for fixed jobs.
                        }
                        // If fixed time doesn't match first_occurrence_dt, it means this particular naive_time
                        // doesn't match the fixed pattern's exact time (e.g., cron is "0 0 2 *" and naive is 02:30:00).
                        // So, we just advance to the next second and continue the loop.
                        naive_time = naive_time.checked_add_signed(match direction {
                            Direction::Forward => Duration::seconds(1),
                            Direction::Backward => Duration::seconds(-1),
                        }).ok_or(CronError::InvalidTime)?;

                    } else { // Interval/Wildcard Job
                        // Interval/Wildcard Job: Execute for each occurrence that matches.
                        let mut primary_match = None;
                        let mut secondary_match = None;

                        if self.is_time_matching(&first_occurrence_dt)? {
                            primary_match = Some(first_occurrence_dt);
                        }
                        if self.is_time_matching(&second_occurrence_dt)? {
                            secondary_match = Some(second_occurrence_dt);
                        }

                        if let Some(p_match) = primary_match {
                            return Ok((p_match, secondary_match)); // Return first, and potentially the second.
                        } else if let Some(s_match) = secondary_match {
                            // Only the second occurrence matched, return it as primary.
                            return Ok((s_match, None)); // No secondary from this point.
                        }
                        // If neither matched the pattern for this ambiguous naive_time, advance and continue.
                        naive_time = naive_time.checked_add_signed(match direction {
                            Direction::Forward => Duration::seconds(1),
                            Direction::Backward => Duration::seconds(-1),
                        }).ok_or(CronError::InvalidTime)?;
                    }
                }
                chrono::LocalResult::None => {
                    // DST Gap (Spring Forward)
                    if job_type == JobType::FixedTime {
                        // For fixed-time jobs that fall into a gap, we want them to "snap" to the first valid time after the gap.
                        // Find the very first valid NaiveDateTime after the current `naive_time`
                        // that can be successfully converted to a DateTime<Tz>.
                        let mut temp_naive = naive_time;
                        let mut gap_adjust_count = 0;
                        const MAX_GAP_SEARCH_SECONDS: u32 = 3600 * 2; // Max 2 hours for a typical gap

                        let resolved_dt_after_gap: DateTime<Tz>;

                        loop {
                            temp_naive = temp_naive.checked_add_signed(match direction {
                                Direction::Forward => Duration::seconds(1),
                                Direction::Backward => Duration::seconds(-1),
                            }).ok_or(CronError::InvalidTime)?;
                            gap_adjust_count += 1;
                            
                            // Try to resolve this `temp_naive` into a real DateTime.
                            let local_result = from_naive(temp_naive, &timezone);

                            if let chrono::LocalResult::Single(dt) = local_result {
                                resolved_dt_after_gap = dt;
                                break;
                            } else if let chrono::LocalResult::Ambiguous(dt1, _) = local_result {
                                // If it resolves to ambiguous (unlikely right at a gap boundary for Single), take the earliest.
                                resolved_dt_after_gap = dt1; 
                                break;
                            }
                            // Keep looping if still None or search limit exceeded
                            if gap_adjust_count > MAX_GAP_SEARCH_SECONDS {
                                return Err(CronError::TimeSearchLimitExceeded);
                            }
                        }

                        // `resolved_dt_after_gap` is now the first valid wall-clock time after the gap.
                        // For a fixed-time job that fell into the gap, this is the time it should run.
                        // We must ensure that its date components (year, month, day, day of week) still match the pattern.
                        // We do NOT check the original fixed hour/minute/second from the pattern, as they were "missing".
                        if self.pattern.day_match(resolved_dt_after_gap.year(), resolved_dt_after_gap.month(), resolved_dt_after_gap.day())? &&
                           self.pattern.month_match(resolved_dt_after_gap.month())? &&
                           self.pattern.year_match(resolved_dt_after_gap.year())? {
                            // No need to update naive_time here
                            return Ok((resolved_dt_after_gap, None)); 
                        } else {
                            // If even the date components of this post-gap time do not match the pattern,
                            // then the fixed job's *date* itself was not the one containing the gap.
                            // In this case, we simply advance `naive_time` past the gap
                            // and let the main loop continue searching for the next matching date.
                            naive_time = temp_naive;
                            continue;
                        }
                    } else { // Interval/Wildcard Job in DST Gap
                        // Existing logic: simply advance by one second/minute
                        naive_time = naive_time.checked_add_signed(match direction {
                            Direction::Forward => Duration::seconds(1),
                            Direction::Backward => Duration::seconds(-1),
                        }).ok_or(CronError::InvalidTime)?;
                    }
                }
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
        CronIterator::new(self.clone(), start_from, true, direction)
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
    pub fn iter_after<Tz: TimeZone>(&self, start_after: DateTime<Tz>) -> CronIterator<Tz> {
        CronIterator::new(self.clone(), start_after, false, Direction::Forward)
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
    pub fn iter_before<Tz: TimeZone>(&self, start_before: DateTime<Tz>) -> CronIterator<Tz> {
        CronIterator::new(self.clone(), start_before, false, Direction::Backward)
    }
  
    /// Returns a human-readable description of the cron pattern.
    ///
    /// This method provides a best-effort English description of the cron schedule.
    /// Note: The cron instance must be parsed successfully before calling this method.
    ///
    /// # Example
    /// ```
    /// use croner::Cron;
    /// use std::str::FromStr as _;
    ///
    /// let cron = Cron::from_str("0 12 * * MON-FRI").unwrap();
    /// println!("{}", cron.describe());
    /// // Output: At on minute 0, at hour 12, on Monday,Tuesday,Wednesday,Thursday,Friday.
    /// ```
    pub fn describe(&self) -> String {
        self.pattern.describe()
    }

    /// Returns a human-readable description using a provided language provider.
    ///
    /// # Arguments
    ///
    /// * `lang` - An object that implements the `Language` trait.
    pub fn describe_lang<L: crate::describe::Language>(&self, lang: L) -> String {
        self.pattern.describe_lang(lang)
    }
  
    /// Determines if the cron pattern represents a Fixed-Time Job or an Interval/Wildcard Job.
    /// A Fixed-Time Job has fixed (non-wildcard, non-stepped, single-value) Seconds, Minute,
    /// and Hour fields. Otherwise, it's an Interval/Wildcard Job.
    pub fn determine_job_type(&self) -> JobType {
        let is_seconds_fixed = self.pattern.seconds.step == 1
            && !self.pattern.seconds.from_wildcard
            && self.pattern.seconds.get_set_values(component::ALL_BIT).len() == 1;
        let is_minutes_fixed = self.pattern.minutes.step == 1
            && !self.pattern.minutes.from_wildcard
            && self.pattern.minutes.get_set_values(component::ALL_BIT).len() == 1;
        let is_hours_fixed = self.pattern.hours.step == 1
            && !self.pattern.hours.from_wildcard
            && self.pattern.hours.get_set_values(component::ALL_BIT).len() == 1;

        if is_seconds_fixed && is_minutes_fixed && is_hours_fixed {
            JobType::FixedTime
        } else {
            JobType::IntervalWildcard
        }
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
        // Check for limits
        match direction {
            Direction::Forward => {
                if current_time.year() >= YEAR_UPPER_LIMIT {
                    return Err(CronError::TimeSearchLimitExceeded);
                }
            }
            Direction::Backward => {
                if current_time.year() <= YEAR_LOWER_LIMIT {
                    return Err(CronError::TimeSearchLimitExceeded);
                }
            }
        }
        match direction {
            Direction::Forward => {
                let duration = match component {
                    TimeComponent::Year => {
                        let next_year = current_time.year() + 1;
                        *current_time = NaiveDate::from_ymd_opt(next_year, 1, 1)
                            .ok_or(CronError::InvalidDate)?
                            .and_hms_opt(0, 0, 0)
                            .ok_or(CronError::InvalidTime)?;
                        return Ok(());
                    }
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
                    TimeComponent::Year => { // Tillagd logik för år
                        let prev_year = current_time.year() - 1;
                        *current_time = NaiveDate::from_ymd_opt(prev_year, 12, 31)
                            .ok_or(CronError::InvalidDate)?
                            .and_hms_opt(23, 59, 59)
                            .ok_or(CronError::InvalidTime)?;
                        return Ok(());
                    }
                    TimeComponent::Minute => Duration::minutes(1),
                    TimeComponent::Hour => Duration::hours(1),
                    TimeComponent::Day => Duration::days(1),
                    TimeComponent::Month => {
                        let next_month_first_day =
                            NaiveDate::from_ymd_opt(current_time.year(), current_time.month(), 1)
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
            TimeComponent::Year => self.pattern.year_match(current_time.year()), // Tillagd
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

        let match_result =
            self.pattern
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

    pub fn as_str(&self) -> &str {
        self.pattern.as_str()
    }
}

impl std::fmt::Display for Cron {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pattern)
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
                Cron::from_str(value).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(CronVisitor)
    }
}

// Convert `NaiveDateTime` back to `DateTime<Tz>`
pub fn from_naive<Tz: TimeZone>(
    naive_time: NaiveDateTime,
    timezone: &Tz,
) -> chrono::LocalResult<DateTime<Tz>> {
    timezone.from_local_datetime(&naive_time)
}

#[cfg(test)]
mod tests {
    use std::hash::{DefaultHasher, Hash, Hasher as _};

    use crate::parser::Seconds;

    use super::*;
    use chrono::{Local, TimeZone};
    use chrono_tz::Tz;

    use rstest::rstest;
    #[cfg(feature = "serde")]
    use serde_test::{assert_de_tokens_error, assert_tokens, Token};
    #[test]
    fn test_is_time_matching() -> Result<(), CronError> {
        // This pattern is meant to match first second of 9 am on the first day of January.
        let cron = Cron::from_str("0 9 1 1 *")?;
        let time_matching = Local.with_ymd_and_hms(2023, 1, 1, 9, 0, 0).unwrap();
        let time_not_matching = Local.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);
        assert!(!cron.is_time_matching(&time_not_matching)?);

        Ok(())
    }

    #[test]
    fn test_last_day_of_february_non_leap_year() -> Result<(), CronError> {
        // This pattern is meant to match every second of 9 am on the last day of February in a non-leap year.
        let cron = Cron::from_str("0 9 L 2 *")?;

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
        let cron = Cron::from_str("0 9 L 2 *")?;

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
        let cron = Cron::from_str("0 0 * * FRI#L")?;

        // February 29th, 2024 is the last day of February in a leap year.
        let time_matching = Local.with_ymd_and_hms(2023, 12, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);

        Ok(())
    }

    #[test]
    fn test_last_friday_of_year_alternative_alpha_syntax() -> Result<(), CronError> {
        // This pattern is meant to match 0:00:00 last friday of current year
        let cron = Cron::from_str("0 0 * * FRIl")?;

        // February 29th, 2024 is the last day of February in a leap year.
        let time_matching = Local.with_ymd_and_hms(2023, 12, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);

        Ok(())
    }

    #[test]
    fn test_last_friday_of_year_alternative_number_syntax() -> Result<(), CronError> {
        // This pattern is meant to match 0:00:00 last friday of current year
        let cron = Cron::from_str("0 0 * * 5L")?;

        // February 29th, 2024 is the last day of February in a leap year.
        let time_matching = Local.with_ymd_and_hms(2023, 12, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&time_matching)?);

        Ok(())
    }

    #[test]
    fn test_find_next_occurrence() -> Result<(), CronError> {
        // This pattern is meant to match every minute at 30 seconds past the minute.
        let cron = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("* * * * * *")?;

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
        let cron = Cron::from_str("* * * * *")?;

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
        let cron = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("0 0 15 * * *")?;

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
        let schedule = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("0 0 0 * * 5,6")?;
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
        let schedule = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("59 59 23 2 * 6")?;
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
        let schedule = Cron::from_str("15 9 * * mon")?;
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
        let cron = Cron::from_str("0 0 13W * WED")?;

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
        for (idx, current_date) in cron
            .clone()
            .iter_from(start_date, Direction::Forward)
            .take(5)
            .enumerate()
        {
            assert_eq!(expected_dates[idx], current_date);
        }

        Ok(())
    }

    #[test]
    fn test_cron_expression_31dec_fri() -> Result<(), CronError> {
        // Parse the cron expression
        let cron = CronParser::builder()
            .seconds(Seconds::Required)
            .dom_and_dow(true)
            .build()
            .parse("0 0 0 31 12 FRI")?;

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
        for (idx, current_date) in cron
            .clone()
            .iter_from(start_date, Direction::Forward)
            .take(5)
            .enumerate()
        {
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
            "0 0 * * * * * *",
            "* * * *",
            "* 60 * * * *",
            "-1 59 * * * *",
            "1- 59 * * * *",
            "0 0 0 5L * *",
            "0 0 0 5#L * *",
        ];
        for expr in invalid_expressions {
            assert!(CronParser::builder()
                .seconds(Seconds::Optional)
                .build()
                .parse(expr)
                .is_err());
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
            assert!(Cron::from_str(expr).is_ok());
        }
    }

    #[test]
    fn test_is_time_matching_different_time_zones() -> Result<(), CronError> {
        use chrono::FixedOffset;

        let cron = Cron::from_str("0 12 * * *")?;
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
        let cron = CronParser::builder()
            .seconds(Seconds::Required)
            .build()
            .parse("59 59 23 * * *")?;
        let start_time = Local.with_ymd_and_hms(2023, 3, 14, 23, 59, 59).unwrap();
        let next_occurrence = cron.find_next_occurrence(&start_time, true)?;
        let expected_time = Local.with_ymd_and_hms(2023, 3, 14, 23, 59, 59).unwrap();
        assert_eq!(next_occurrence, expected_time);
        Ok(())
    }

    #[test]
    fn test_find_next_occurrence_edge_case_exclusive() -> Result<(), CronError> {
        let cron = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("59 59 23 * * *")?;
        let start_time = Local.with_ymd_and_hms(2023, 3, 14, 23, 59, 59).unwrap();
        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;
        let expected_time = Local.with_ymd_and_hms(2023, 3, 15, 23, 59, 59).unwrap();
        assert_eq!(next_occurrence, expected_time);
        Ok(())
    }

    #[test]
    fn test_cron_iterator_large_time_jumps() -> Result<(), CronError> {
        let cron = Cron::from_str("0 0 * * *")?;
        let start_time = Local.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let mut iterator = cron.iter_after(start_time);
        let next_run = iterator.nth(365 * 5 + 1); // Jump 5 years ahead
        let expected_time = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(next_run, Some(expected_time));
        Ok(())
    }

    #[test]
    fn test_handling_different_month_lengths() -> Result<(), CronError> {
        let cron = Cron::from_str("0 0 L * *")?; // Last day of the month
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
        let cron = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("*/29 */13 * * * *")?;
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
        let cron = Cron::from_str("7/29 2/13 * * *")?;
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
        let cron = Cron::from_str("0 0 */31,1-7 */1 MON")?;

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
        for current_date in cron
            .iter_from(start_date, Direction::Forward)
            .take(expected_dates.len())
        {
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
        let cron = CronParser::builder()
            .seconds(Seconds::Optional) // Just to differ as much from the non dom-and-dow test
            .dom_and_dow(true)
            .build()
            .parse("0 0 */31,1-7 */1 MON")?;

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
        for current_date in cron
            .iter_from(start_date, Direction::Forward)
            .take(expected_dates.len())
        {
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
        let cron = CronParser::builder()
            .seconds(Seconds::Optional) // Just to differ as much from the non dom-and-dow test
            .dom_and_dow(true)
            .build()
            .parse("0 0 29 2-3 FRI")?;

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
        let cron = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("0 0 0 * * 7#2")?;

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
        let cron = Cron::from_str("15 */2 * 3,5 FRI")?;
        let matching_time = Local.with_ymd_and_hms(2023, 3, 3, 2, 15, 0).unwrap();
        let non_matching_time = Local.with_ymd_and_hms(2023, 3, 3, 3, 15, 0).unwrap();

        assert!(cron.is_time_matching(&matching_time)?);
        assert!(!cron.is_time_matching(&non_matching_time)?);

        Ok(())
    }

    #[test]
    fn test_month_weekday_edge_cases() -> Result<(), CronError> {
        let cron = Cron::from_str("0 0 * 2-3 SUN")?;

        let matching_time = Local.with_ymd_and_hms(2023, 2, 5, 0, 0, 0).unwrap();
        let non_matching_time = Local.with_ymd_and_hms(2023, 2, 5, 0, 0, 1).unwrap();

        assert!(cron.is_time_matching(&matching_time)?);
        assert!(!cron.is_time_matching(&non_matching_time)?);

        Ok(())
    }

    #[test]
    fn test_leap_year() -> Result<(), CronError> {
        let cron = Cron::from_str("0 0 29 2 *")?;
        let leap_year_matching = Local.with_ymd_and_hms(2024, 2, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&leap_year_matching)?);

        Ok(())
    }

    #[test]
    fn test_tabs_for_separator() -> Result<(), CronError> {
        let cron = Cron::from_str("0 0   29  2   *")?;
        let leap_year_matching = Local.with_ymd_and_hms(2024, 2, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&leap_year_matching)?);

        Ok(())
    }

    #[test]
    fn test_mixed_separators() -> Result<(), CronError> {
        let cron = Cron::from_str("0  0    29  2      *")?;
        let leap_year_matching = Local.with_ymd_and_hms(2024, 2, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&leap_year_matching)?);

        Ok(())
    }

    #[test]
    fn test_mixed_leading_separators() -> Result<(), CronError> {
        let cron = Cron::from_str("  0 0 29 2 *")?;
        let leap_year_matching = Local.with_ymd_and_hms(2024, 2, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&leap_year_matching)?);

        Ok(())
    }

    #[test]
    fn test_mixed_tailing_separators() -> Result<(), CronError> {
        let cron = Cron::from_str("0 0 29 2 *    ")?;
        let leap_year_matching = Local.with_ymd_and_hms(2024, 2, 29, 0, 0, 0).unwrap();

        assert!(cron.is_time_matching(&leap_year_matching)?);

        Ok(())
    }

    #[test]
    fn test_time_overflow() -> Result<(), CronError> {
        let cron_match = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("59 59 23 31 12 *")?;
        let cron_next = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("0 0 0 1 1 *")?;
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
        let cron = Cron::from_str("0 0 1 1 *")?;
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
    #[case("0 0 * * * *", "@hourly", true)]
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
    #[case("* * * ? * ?", "* * * * * *", true)]
    #[case("@monthly", "0 0 1 * *", true)]
    #[case("* * * * 1,3,5", "* * * * MON,WED,FRI", true)]
    #[case("* * * mar *", "* * * 3 *", true)]
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
        use crate::parser::Seconds;

        eprintln!("Parsing {pattern_1}");
        let cron_1 = Cron::from_str(pattern_1).unwrap_or_else(|err| {
            eprintln!(
                "Initial parse attempt failed ({err}). Trying again but with allowed seconds."
            );
            CronParser::builder()
                .seconds(Seconds::Required)
                .build()
                .parse(pattern_1)
                .unwrap()
        });

        eprintln!("Parsing {pattern_2}");
        let cron_2 = Cron::from_str(pattern_2).unwrap_or_else(|err| {
            eprintln!(
                "Initial parse attempt failed ({err}). Trying again but with allowed seconds."
            );
            CronParser::builder()
                .seconds(Seconds::Required)
                .build()
                .parse(pattern_2)
                .unwrap()
        });

        assert_eq!(
            cron_1 == cron_2,
            equal,
            "Equality relation between both patterns is not {equal}. {cron_1} != {cron_2}."
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

    /// KNOWN BUG: these patterns are technically identical but the current
    /// `PartialEq` implementation doesn't respect that.
    #[rstest]
    #[case("0 0 1-7 * 1", "0 0 * * 1#1")]
    #[case("0 0 8-14 * MON", "0 0 * * MON#2")]
    #[should_panic(expected = "Patterns are not equal")]
    fn failed_equality(#[case] pattern_1: &str, #[case] pattern_2: &str) {
        let cron_1 = Cron::from_str(pattern_1).unwrap();
        let cron_2 = Cron::from_str(pattern_2).unwrap();
        assert!(cron_1 == cron_2, "Patterns are not equal");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_tokens() {
        let cron = Cron::from_str("0 0 * * *").expect("should be valid pattern");
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
            let cron = Cron::from_str(shorthand).expect("should be valid pattern");
            assert_tokens(&cron.to_string(), &[Token::Str(expected)]);
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_invalid_serde_tokens() {
        assert_de_tokens_error::<Cron>(
            &[Token::Str("Invalid cron pattern")],
            "Invalid pattern: Pattern must have between 5 and 7 fields."
        );
    }

    #[test]
    fn test_find_previous_occurrence() -> Result<(), CronError> {
        let cron = Cron::from_str("* * * * *")?;
        let start_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 1, 30).unwrap();
        let prev_occurrence = cron.find_previous_occurrence(&start_time, false)?;
        let expected_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 1, 0).unwrap();
        assert_eq!(prev_occurrence, expected_time);
        Ok(())
    }

    #[test]
    fn test_find_previous_occurrence_inclusive() -> Result<(), CronError> {
        let cron = Cron::from_str("* * * * *")?;
        let start_time = Local.with_ymd_and_hms(2023, 1, 1, 0, 1, 0).unwrap();
        let prev_occurrence = cron.find_previous_occurrence(&start_time, true)?;
        assert_eq!(prev_occurrence, start_time);
        Ok(())
    }

    #[test]
    fn test_wrap_year_backwards() -> Result<(), CronError> {
        let cron = Cron::from_str("0 0 1 1 *")?; // Jan 1st, 00:00
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

    #[test]
    fn test_find_occurrence_at_min_year_limit() -> Result<(), CronError> {
        // This pattern matches at midnight on January 1st every year.
        let cron = Cron::from_str("0 0 1 1 *")?;

        // Start the search just after midnight on the first day of the minimum allowed year.
        let start_time = Local
            .with_ymd_and_hms(YEAR_LOWER_LIMIT, 1, 1, 0, 0, 1)
            .unwrap();

        // Find the previous occurrence, which should be exactly at the start of the minimum year.
        let prev_occurrence = cron.find_previous_occurrence(&start_time, false)?;
        let expected_time = Local
            .with_ymd_and_hms(YEAR_LOWER_LIMIT, 1, 1, 0, 0, 0)
            .unwrap();
        assert_eq!(prev_occurrence, expected_time);

        // Searching past the limit will return TimeSearchLimitExceeded.
        let result = cron.find_previous_occurrence(&expected_time, false);
        assert!(matches!(result, Err(CronError::TimeSearchLimitExceeded)));

        Ok(())
    }

    #[test]
    fn test_find_occurrence_at_max_year_limit() -> Result<(), CronError> {
        // This pattern matches at midnight on January 1st every year.
        let cron = Cron::from_str("0 0 1 1 *")?;

        // Start the search late in the year just before the upper limit.
        let start_time = Local
            .with_ymd_and_hms(YEAR_UPPER_LIMIT - 1, 12, 31, 23, 59, 59)
            .unwrap();

        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;
        let expected_time = Local
            .with_ymd_and_hms(YEAR_UPPER_LIMIT, 1, 1, 0, 0, 0)
            .unwrap();
        assert_eq!(next_occurrence, expected_time);

        // Any search beyond the maximum year limit should fail.
        let result = cron.find_next_occurrence(&expected_time, false);
        assert!(matches!(result, Err(CronError::TimeSearchLimitExceeded)));

        Ok(())
    }

    #[test]
    fn test_weekday_for_historical_date_1831() -> Result<(), CronError> {
        // This pattern should match at midnight every Sunday.
        let cron = Cron::from_str("0 0 * * SUN")?;

        // June 5, 1831 was a Sunday.
        let matching_sunday = Local.with_ymd_and_hms(1831, 6, 5, 0, 0, 0).unwrap();

        // June 6, 1831 was a Monday.
        let non_matching_monday = Local.with_ymd_and_hms(1831, 6, 6, 0, 0, 0).unwrap();

        // Verify that the Sunday matches and the Monday does not.
        assert!(
            cron.is_time_matching(&matching_sunday)?,
            "Should match on Sunday, June 5, 1831"
        );
        assert!(
            !cron.is_time_matching(&non_matching_monday)?,
            "Should not match on Monday, June 6, 1831"
        );

        Ok(())
    }


    #[test]
    fn test_find_next_occurrence_with_year_range_outside_start() {
        let cron = Cron::from_str("0 0 0 1 1 * 2080-2085").unwrap();
        
        let start_time = Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        let next_occurrence = cron.find_next_occurrence(&start_time, false).unwrap();
        let expected_time = Local.with_ymd_and_hms(2080, 1, 1, 0, 0, 0).unwrap();
        
        assert_eq!(next_occurrence, expected_time, "Iterator should jump forward to the correct year.");
    }

    #[test]
    fn test_find_previous_occurrence_with_year_range_outside_start() {
        let cron = Cron::from_str("0 0 0 1 1 * 2030-2035").unwrap();

        let start_time = Local.with_ymd_and_hms(2050, 1, 1, 0, 0, 0).unwrap();

        let prev_occurrence = cron.find_previous_occurrence(&start_time, false).unwrap();
        let expected_time = Local.with_ymd_and_hms(2035, 1, 1, 0, 0, 0).unwrap();

        assert_eq!(prev_occurrence, expected_time, "Iteratorn should jump backwards to the correct year.");
    }

    // --- DST Gap (Spring Forward) Tests ---
    #[test]
    fn test_dst_gap_fixed_time_job() -> Result<(), CronError> {
        // Europe/Stockholm: 2025-03-30 02:00:00 (CET) -> 03:00:00 (CEST)
        // The hour 02:00-02:59:59 does not exist.
        let timezone: Tz = "Europe/Stockholm".parse().unwrap();

        // Fixed-Time Job: Scheduled for 02:30:00, which falls in the gap.
        // According to spec: Should execute at the first valid second/minute immediately following the gap (03:00:00).
        let cron = Cron::from_str("0 30 2 * * *")?; // 02:30:00
        let start_time = timezone.with_ymd_and_hms(2025, 3, 30, 1, 59, 59).unwrap(); // Just before the gap

        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;
        
        // The hour 02:00-02:59:59 does not exist.
        // According to spec: Should execute at the first valid second/minute immediately following the gap (03:00:00).
        let expected_time = timezone.with_ymd_and_hms(2025, 3, 30, 3, 0, 0).unwrap();
        assert_eq!(next_occurrence, expected_time, "Fixed-time job in DST gap should execute on the next valid occurrence of its pattern.");
        Ok(())
    }

    #[test]
    fn test_dst_gap_interval_wildcard_job_minute() -> Result<(), CronError> {
        // Europe/Stockholm: 2025-03-30 02:00:00 (CET) -> 03:00:00 (CEST)
        // The hour 02:00-02:59:59 does not exist.
        let timezone: Tz = "Europe/Stockholm".parse().unwrap();

        // Interval/Wildcard Job: Every 5 minutes, scheduled at 02:05, 02:10, etc.
        // These should be skipped. Next run should be relative to new wall clock time.
        let cron = Cron::from_str("0 */5 * * * *")?; // Every 5 minutes
        let start_time = timezone.with_ymd_and_hms(2025, 3, 30, 1, 59, 59).unwrap(); // Just before the gap

        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;
        // After 01:59:59, clock jumps to 03:00:00.
        // The next 5-minute interval after 03:00:00 is 03:00:00 itself (03:00 is a multiple of 5).
        let expected_time = timezone.with_ymd_and_hms(2025, 3, 30, 3, 0, 0).unwrap();

        assert_eq!(next_occurrence, expected_time, "Interval job in DST gap should skip the gap and resume relative to new wall time.");
        Ok(())
    }

    #[test]
    fn test_dst_gap_interval_wildcard_job_second() -> Result<(), CronError> {
        // Europe/Stockholm: 2025-03-30 02:00:00 (CET) -> 03:00:00 (CEST)
        // The hour 02:00-02:59:59 does not exist.
        let timezone: Tz = "Europe/Stockholm".parse().unwrap();

        // Interval/Wildcard Job: Every second
        let cron = Cron::from_str("* * * * * *")?; // Every second
        let start_time = timezone.with_ymd_and_hms(2025, 3, 30, 1, 59, 59).unwrap(); // Just before the gap

        let next_occurrence = cron.find_next_occurrence(&start_time, false)?;
        // After 01:59:59, clock jumps to 03:00:00.
        // The next second is 03:00:00.
        let expected_time = timezone.with_ymd_and_hms(2025, 3, 30, 3, 0, 0).unwrap();

        assert_eq!(next_occurrence, expected_time, "Every second job in DST gap should jump to the first valid second after the gap.");
        Ok(())
    }

    // --- DST Overlap (Fall Back) Tests ---

    #[test]
    fn test_dst_overlap_fixed_time_job() -> Result<(), CronError> {
        // Europe/Stockholm: 2025-10-26 03:00:00 (CEST) -> 02:00:00 (CET)
        // The hour 02:00-02:59:59 occurs twice.
        // First occurrence: 02:00:00-02:59:59 CEST
        // Second occurrence: 02:00:00-02:59:59 CET (after fallback from 03:00 CEST)
        let timezone: Tz = "Europe/Stockholm".parse().unwrap();

        // Fixed-Time Job: Scheduled for 02:30:00.
        // Should execute only once, at its first occurrence (CEST).
        let cron = Cron::from_str("0 30 2 * * *")?; // 02:30:00
        let start_time = timezone.with_ymd_and_hms(2025, 10, 26, 1, 59, 59).unwrap(); // Just before the repeated hour

        // First expected run: 02:30:00 CEST
        let first_occurrence = cron.find_next_occurrence(&start_time, false)?;
        let expected_first_time = timezone.with_ymd_and_hms(2025, 10, 26, 2, 30, 0).earliest().unwrap(); // This is 02:30 CEST
        assert_eq!(first_occurrence, expected_first_time, "Fixed-time job in DST overlap should run at first occurrence.");

        // Check that it does NOT run again for the second occurrence of 02:30:00 (CET)
        // Start search just after the first occurrence of 02:30:00 CEST.
        // The naive_time 02:30:00 is ambiguous, so after `first_occurrence`, the next naive_time is 02:30:01.
        // We need to advance past the entire ambiguous period.
        let _next_search_start = timezone.with_ymd_and_hms(2025, 10, 26, 2, 59, 59).earliest().unwrap(); // End of first 2am hour (CEST)
        let next_search_start_after_overlap = timezone.with_ymd_and_hms(2025, 10, 26, 3, 0, 0).unwrap(); // Start of the *second* 2am hour (CET)
        
        // Find the next occurrence after the *entire* ambiguous period.
        // The next 02:30:00 will be on the next day.
        let next_occurrence_after_overlap = cron.find_next_occurrence(&next_search_start_after_overlap, false)?;
        let expected_next_day = timezone.with_ymd_and_hms(2025, 10, 27, 2, 30, 0).unwrap(); // Next day at 02:30 CET
        
        assert_eq!(next_occurrence_after_overlap, expected_next_day, "Fixed-time job should not re-run during the repeated hour.");
        Ok(())
    }

    #[test]
    fn test_dst_overlap_interval_wildcard_job() -> Result<(), CronError> {
        // Europe/Stockholm: 2025-10-26 03:00:00 (CEST) -> 02:00:00 (CET)
        // The hour 02:00-02:59:59 occurs twice.
        let timezone: Tz = "Europe/Stockholm".parse().unwrap();

        // Interval/Wildcard Job: Every minute
        // Should execute for both occurrences of each minute in the repeated hour.
        let cron = Cron::from_str("0 * * * * *")?; // Every minute at 0 seconds
        let start_time = timezone.with_ymd_and_hms(2025, 10, 26, 1, 59, 59).unwrap(); // Just before the repeated hour

        let mut occurrences = Vec::new();
        let mut iter = cron.iter_after(start_time);

        // Collect occurrences for the repeated hour (02:00:00 to 02:59:00 twice)
        // We expect two entries for each minute from 02:00 to 02:59.
        // The loop should find the 02:00:00 CEST, then 02:01:00 CEST... 02:59:00 CEST,
        // then 02:00:00 CET, then 02:01:00 CET... 02:59:00 CET.
        // So, 60 minutes * 2 occurrences = 120 entries.
        for _ in 0..120 {
            if let Some(time) = iter.next() {
                occurrences.push(time);
            } else {
                break;
            }
        }

        assert_eq!(occurrences.len(), 120, "Interval job in DST overlap should run for both occurrences of each minute.");

        // Verify occurrences for each minute
        for m in 0..60 { // m is u32
            let naive_time_m_00 = chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2025, 10, 26).unwrap(),
                chrono::NaiveTime::from_hms_opt(2, m, 0).unwrap(),
            );
            let ambiguous_m_00 = timezone.from_local_datetime(&naive_time_m_00);

            // Assert CEST occurrence (earliest)
            assert_eq!(
                occurrences[(2 * m) as usize], // <-- CAST TO usize HERE
                ambiguous_m_00.earliest().unwrap(),
                "Minute {m}: CEST occurrence mismatch"
            );

            // Assert CET occurrence (latest)
            assert_eq!(
                occurrences[(2 * m + 1) as usize], // <-- CAST TO usize HERE
                ambiguous_m_00.latest().unwrap(),
                "Minute {m}: CET occurrence mismatch"
            );
        }

        Ok(())
    }

    #[test]
    fn test_dst_overlap_interval_wildcard_job_hour_step() -> Result<(), CronError> {
        // Europe/Stockholm: 2025-10-26 03:00:00 (CEST) -> 02:00:00 (CET)
        // The hour 02:00-02:59:59 occurs twice.
        let timezone: Tz = "Europe/Stockholm".parse().unwrap();

        // Interval/Wildcard Job: Every 2 hours, at 0 minutes and 0 seconds
        let cron = Cron::from_str("0 0 */2 * * *")?; // Every 2 hours
        let start_time = timezone.with_ymd_and_hms(2025, 10, 26, 0, 0, 0).unwrap(); // Start at midnight

        let mut iter = cron.iter_from(start_time, Direction::Forward);

        // Expected sequence:
        // 00:00:00 (CEST)
        // 02:00:00 (CEST) - first occurrence of 2 AM
        // 02:00:00 (CET) - the second occurrence of 2 AM
        // 04:00:00 (CET) - next 2-hour interval after the second 2 AM

        let first_run = iter.next().unwrap(); // 00:00:00 CEST
        let second_run = iter.next().unwrap(); // 02:00:00 CEST
        let third_run = iter.next().unwrap();  // 02:00:00 CET (the second occurrence of 2 AM)
        let fourth_run = iter.next().unwrap(); // 04:00:00 CET (next 2-hour interval after the second 2 AM)

        let naive_time_2_00 = chrono::NaiveDateTime::new(chrono::NaiveDate::from_ymd_opt(2025, 10, 26).unwrap(), chrono::NaiveTime::from_hms_opt(2, 0, 0).unwrap());
        let ambiguous_2_00 = timezone.from_local_datetime(&naive_time_2_00);

        assert_eq!(first_run, timezone.with_ymd_and_hms(2025, 10, 26, 0, 0, 0).unwrap());
        assert_eq!(second_run, ambiguous_2_00.earliest().unwrap()); // First 2 AM (CEST)
        assert_eq!(third_run, ambiguous_2_00.latest().unwrap()); // Second 2 AM (CET)
        assert_eq!(fourth_run, timezone.with_ymd_and_hms(2025, 10, 26, 4, 0, 0).unwrap()); // 4 AM CET - this is not ambiguous, so earlier() is fine

        Ok(())
    }


}
