pub mod pattern;

mod component;
mod errors;

use component::ALL_BIT;
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
                self.current_time = next_time
                    .clone()
                    .checked_add_signed(Duration::seconds(1)).unwrap();
                Some(next_time)
            }
            Err(_) => None, // Stop the iteration if we cannot find the next occurrence
        }
    }
}

// Scheduler module responsible for matching times against cron patterns
#[derive(Clone)]
pub struct Cron {
    pub pattern: CronPattern, // Parsed cron pattern
}
impl Cron {
    // Constructor-like function to create a new Cron with a pattern
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
    /// fn main() {
    ///     // Parse cron expression
    ///     let cron: Cron = "0 * * * * *".parse().expect("Couldn't parse cron string");
    /// 
    ///     // Compare to time now
    ///     let time = Local::now();
    ///     let matches_all = cron.is_time_matching(&time).unwrap();
    /// 
    ///     // Output results
    ///     println!("Time is: {}", time);
    ///     println!(
    ///         "Pattern \"{}\" does {} time {}",
    ///         cron.pattern.to_string(),
    ///         if matches_all { "match" } else { "not match" },
    ///         time
    ///     );
    /// }
    /// ``` 
    pub fn is_time_matching<Tz: TimeZone>(&self, time: &DateTime<Tz>) -> Result<bool, CronError> {
        let second_matches = self
            .pattern
            .seconds
            .is_bit_set(time.second() as u8, ALL_BIT)?;
        let minute_matches = self
            .pattern
            .minutes
            .is_bit_set(time.minute() as u8, ALL_BIT)?;
        let hour_matches = self.pattern.hours.is_bit_set(time.hour() as u8, ALL_BIT)?;
        let month_matches = self
            .pattern
            .months
            .is_bit_set(time.month() as u8, ALL_BIT)?;
        let day_of_month_matches = self
            .pattern
            .day_match(time.year(), time.month(), time.day())?;
        Ok(second_matches
            && minute_matches
            && hour_matches
            && day_of_month_matches
            && month_matches)
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
    /// fn main() {
    ///     // Parse cron expression
    ///     let cron: Cron = "0 18 * * * 5".parse().expect("Couldn't parse cron string");
    /// 
    ///     // Get next match
    ///     let time = Local::now();
    ///     let next = cron.find_next_occurrence(&time, false).unwrap();
    /// 
    ///     println!(
    ///         "Pattern \"{}\" will match next time at {}",
    ///         cron.pattern.to_string(),
    ///         next
    ///     );
    /// }
    /// ```
    pub fn find_next_occurrence<Tz: TimeZone>(
        &self,
        start_time: &DateTime<Tz>,
        inclusive: bool
    ) -> Result<DateTime<Tz>, CronError> {
        let mut current_time = start_time
            .clone()
            .with_nanosecond(0)
            .ok_or(CronError::InvalidTime)?;
        
        // Start at next second if inclusive flag is false
        if !inclusive {
            current_time = current_time
                .checked_add_signed(Duration::seconds(1))
                .ok_or(CronError::InvalidTime)?;
        }

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
                    current_time = tz
                        .with_ymd_and_hms(
                            current_time.year(),
                            current_time.month(),
                            current_time.day(),
                            0,
                            0,
                            0,
                        )
                        .unwrap();

                    // If the day changes to the first of the next month, start 'outer loop again
                    if current_time.day() == 1
                        || self.pattern.day_match(
                            current_time.year(),
                            current_time.month(),
                            current_time.day(),
                        )?
                    {
                        continue 'outer;
                    }
                }
            }

            // Check if the current hour matches the pattern
            if !self.pattern.hour_match(current_time.hour())? {
                current_time = current_time
                    .checked_add_signed(Duration::hours(1))
                    .and_then(|time| time.with_minute(0))
                    .and_then(|time| time.with_second(0))
                    .ok_or(CronError::InvalidDate)?;
                continue;
            }

            // Check if the current minute matches the pattern
            if !self.pattern.minute_match(current_time.minute())? {
                current_time = current_time
                    .checked_add_signed(Duration::minutes(1))
                    .and_then(|time| time.with_second(0))
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
    /// fn main() {
    ///     // Parse cron expression
    ///     let cron: Cron = "* * * * * *".parse().expect("Couldn't parse cron string");
    /// 
    ///     // Compare to time now
    ///     let time = Local::now();
    /// 
    ///     // Get next 5 matches using iter_from
    ///     println!("Finding matches of pattern '{}' starting from {}:", cron.pattern.to_string(), time);
    /// 
    ///     for time in cron.clone().iter_from(time).take(5) {
    ///         println!("{}", time);
    ///     }
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
    /// fn main() {
    ///     // Parse cron expression
    ///     let cron: Cron = "* * * * * *".parse().expect("Couldn't parse cron string");
    /// 
    ///     // Compare to time now
    ///     let time = Local::now();
    /// 
    ///     // Get next 5 matches using iter_from
    ///     println!("Finding matches of pattern '{}' starting from {}:", cron.pattern.to_string(), time);
    /// 
    ///     for time in cron.clone().iter_after(time).take(5) {
    ///         println!("{}", time);
    ///     }
    /// }    
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
