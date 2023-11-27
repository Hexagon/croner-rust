pub mod scheduler;

mod component;
mod errors;
mod pattern;
mod threadpool;

use errors::CronError;
use pattern::{CronPattern, NO_MATCH};
use std::str::FromStr;

use chrono::{
    DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike,
};

pub struct CronIterator<Tz>
where
    Tz: TimeZone,
{
    cron: Cron,
    current_time: DateTime<Tz>,
}

impl<Tz> CronIterator<Tz>
where
    Tz: TimeZone,
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
{
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.cron.find_next_occurrence(&self.current_time, true) {
            Ok(next_time) => {
                // Check if we can add one second without overflow
                let next_time_clone = next_time.clone();
                if let Some(updated_time) = next_time.checked_add_signed(Duration::seconds(1)) {
                    self.current_time = updated_time;
                    Some(next_time_clone) // Return the next time
                } else {
                    // If we hit an overflow, stop the iteration
                    None
                }
            }
            Err(_) => None, // Stop the iteration if we cannot find the next occurrence
        }
    }
}

enum TimeComponent {
    Second = 1,
    Minute,
    Hour,
    Day,
    Month,
    Year,
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

pub fn to_naive<Tz: TimeZone>(
    time: &DateTime<Tz>, /*timezone: Tz*/
) -> Result<NaiveDateTime, CronError> {
    // ToDo: Support for alternative time zones
    // Assume `timezone` is of type `&Tz` and compatible with `time`.
    // let local_time = time.withtimezone(timezone);

    // Convert to NaiveDateTime
    // Ok(local_time.naive_local())
    Ok(time.naive_local())
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
    /// use chrono::Utc;
    ///
    /// // Parse cron expression
    /// let cron: Cron = "0 * * * * *".parse().expect("Couldn't parse cron string");
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
        let naive_time = to_naive(time)?;

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
    /// let cron: Cron = "0 18 * * * 5".parse().expect("Couldn't parse cron string");
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
        let mut naive_time = to_naive(start_time)?;
        let originaltimezone = start_time.timezone();

        if !inclusive {
            increment_time_component(&mut naive_time, TimeComponent::Second)?;
        }

        loop {
            if self.find_next_matching_month(&mut naive_time)? {
                continue;
            };
            if self.find_next_matching_day(&mut naive_time)? {
                continue;
            };
            if self.find_next_matching_hour(&mut naive_time)? {
                continue;
            };
            if self.find_next_matching_minute(&mut naive_time)? {
                continue;
            };
            if self.find_next_matching_second(&mut naive_time)? {
                continue;
            };

            // Convert back to utc
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
    /// let cron: Cron = "* * * * * *".parse().expect("Couldn't parse cron string");
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
    /// let cron: Cron = "* * * * * *".parse().expect("Couldn't parse cron string");
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
        let next_match = self.pattern.next_hour_match(current_time.hour()).unwrap();
        if next_match == NO_MATCH {
            increment_time_component(current_time, TimeComponent::Day)?;
            incremented = true;
        } else if next_match != current_time.hour() {
            incremented = true;
            set_time_component(current_time, TimeComponent::Hour, next_match)?;
        }
        Ok(incremented)
    }
    fn find_next_matching_minute(
        &self,
        current_time: &mut NaiveDateTime,
    ) -> Result<bool, CronError> {
        let mut incremented = false;
        let next_match = self
            .pattern
            .next_minute_match(current_time.minute())
            .unwrap();
        if next_match == NO_MATCH {
            increment_time_component(current_time, TimeComponent::Hour)?;
            incremented = true;
        } else if next_match != current_time.minute() {
            incremented = true;
            set_time_component(current_time, TimeComponent::Minute, next_match)?;
        }
        Ok(incremented)
    }
    fn find_next_matching_second(
        &self,
        current_time: &mut NaiveDateTime,
    ) -> Result<bool, CronError> {
        let mut incremented = false;
        let next_match = self
            .pattern
            .next_second_match(current_time.second())
            .unwrap();
        if next_match == NO_MATCH {
            increment_time_component(current_time, TimeComponent::Minute)?;
            incremented = true;
        } else {
            set_time_component(current_time, TimeComponent::Second, next_match)?;
        }
        Ok(incremented)
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
        assert_eq!(next_occurrence, expected_time);

        Ok(())
    }

    #[test]
    fn test_weekday_pattern_correct_weekdays() -> Result<(), CronError> {
        let schedule = Cron::parse("0 0 0 * * 5,6")?;
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
        let schedule = Cron::parse("59 59 23 2 * 6")?;
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
        let schedule = Cron::parse("15 9 * * mon")?;
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
}
