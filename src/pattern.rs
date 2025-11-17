use std::cmp::Ordering;
use std::hash::Hasher;

use crate::component::{
    CronComponent, ALL_BIT, CLOSEST_WEEKDAY_BIT, LAST_BIT, NONE_BIT, NTH_1ST_BIT, NTH_2ND_BIT,
    NTH_3RD_BIT, NTH_4TH_BIT, NTH_5TH_BIT, NTH_ALL,
};
use crate::errors::CronError;
use crate::{Direction, TimeComponent, YEAR_LOWER_LIMIT, YEAR_UPPER_LIMIT};
use chrono::{Datelike, Duration, NaiveDate, Weekday};

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

    // Determines the nth weekday of the month
    fn is_nth_weekday_of_month(date: chrono::NaiveDate, nth: u8, weekday: Weekday) -> bool {
        let mut count = 0;
        let mut current = date.with_day(1).unwrap();
        while current.month() == date.month() {
            if current.weekday() == weekday {
                count += 1;
                if count == nth {
                    return current.day() == date.day();
                }
            }
            current += chrono::Duration::days(1);
        }
        false
    }

    // Checks if a given year, month, and day match the day part of the cron pattern.
    pub fn day_match(&self, year: i32, month: u32, day: u32) -> Result<bool, CronError> {
        if day == 0 || day > 31 || month == 0 || month > 12 {
            return Err(CronError::InvalidDate);
        }

        let date = NaiveDate::from_ymd_opt(year, month, day).ok_or(CronError::InvalidDate)?;
        let mut day_matches = self.days.is_bit_set(day as u16, ALL_BIT)?; // Use u16
        let mut dow_matches = false;

        // Check for LW (last weekday) - both LAST_BIT and CLOSEST_WEEKDAY_BIT enabled
        // This must be checked BEFORE the plain LAST_BIT check to avoid matching both
        if !day_matches
            && self.days.is_feature_enabled(LAST_BIT)
            && self.days.is_feature_enabled(CLOSEST_WEEKDAY_BIT)
            && day == Self::last_weekday_of_month(year, month)?
        {
            day_matches = true;
        } else if !day_matches
            && self.days.is_feature_enabled(LAST_BIT)
            && !self.days.is_feature_enabled(CLOSEST_WEEKDAY_BIT)
            && day == Self::last_day_of_month(year, month)?
        {
            // Check for L (last day of month) - only if CLOSEST_WEEKDAY_BIT is not enabled
            day_matches = true;
        }

        if !day_matches && self.closest_weekday(year, month, day)? {
            day_matches = true;
        }

        for nth in 1..=5 {
            let nth_bit = match nth {
                1 => NTH_1ST_BIT,
                2 => NTH_2ND_BIT,
                3 => NTH_3RD_BIT,
                4 => NTH_4TH_BIT,
                5 => NTH_5TH_BIT,
                _ => continue,
            };
            if self
                .days_of_week
                .is_bit_set(date.weekday().num_days_from_sunday() as u16, nth_bit)? // Use u16
                && Self::is_nth_weekday_of_month(date, nth, date.weekday())
            {
                dow_matches = true;
                break;
            }
        }

        if !dow_matches
            && self
                .days_of_week
                .is_bit_set(date.weekday().num_days_from_sunday() as u16, LAST_BIT)? // Use u16
            && (date + chrono::Duration::days(7)).month() != date.month()
        {
            dow_matches = true;
        }

        dow_matches = dow_matches
            || self
                .days_of_week
                .is_bit_set(date.weekday().num_days_from_sunday() as u16, ALL_BIT)?; // Use u16

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

    // Helper function to find the last day of a given month
    fn last_day_of_month(year: i32, month: u32) -> Result<u32, CronError> {
        if !(1..=12).contains(&month) {
            return Err(CronError::InvalidDate);
        }
        let (y, m) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        Ok(NaiveDate::from_ymd_opt(y, m, 1)
            .unwrap()
            .pred_opt()
            .unwrap()
            .day())
    }

    // Helper function to find the last weekday (Mon-Fri) of a given month
    fn last_weekday_of_month(year: i32, month: u32) -> Result<u32, CronError> {
        let last_day = Self::last_day_of_month(year, month)?;
        let mut current_date =
            NaiveDate::from_ymd_opt(year, month, last_day).ok_or(CronError::InvalidDate)?;

        // Walk backwards from the last day until we find a weekday (Mon-Fri)
        while current_date.weekday().num_days_from_sunday() == 0
            || current_date.weekday().num_days_from_sunday() == 6
        {
            current_date = current_date.pred_opt().ok_or(CronError::InvalidDate)?;
        }

        Ok(current_date.day())
    }

    pub fn closest_weekday(&self, year: i32, month: u32, day: u32) -> Result<bool, CronError> {
        // Iterate through all possible days to see if any have the 'W' flag.
        for pattern_day_u16 in 1..=31 {
            if self.days.is_bit_set(pattern_day_u16, CLOSEST_WEEKDAY_BIT)? {
                // A 'W' day exists in the pattern. Check if it resolves to the function's date argument.
                let pattern_day = pattern_day_u16 as u32;

                // Ensure the 'W' day is a valid calendar date for the given month/year.
                if let Some(pattern_date) = NaiveDate::from_ymd_opt(year, month, pattern_day) {
                    let weekday = pattern_date.weekday();

                    // Determine the actual trigger date based on the 'W' rule.
                    let target_date = match weekday {
                        // If the pattern day is a weekday, it triggers on that day.
                        Weekday::Mon
                        | Weekday::Tue
                        | Weekday::Wed
                        | Weekday::Thu
                        | Weekday::Fri => pattern_date,
                        // If it's a Saturday, find the nearest weekday within the month.
                        Weekday::Sat => {
                            // The nearest weekday is Friday, but check if it's in the same month.
                            let adjusted_date = pattern_date - Duration::days(1);
                            if adjusted_date.month() == month {
                                adjusted_date // It's Friday of the same month.
                            } else {
                                // Crossed boundary (e.g., 1st was Sat), so move forward to Monday.
                                pattern_date + Duration::days(2)
                            }
                        }
                        // If it's a Sunday, find the nearest weekday within the month.
                        Weekday::Sun => {
                            // The nearest weekday is Monday, but check if it's in the same month.
                            let adjusted_date = pattern_date + Duration::days(1);
                            if adjusted_date.month() == month {
                                adjusted_date // It's Monday of the same month.
                            } else {
                                // Crossed boundary (e.g., 31st was Sun), so move back to Friday.
                                pattern_date - Duration::days(2)
                            }
                        }
                    };

                    // Check if the calculated target day is the day we're currently testing.
                    if target_date.day() == day && target_date.month() == month {
                        return Ok(true);
                    }
                }
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
    use crate::parser::{CronParser, Seconds};

    use super::*;

    #[test]
    fn test_last_day_of_month() -> Result<(), CronError> {
        // Check the last day of February for a non-leap year
        assert_eq!(CronPattern::last_day_of_month(2021, 2)?, 28);

        // Check the last day of February for a leap year
        assert_eq!(CronPattern::last_day_of_month(2020, 2)?, 29);

        // Check for an invalid month (0 or greater than 12)
        assert!(CronPattern::last_day_of_month(2023, 0).is_err());
        assert!(CronPattern::last_day_of_month(2023, 13).is_err());

        Ok(())
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
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).expect("To work"); // 15th June 2023
        assert!(cron
            .pattern
            .day_match(date.year(), date.month(), date.day())?);

        // Test a month where the 15th is a Saturday
        // The closest weekday would be Friday, 14th.
        let date = NaiveDate::from_ymd_opt(2024, 6, 14).expect("To work"); // 14th May 2023
        assert!(cron
            .pattern
            .day_match(date.year(), date.month(), date.day())?);

        // Test a month where the 15th is a Sunday
        // The closest weekday would be Monday, 16th.
        let date = NaiveDate::from_ymd_opt(2023, 10, 16).expect("To work"); // 16th October 2023
        assert!(cron
            .pattern
            .day_match(date.year(), date.month(), date.day())?);

        // Test a non-matching date
        let date = NaiveDate::from_ymd_opt(2023, 6, 16).expect("To work"); // 16th June 2023
        assert!(!cron
            .pattern
            .day_match(date.year(), date.month(), date.day())?);

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
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).expect("To work"); // 15th June 2023
        assert!(cron
            .pattern
            .day_match(date.year(), date.month(), date.day())?);

        // Test a month where the 15th is a Saturday
        // The closest weekday would be Friday, 14th.
        let date = NaiveDate::from_ymd_opt(2024, 6, 14).expect("To work"); // 14th May 2023
        assert!(cron
            .pattern
            .day_match(date.year(), date.month(), date.day())?);

        // Test a month where the 15th is a Sunday
        // The closest weekday would be Monday, 16th.
        let date = NaiveDate::from_ymd_opt(2023, 10, 16).expect("To work"); // 16th October 2023
        assert!(cron
            .pattern
            .day_match(date.year(), date.month(), date.day())?);

        // Test a non-matching date
        let date = NaiveDate::from_ymd_opt(2023, 6, 16).expect("To work"); // 16th June 2023
        assert!(!cron
            .pattern
            .day_match(date.year(), date.month(), date.day())?);

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
            !cron.pattern.day_match(2025, 10, 31)?,
            "Should not trigger on previous month"
        );
        assert!(
            cron.pattern.day_match(2025, 11, 3)?,
            "Should trigger on Mon 3rd for Sat 1st"
        );
        assert!(
            !cron.pattern.day_match(2025, 11, 1)?,
            "Should not trigger on Sat 1st itself"
        );

        // Case 2: The 1st is a Sunday (June 2025).
        // Should trigger on Monday the 2nd.
        assert!(
            cron.pattern.day_match(2025, 6, 2)?,
            "Should trigger on Mon 2nd for Sun 1st"
        );
        assert!(
            !cron.pattern.day_match(2025, 6, 3)?,
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
            cron_end.pattern.day_match(2025, 8, 29)?,
            "Should trigger on Fri 29th for Sun 31st"
        );
        assert!(
            !cron_end.pattern.day_match(2025, 9, 1)?,
            "Should not trigger on next month"
        );

        Ok(())
    }
}
