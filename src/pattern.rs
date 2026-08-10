use std::cmp::Ordering;
use std::hash::Hasher;

use crate::component::{
    CronComponent, ALL_BIT, CLOSEST_WEEKDAY_BIT, LAST_BIT, NONE_BIT, NTH_1ST_BIT, NTH_2ND_BIT,
    NTH_3RD_BIT, NTH_4TH_BIT, NTH_5TH_BIT, NTH_ALL,
};
use crate::errors::CronError;
use crate::time::{days_in_month, Weekday};
use crate::{Direction, TimeComponent, YEAR_LOWER_LIMIT, YEAR_UPPER_LIMIT};

// This struct is used for representing and validating cron pattern strings.
#[derive(Debug, Clone, Eq)]
pub struct CronPattern {
    pub(crate) pattern: String, // The original pattern

    pub seconds: CronComponent,      // -
    pub minutes: CronComponent,      // --
    pub hours: CronComponent,        // --- Each individual part of the cron expression
    pub days: CronComponent,         // --- represented by a bitmask, min and max value
    pub months: CronComponent,       // ---
    pub days_of_week: CronComponent, // --
    pub years: CronComponent,        // -

    pub(crate) star_dom: bool,
    pub(crate) star_dow: bool,

    pub(crate) dom_and_dow: bool,
}

// Implementation block for CronPattern struct
impl CronPattern {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            seconds: CronComponent::new(0, 59, NONE_BIT, 0),
            minutes: CronComponent::new(0, 59, NONE_BIT, 0),
            hours: CronComponent::new(0, 23, NONE_BIT, 0),
            days: CronComponent::new(1, 31, LAST_BIT | CLOSEST_WEEKDAY_BIT, 0),
            months: CronComponent::new(1, 12, NONE_BIT, 0),
            days_of_week: CronComponent::new(0, 7, LAST_BIT | NTH_ALL, 0),
            years: CronComponent::new(
                YEAR_LOWER_LIMIT as u16,
                YEAR_UPPER_LIMIT as u16,
                NONE_BIT,
                0,
            ), // Use u16 for year range
            star_dom: false,
            star_dow: false,
            dom_and_dow: false,
        }
    }

    // Checks if a given year matches the year part of the cron pattern.
    pub fn year_match(&self, year: i32) -> Result<bool, CronError> {
        if !(YEAR_LOWER_LIMIT..=YEAR_UPPER_LIMIT).contains(&year) {
            // This case should ideally be prevented by search limits, but serves as a safeguard.
            return Ok(false);
        }
        self.years.is_bit_set(year as u16, ALL_BIT) // Use u16 cast
    }

    // Returns the bit for which occurrence of its own weekday a day is, or
    // `None` past the fifth. Days 1-7 hold the first occurrence of every
    // weekday, days 8-14 the second, and so on, so no calendar walk is needed.
    fn nth_weekday_bit(day: u32) -> Option<u8> {
        match (day - 1) / 7 {
            0 => Some(NTH_1ST_BIT),
            1 => Some(NTH_2ND_BIT),
            2 => Some(NTH_3RD_BIT),
            3 => Some(NTH_4TH_BIT),
            4 => Some(NTH_5TH_BIT),
            _ => None,
        }
    }

    // Checks if a given date matches the day part of the cron pattern.
    //
    // `weekday` must be the weekday of `day`. The caller already knows it, and
    // every other weekday this function needs belongs to the same month, so it
    // follows from a day offset rather than a calendar lookup.
    pub fn day_match(
        &self,
        year: i32,
        month: u32,
        day: u32,
        weekday: Weekday,
    ) -> Result<bool, CronError> {
        // The month has to be real before its length means anything, and the
        // day has to fall inside that length.
        if month == 0 || month > 12 {
            return Err(CronError::InvalidDate);
        }
        let month_length = days_in_month(year, month);
        if day == 0 || day > month_length {
            return Err(CronError::InvalidDate);
        }

        let weekday_bit = weekday.num_days_from_sunday() as u16;
        let mut day_matches = self.days.is_bit_set(day as u16, ALL_BIT)?; // Use u16
        let mut dow_matches = false;

        // Check for LW (last weekday) - both LAST_BIT and CLOSEST_WEEKDAY_BIT enabled
        // This must be checked BEFORE the plain LAST_BIT check to avoid matching both
        if !day_matches
            && self.days.is_feature_enabled(LAST_BIT)
            && self.days.is_feature_enabled(CLOSEST_WEEKDAY_BIT)
            && day
                == Self::last_weekday_of_month(
                    month_length,
                    weekday.shift((month_length - day) as i32),
                )
        {
            day_matches = true;
        } else if !day_matches
            && self.days.is_feature_enabled(LAST_BIT)
            && !self.days.is_feature_enabled(CLOSEST_WEEKDAY_BIT)
            && day == month_length
        {
            // Check for L (last day of month) - only if CLOSEST_WEEKDAY_BIT is not enabled
            day_matches = true;
        }

        if !day_matches && self.closest_weekday(month_length, day, weekday)? {
            day_matches = true;
        }

        // A day can only be one occurrence of its own weekday, so only that
        // one nth bit needs testing.
        if let Some(nth_bit) = Self::nth_weekday_bit(day) {
            dow_matches = self.days_of_week.is_bit_set(weekday_bit, nth_bit)?;
        }

        // The last occurrence of a weekday is the one with no room for another
        // seven days later in the same month.
        if !dow_matches
            && self.days_of_week.is_bit_set(weekday_bit, LAST_BIT)?
            && day + 7 > month_length
        {
            dow_matches = true;
        }

        dow_matches = dow_matches || self.days_of_week.is_bit_set(weekday_bit, ALL_BIT)?;

        if (day_matches && self.star_dow) || (dow_matches && self.star_dom) {
            Ok(true)
        } else if !self.star_dom && !self.star_dow {
            if !self.dom_and_dow {
                Ok(day_matches || dow_matches)
            } else {
                Ok(day_matches && dow_matches)
            }
        } else {
            Ok(false)
        }
    }

    // Helper function to find the last weekday (Mon-Fri) of a given month,
    // given that month's length and the weekday of its last day.
    fn last_weekday_of_month(month_length: u32, last_weekday: Weekday) -> u32 {
        // Walking back from the last day stops after at most two steps, so the
        // weekday of the last day decides the answer on its own.
        match last_weekday {
            Weekday::Saturday => month_length - 1,
            Weekday::Sunday => month_length - 2,
            _ => month_length,
        }
    }

    // Checks whether a 'W' day in the pattern resolves to `day`.
    //
    // `weekday` must be the weekday of `day`, and `month_length` the length of
    // the month both belong to.
    pub fn closest_weekday(
        &self,
        month_length: u32,
        day: u32,
        weekday: Weekday,
    ) -> Result<bool, CronError> {
        // The 'W' rule never moves a date by more than two days, so only
        // pattern days within two days of `day` can resolve to it. Clamping to
        // the month also drops the pattern days that month does not have.
        let first = day.saturating_sub(2).max(1);
        let last = (day + 2).min(month_length);
        for pattern_day in first..=last {
            if !self
                .days
                .is_bit_set(pattern_day as u16, CLOSEST_WEEKDAY_BIT)?
            {
                continue;
            }

            // Every candidate shares the month with `day`, so its weekday is a
            // plain offset and the 'W' rule resolves on day numbers alone. The
            // rule never leaves the month, and a month is at least 28 days, so
            // each branch below stays in range.
            let target_day = match weekday.shift(pattern_day as i32 - day as i32) {
                // If the pattern day is a weekday, it triggers on that day.
                Weekday::Monday
                | Weekday::Tuesday
                | Weekday::Wednesday
                | Weekday::Thursday
                | Weekday::Friday => pattern_day,
                // A Saturday moves back to Friday, or forward to Monday when
                // Friday would fall in the previous month.
                Weekday::Saturday if pattern_day > 1 => pattern_day - 1,
                Weekday::Saturday => pattern_day + 2,
                // A Sunday moves forward to Monday, or back to Friday when
                // Monday would fall in the next month.
                Weekday::Sunday if pattern_day < month_length => pattern_day + 1,
                Weekday::Sunday => pattern_day - 2,
            };

            // Check if the calculated target day is the day we're currently testing.
            if target_day == day {
                return Ok(true);
            }
        }

        // No 'W' pattern matched the current day.
        Ok(false)
    }

    pub fn month_match(&self, month: u32) -> Result<bool, CronError> {
        if !(1..=12).contains(&month) {
            return Err(CronError::InvalidDate);
        }
        self.months.is_bit_set(month as u16, ALL_BIT)
    }

    pub fn hour_match(&self, hour: u32) -> Result<bool, CronError> {
        if hour > 23 {
            return Err(CronError::InvalidTime);
        }
        self.hours.is_bit_set(hour as u16, ALL_BIT)
    }

    pub fn minute_match(&self, minute: u32) -> Result<bool, CronError> {
        if minute > 59 {
            return Err(CronError::InvalidTime);
        }
        self.minutes.is_bit_set(minute as u16, ALL_BIT)
    }

    pub fn second_match(&self, second: u32) -> Result<bool, CronError> {
        if second > 59 {
            return Err(CronError::InvalidTime);
        }
        self.seconds.is_bit_set(second as u16, ALL_BIT)
    }

    /// Finds the next or previous matching value for a given time component based on direction.
    pub fn find_match_in_component(
        &self,
        value: u32,
        component_type: TimeComponent,
        direction: Direction,
    ) -> Result<Option<u32>, CronError> {
        let component = match component_type {
            TimeComponent::Second => &self.seconds,
            TimeComponent::Minute => &self.minutes,
            TimeComponent::Hour => &self.hours,
            _ => {
                return Err(CronError::ComponentError(
                    "Invalid component type for match search".to_string(),
                ))
            }
        };

        let value_u16 = value as u16;
        if value_u16 > component.max {
            return Err(CronError::ComponentError(format!(
                "Input value {} is out of bounds for the component (max: {}).",
                value, component.max
            )));
        }

        match direction {
            Direction::Forward => {
                for next_value in value_u16..=component.max {
                    if component.is_bit_set(next_value, ALL_BIT)? {
                        return Ok(Some(next_value as u32));
                    }
                }
            }
            Direction::Backward => {
                for prev_value in (component.min..=value_u16).rev() {
                    if component.is_bit_set(prev_value, ALL_BIT)? {
                        return Ok(Some(prev_value as u32));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Returns a human-readable description of the cron pattern.
    ///
    /// This method provides a best-effort English description of the cron schedule.
    /// Note: The pattern must be parsed successfully before calling this method.
    /// Returns a human-readable description of the cron pattern in English.
    pub fn describe(&self) -> String {
        self.describe_lang(crate::describe::English)
    }

    /// Returns a human-readable description using a provided language provider.
    ///
    /// # Arguments
    ///
    /// * `lang` - An object that implements the `Language` trait.
    pub fn describe_lang<L: crate::describe::Language>(&self, lang: L) -> String {
        crate::describe::describe(self, &lang)
    }

    // Get a reference to the original pattern
    pub fn as_str(&self) -> &str {
        &self.pattern
    }
}

impl std::fmt::Display for CronPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pattern)
    }
}

impl PartialEq for CronPattern {
    /// Checks for functional equality between two CronPattern instances.
    ///
    /// Two patterns are considered equal if they have been parsed and their
    /// resulting schedule components and behavioral options are identical.
    /// The original pattern string is ignored in this comparison.
    ///
    /// Returns `false` if either pattern has not been parsed.
    fn eq(&self, other: &Self) -> bool {
        // Compare all components and boolean flags that define the schedule.
        self.seconds == other.seconds
            && self.minutes == other.minutes
            && self.hours == other.hours
            && self.days == other.days
            && self.months == other.months
            && self.days_of_week == other.days_of_week
            && self.years == other.years
            && self.star_dom == other.star_dom
            && self.star_dow == other.star_dow
            && self.dom_and_dow == other.dom_and_dow
    }
}

// To implement Ord, we must first implement PartialOrd.
// For types where comparison never fails, this is the standard way to do it.
impl PartialOrd for CronPattern {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// The primary implementation for Ord.
impl Ord for CronPattern {
    /// Implements the total ordering for `CronPattern`.
    ///
    /// This allows for consistent, deterministic sorting of cron patterns based on
    /// their functional schedule, not their string representation. The comparison
    /// is performed lexicographically on the parsed time components and behavioral flags.
    ///
    /// An unparsed pattern is always considered less than a parsed one.
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare the time components in logical order, from most to least
        // significant.
        self.seconds
            .cmp(&other.seconds)
            .then_with(|| self.minutes.cmp(&other.minutes))
            .then_with(|| self.hours.cmp(&other.hours))
            .then_with(|| self.days.cmp(&other.days))
            .then_with(|| self.months.cmp(&other.months))
            .then_with(|| self.days_of_week.cmp(&other.days_of_week))
            .then_with(|| self.years.cmp(&other.years))
            // Finally, compare the boolean flags to ensure a stable order
            // for patterns that are otherwise identical.
            .then_with(|| self.star_dom.cmp(&other.star_dom))
            .then_with(|| self.star_dow.cmp(&other.star_dow))
            .then_with(|| self.dom_and_dow.cmp(&other.dom_and_dow))
    }
}

impl std::hash::Hash for CronPattern {
    /// Hashes the functionally significant fields of the CronPattern.
    ///
    /// This implementation is consistent with the `PartialEq` implementation,
    /// ensuring that functionally identical patterns produce the same hash.
    /// The original pattern string is not included in the hash.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.seconds.hash(state);
        self.minutes.hash(state);
        self.hours.hash(state);
        self.days.hash(state);
        self.months.hash(state);
        self.days_of_week.hash(state);
        self.years.hash(state);
        self.star_dom.hash(state);
        self.star_dow.hash(state);
        self.dom_and_dow.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike as _, NaiveDate};

    use crate::parser::{CronParser, Seconds};

    use super::*;

    // Calls `day_match` with the weekday the caller would have supplied. The
    // weekday comes from chrono so that the test is an independent check on the
    // matcher's own day arithmetic.
    fn day_match(
        pattern: &CronPattern,
        year: i32,
        month: u32,
        day: u32,
    ) -> Result<bool, CronError> {
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("test date must exist");
        let weekday = Weekday::from_days_from_sunday(date.weekday().num_days_from_sunday());
        pattern.day_match(year, month, day, weekday)
    }

    #[test]
    fn test_closest_weekday() -> Result<(), CronError> {
        // Example cron pattern: "0 0 15W * *" which means at 00:00 on the closest weekday to the 15th of each month
        let cron = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("0 0 0 15W * *")?;

        // Test a month where the 15th is a weekday
        // Assuming 15th is Wednesday (a weekday), the closest weekday is the same day.
        assert!(day_match(&cron.pattern, 2023, 6, 15)?);

        // Test a month where the 15th is a Saturday
        // The closest weekday would be Friday, 14th.
        assert!(day_match(&cron.pattern, 2024, 6, 14)?);

        // Test a month where the 15th is a Sunday
        // The closest weekday would be Monday, 16th.
        assert!(day_match(&cron.pattern, 2023, 10, 16)?);

        // Test a non-matching date
        assert!(!day_match(&cron.pattern, 2023, 6, 16)?);

        Ok(())
    }

    #[test]
    fn test_closest_weekday_with_alternative_weekdays() -> Result<(), CronError> {
        // Example cron pattern: "0 0 15W * *" which means at 00:00 on the closest weekday to the 15th of each month
        let cron = CronParser::builder()
            .seconds(Seconds::Required)
            .alternative_weekdays(true)
            .build()
            .parse("0 0 0 15W * *")?;

        // Test a month where the 15th is a weekday
        // Assuming 15th is Wednesday (a weekday), the closest weekday is the same day.
        assert!(day_match(&cron.pattern, 2023, 6, 15)?);

        // Test a month where the 15th is a Saturday
        // The closest weekday would be Friday, 14th.
        assert!(day_match(&cron.pattern, 2024, 6, 14)?);

        // Test a month where the 15th is a Sunday
        // The closest weekday would be Monday, 16th.
        assert!(day_match(&cron.pattern, 2023, 10, 16)?);

        // Test a non-matching date
        assert!(!day_match(&cron.pattern, 2023, 6, 16)?);

        Ok(())
    }

    #[test]
    fn test_closest_weekday_month_boundary() -> Result<(), CronError> {
        // --- TEST START OF MONTH ---
        let cron = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("0 0 0 1W * *")?;

        // Case 1: The 1st is a Saturday (Nov 2025).
        // Should trigger on Monday the 3rd, not jump back to October.
        assert!(
            !day_match(&cron.pattern, 2025, 10, 31)?,
            "Should not trigger on previous month"
        );
        assert!(
            day_match(&cron.pattern, 2025, 11, 3)?,
            "Should trigger on Mon 3rd for Sat 1st"
        );
        assert!(
            !day_match(&cron.pattern, 2025, 11, 1)?,
            "Should not trigger on Sat 1st itself"
        );

        // Case 2: The 1st is a Sunday (June 2025).
        // Should trigger on Monday the 2nd.
        assert!(
            day_match(&cron.pattern, 2025, 6, 2)?,
            "Should trigger on Mon 2nd for Sun 1st"
        );
        assert!(
            !day_match(&cron.pattern, 2025, 6, 3)?,
            "Should NOT trigger on Tue 3rd for Sun 1st"
        );

        // --- TEST END OF MONTH ---
        let cron_end = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("0 0 0 31W * *")?;

        // Case 3: The 31st is a Sunday (Aug 2025).
        // Should trigger on Friday the 29th, not jump forward to September.
        assert!(
            day_match(&cron_end.pattern, 2025, 8, 29)?,
            "Should trigger on Fri 29th for Sun 31st"
        );
        assert!(
            !day_match(&cron_end.pattern, 2025, 9, 1)?,
            "Should not trigger on next month"
        );

        Ok(())
    }

    #[test]
    fn day_match_rejects_dates_that_do_not_exist() -> Result<(), CronError> {
        let cron = CronParser::builder().build().parse("0 0 * * *")?;
        assert!(cron
            .pattern
            .day_match(2023, 2, 29, Weekday::Wednesday)
            .is_err());
        assert!(cron.pattern.day_match(2023, 0, 1, Weekday::Sunday).is_err());
        assert!(cron
            .pattern
            .day_match(2023, 13, 1, Weekday::Sunday)
            .is_err());
        assert!(cron.pattern.day_match(2023, 1, 0, Weekday::Sunday).is_err());
        assert!(cron
            .pattern
            .day_match(2023, 4, 31, Weekday::Sunday)
            .is_err());
        // The same day in a leap year does exist.
        assert!(day_match(&cron.pattern, 2024, 2, 29)?);
        Ok(())
    }
}
