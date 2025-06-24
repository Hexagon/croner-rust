use std::cmp::Ordering;
use std::hash::Hasher;

use crate::component::{
    CronComponent, ALL_BIT, CLOSEST_WEEKDAY_BIT, LAST_BIT, NONE_BIT, NTH_1ST_BIT, NTH_2ND_BIT,
    NTH_3RD_BIT, NTH_4TH_BIT, NTH_5TH_BIT, NTH_ALL,
};
use crate::errors::CronError;
use chrono::{Datelike, Duration, NaiveDate, Weekday};

// This struct is used for representing and validating cron pattern strings.
// It supports parsing cron patterns with optional seconds field and provides functionality to check pattern matching against specific datetime.
#[derive(Debug, Clone, Eq)]
pub struct CronPattern {
    pattern: String, // The original pattern
    //
    pub seconds: CronComponent,      // -
    pub minutes: CronComponent,      // --
    pub hours: CronComponent,        // --- Each individual part of the cron expression
    pub days: CronComponent,         // --- represented by a bitmask, min and max value
    pub months: CronComponent,       // --
    pub days_of_week: CronComponent, // -

    star_dom: bool,
    star_dow: bool,

    // Options
    pub dom_and_dow: bool, // Setting to alter how dom_and_dow is combined
    pub with_seconds_optional: bool, // Setting to alter if seconds (6-part patterns) are allowed or not
    pub with_seconds_required: bool, // Setting to alter if seconds (6-part patterns) are required or not
    pub with_alternative_weekdays: bool, // Setting to alter if weekdays are offset by one or not

    // Status
    is_parsed: bool,
}

// Implementation block for CronPattern struct, providing methods for creating and parsing cron pattern strings.
impl CronPattern {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            seconds: CronComponent::new(0, 59, NONE_BIT, 0),
            minutes: CronComponent::new(0, 59, NONE_BIT, 0),
            hours: CronComponent::new(0, 23, NONE_BIT, 0),
            days: CronComponent::new(1, 31, LAST_BIT | CLOSEST_WEEKDAY_BIT, 0), // Special bit LAST_BIT is available
            months: CronComponent::new(1, 12, NONE_BIT, 0),
            days_of_week: CronComponent::new(0, 7, LAST_BIT | NTH_ALL, 0), // Actually 0-7 in pattern, 7 is converted to 0 in POSIX mode
            star_dom: false,
            star_dow: false,

            // Options
            dom_and_dow: false,
            with_seconds_optional: false,
            with_seconds_required: false,
            with_alternative_weekdays: false,

            // Status
            is_parsed: false,
        }
    }

    // Parses the cron pattern string into its respective fields.
    // Handles optional seconds field, named shortcuts, and determines if 'L' flag is used for last day of the month.
    pub fn parse(&mut self) -> Result<CronPattern, CronError> {
        if self.pattern.trim().is_empty() {
            return Err(CronError::EmptyPattern);
        }

        // Replace any '?' with '*' in the cron pattern
        self.pattern = self.pattern.replace('?', "*");

        // Handle @nicknames
        if self.pattern.contains('@') {
            self.pattern = Self::handle_nicknames(&self.pattern, self.with_seconds_required)
                .trim()
                .to_string();
        }

        // Handle day-of-week and month aliases (MON... and JAN...)
        self.pattern = Self::replace_alpha_weekdays(&self.pattern, self.with_alternative_weekdays)
            .trim()
            .to_string();
        self.pattern = Self::replace_alpha_months(&self.pattern).trim().to_string();

        // Check that the pattern contains 5 or 6 parts
        let mut parts: Vec<&str> = self.pattern.split_whitespace().collect();
        if parts.len() < 5 || parts.len() > 6 {
            return Err(CronError::InvalidPattern(String::from("Pattern must consist of five or six fields (minute, hour, day, month, day of week, and optional second).")));
        }

        // Error if there is five parts and seconds are required
        if parts.len() == 5 && self.with_seconds_required {
            return Err(CronError::InvalidPattern(String::from(
                "Pattern must consist of six fields, seconds can not be omitted.",
            )));
        }

        // Error if there is six parts and seconds are disallowed
        if parts.len() == 6 && !(self.with_seconds_optional || self.with_seconds_required) {
            return Err(CronError::InvalidPattern(String::from(
                "Pattern must consist of five fields, seconds are not allowed by configuration.",
            )));
        }

        // Default seconds to "0" if omitted
        if parts.len() == 5 {
            parts.insert(0, "0"); // prepend "0" if the seconds part is missing

            // Error it there is an extra part and seconds are not allowed
        }

        // Handle star-dom and star-dow
        self.star_dom = parts[3].trim() == "*";
        self.star_dow = parts[5].trim() == "*";

        // Parse the individual components
        self.seconds.parse(parts[0])?;
        self.minutes.parse(parts[1])?;
        self.hours.parse(parts[2])?;
        self.days.parse(parts[3])?;
        self.months.parse(parts[4])?;
        self.days_of_week.parse(parts[5])?;

        // Handle conversion of 7 to 0 for day_of_week if necessary
        // this has to be done last because range could be 6-7 (sat-sun)
        if !self.with_alternative_weekdays {
            for nth_bit in [
                ALL_BIT,
                NTH_1ST_BIT,
                NTH_2ND_BIT,
                NTH_3RD_BIT,
                NTH_4TH_BIT,
                NTH_5TH_BIT,
            ] {
                if self.days_of_week.is_bit_set(7, nth_bit)? {
                    self.days_of_week.unset_bit(7, nth_bit)?;
                    self.days_of_week.set_bit(0, nth_bit)?;
                }
            }
        }

        // Success!
        self.is_parsed = true;
        Ok(self.clone())
    }

    // Validates that the cron pattern only contains legal characters for each field.
    // - ? is replaced with * before parsing, so it does not need to be included
    pub fn throw_at_illegal_characters(&self, parts: &[&str]) -> Result<(), CronError> {
        let base_allowed_characters = [
            '*', '/', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ',', '-',
        ];
        let day_of_week_additional_characters = ['#', 'W'];
        let day_of_month_additional_characters = ['L'];

        for (i, part) in parts.iter().enumerate() {
            // Decide which set of allowed characters to use
            let allowed = match i {
                5 => [
                    base_allowed_characters.as_ref(),
                    day_of_week_additional_characters.as_ref(),
                ]
                .concat(),
                3 => [
                    base_allowed_characters.as_ref(),
                    day_of_month_additional_characters.as_ref(),
                ]
                .concat(),
                _ => base_allowed_characters.to_vec(),
            };

            for ch in part.chars() {
                if !allowed.contains(&ch) {
                    return Err(CronError::IllegalCharacters(String::from(
                        "CronPattern contains illegal characters.",
                    )));
                }
            }
        }

        Ok(())
    }

    // Converts named cron pattern shortcuts like '@daily' into their equivalent standard cron pattern.
    fn handle_nicknames(pattern: &str, with_seconds_required: bool) -> String {
        let pattern = pattern.trim();

        // Closure for case-insensitive comparison
        let eq_ignore_case = |a: &str, b: &str| a.eq_ignore_ascii_case(b);

        let base_pattern = match pattern {
            p if eq_ignore_case(p, "@yearly") || eq_ignore_case(p, "@annually") => "0 0 1 1 *",
            p if eq_ignore_case(p, "@monthly") => "0 0 1 * *",
            p if eq_ignore_case(p, "@weekly") => "0 0 * * 0",
            p if eq_ignore_case(p, "@daily") => "0 0 * * *",
            p if eq_ignore_case(p, "@hourly") => "0 * * * *",
            _ => pattern,
        };

        if with_seconds_required {
            format!("0 {}", base_pattern)
        } else {
            base_pattern.to_string()
        }
    }

    // Converts day-of-week nicknames into their equivalent standard cron pattern.
    fn replace_alpha_weekdays(pattern: &str, alternative_weekdays: bool) -> String {
        // Day-of-week nicknames to their numeric values.
        let nicknames = if !alternative_weekdays {
            [
                ("-sun", "-7"), // Use 7 for upper range sunday
                ("sun", "0"),
                ("mon", "1"),
                ("tue", "2"),
                ("wed", "3"),
                ("thu", "4"),
                ("fri", "5"),
                ("sat", "6"),
            ]
        } else {
            [
                ("-sun", "-1"),
                ("sun", "1"),
                ("mon", "2"),
                ("tue", "3"),
                ("wed", "4"),
                ("thu", "5"),
                ("fri", "6"),
                ("sat", "7"),
            ]
        };

        let mut replaced = pattern.trim().to_lowercase();

        // Replace nicknames with their numeric values
        for &(nickname, value) in &nicknames {
            replaced = replaced.replace(nickname, value);
        }

        replaced
    }

    // Converts month nicknames into their equivalent standard cron pattern.
    fn replace_alpha_months(pattern: &str) -> String {
        // Day-of-week nicknames to their numeric values.
        let nicknames = [
            ("jan", "1"),
            ("feb", "2"),
            ("mar", "3"),
            ("apr", "4"),
            ("may", "5"),
            ("jun", "6"),
            ("jul", "7"),
            ("aug", "8"),
            ("sep", "9"),
            ("oct", "10"),
            ("nov", "11"),
            ("dec", "12"),
        ];

        let mut replaced = pattern.trim().to_lowercase();

        // Replace nicknames with their numeric values
        for &(nickname, value) in &nicknames {
            replaced = replaced.replace(nickname, value);
        }

        replaced
    }

    // Additional method needed to determine the nth weekday of the month
    fn is_nth_weekday_of_month(date: chrono::NaiveDate, nth: u8, weekday: Weekday) -> bool {
        let mut count = 0;
        let mut current = date
            .with_day(1)
            .expect("Invalid date encountered while setting to first of the month");
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

    // This method checks if a given year, month, and day match the day part of the cron pattern.
    pub fn day_match(&self, year: i32, month: u32, day: u32) -> Result<bool, CronError> {
        // First, check if the day is within the valid range
        if day == 0 || day > 31 || month == 0 || month > 12 {
            return Err(CronError::InvalidDate);
        }

        // Convert year, month, and day into a date object to work with
        let date =
            chrono::NaiveDate::from_ymd_opt(year, month, day).ok_or(CronError::InvalidDate)?;

        let mut day_matches = self.days.is_bit_set(day as u8, ALL_BIT)?;
        let mut dow_matches = false;

        // If the 'L' flag is used, we need to check if the given day is the last day of the month
        if !day_matches && self.days.is_feature_enabled(LAST_BIT) {
            let last_day = CronPattern::last_day_of_month(year, month)?;
            if !day_matches && day == last_day {
                day_matches = true;
            }
        }

        // Make an extra check if any adjacent day is matching through the closest-weekday flag
        if !day_matches && self.closest_weekday(year, month, day)? {
            day_matches = true;
        }

        // Check for nth weekday of the month flags
        for nth in 1..=5 {
            let nth_bit = match nth {
                1 => NTH_1ST_BIT,
                2 => NTH_2ND_BIT,
                3 => NTH_3RD_BIT,
                4 => NTH_4TH_BIT,
                5 => NTH_5TH_BIT,
                _ => continue, // We have already validated that nth is between 1 and 5
            };
            if self
                .days_of_week
                .is_bit_set(date.weekday().num_days_from_sunday() as u8, nth_bit)?
                && CronPattern::is_nth_weekday_of_month(date, nth, date.weekday())
            {
                dow_matches = true;
                break;
            }
        }

        // If the 'L' flag is used for the day of the week, check if it's the last one of the month
        if !dow_matches
            && self
                .days_of_week
                .is_bit_set(date.weekday().num_days_from_sunday() as u8, LAST_BIT)?
        {
            let next_weekday = date + chrono::Duration::days(7);
            if !dow_matches && next_weekday.month() != date.month() {
                // If adding 7 days changes the month, then it is the last occurrence of the day of the week
                dow_matches = true;
            }
        }

        // Check if the specific day of the week is set in the bitset
        // Note: In chrono, Sunday is 0, Monday is 1, and so on...
        let day_of_week = date.weekday().num_days_from_sunday() as u8; // Adjust as necessary for your bitset
        dow_matches = dow_matches || self.days_of_week.is_bit_set(day_of_week, ALL_BIT)?;

        // The day matches if it's set in the days bitset or the days of the week bitset
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

    pub fn closest_weekday(&self, year: i32, month: u32, day: u32) -> Result<bool, CronError> {
        let candidate_date =
            NaiveDate::from_ymd_opt(year, month, day).ok_or(CronError::InvalidDate)?;
        let weekday = candidate_date.weekday();

        // Only check weekdays
        if weekday != Weekday::Sat && weekday != Weekday::Sun {
            // Check if the current day has the CLOSEST_WEEKDAY_BIT set
            if self.days.is_bit_set(day as u8, CLOSEST_WEEKDAY_BIT)? {
                return Ok(true);
            }

            // Check the previous and next days if the current day is a weekday
            let previous_day = candidate_date - Duration::days(1);
            let next_day = candidate_date + Duration::days(1);

            let check_previous = previous_day.weekday() == Weekday::Sun
                && self
                    .days
                    .is_bit_set(previous_day.day() as u8, CLOSEST_WEEKDAY_BIT)?;
            let check_next = next_day.weekday() == Weekday::Sat
                && self
                    .days
                    .is_bit_set(next_day.day() as u8, CLOSEST_WEEKDAY_BIT)?;
            if check_previous || check_next {
                return Ok(true);
            }
        }

        Ok(false)
    }

    // Checks if a given month matches the month part of the cron pattern.
    pub fn month_match(&self, month: u32) -> Result<bool, CronError> {
        if month == 0 || month > 12 {
            return Err(CronError::InvalidDate);
        }
        self.months.is_bit_set(month as u8, ALL_BIT)
    }

    // Checks if a given hour matches the hour part of the cron pattern.
    pub fn hour_match(&self, hour: u32) -> Result<bool, CronError> {
        if hour > 23 {
            return Err(CronError::InvalidTime);
        }
        self.hours.is_bit_set(hour as u8, ALL_BIT)
    }

    // Checks if a given minute matches the minute part of the cron pattern.
    pub fn minute_match(&self, minute: u32) -> Result<bool, CronError> {
        if minute > 59 {
            return Err(CronError::InvalidTime);
        }
        self.minutes.is_bit_set(minute as u8, ALL_BIT)
    }

    // Checks if a given second matches the second part of the cron pattern.
    pub fn second_match(&self, second: u32) -> Result<bool, CronError> {
        if second > 59 {
            return Err(CronError::InvalidTime);
        }
        self.seconds.is_bit_set(second as u8, ALL_BIT)
    }

    // Finds the next hour that matches the hour part of the cron pattern.
    pub fn next_hour_match(&self, hour: u32) -> Result<Option<u32>, CronError> {
        if hour > 23 {
            return Err(CronError::InvalidTime);
        }
        for next_hour in hour..=23 {
            if self.hours.is_bit_set(next_hour as u8, ALL_BIT)? {
                return Ok(Some(next_hour));
            }
        }
        Ok(None) // No match found within the current range
    }

    // Finds the next minute that matches the minute part of the cron pattern.
    pub fn next_minute_match(&self, minute: u32) -> Result<Option<u32>, CronError> {
        if minute > 59 {
            return Err(CronError::InvalidTime);
        }
        for next_minute in minute..=59 {
            if self.minutes.is_bit_set(next_minute as u8, ALL_BIT)? {
                return Ok(Some(next_minute));
            }
        }
        Ok(None) // No match found within the current range
    }

    // Finds the next second that matches the second part of the cron pattern.
    pub fn next_second_match(&self, second: u32) -> Result<Option<u32>, CronError> {
        if second > 59 {
            return Err(CronError::InvalidTime);
        }
        for next_second in second..=59 {
            if self.seconds.is_bit_set(next_second as u8, ALL_BIT)? {
                return Ok(Some(next_second));
            }
        }
        Ok(None) // No match found within the current range
    }

    // Method to set the dom_and_dow flag
    pub fn with_dom_and_dow(&mut self) -> &mut Self {
        self.dom_and_dow = true;
        self
    }

    // Method to set wether seconds should be allowed
    pub fn with_seconds_optional(&mut self) -> &mut Self {
        self.with_seconds_optional = true;
        self
    }

    // Method to set wether seconds should be allowed
    pub fn with_seconds_required(&mut self) -> &mut Self {
        self.with_seconds_required = true;
        self
    }

    // Method to set if weekdays should be offset by one (Quartz Scheduler style)
    pub fn with_alternative_weekdays(&mut self) -> &mut Self {
        self.with_alternative_weekdays = true;
        //  We need to recreate self.days_of_week
        self.days_of_week = CronComponent::new(0, 7, LAST_BIT | NTH_ALL, 1);
        self
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
        match (self.is_parsed, other.is_parsed) {
            // Compare all parsed components and boolean flags that define the schedule.
            // `self.pattern` is ignored.
            (true, true) => {
                self.seconds == other.seconds
                    && self.minutes == other.minutes
                    && self.hours == other.hours
                    && self.days == other.days
                    && self.months == other.months
                    && self.days_of_week == other.days_of_week
                    && self.star_dom == other.star_dom
                    && self.star_dow == other.star_dow
                    && self.dom_and_dow == other.dom_and_dow
                    && self.with_seconds_optional == other.with_seconds_optional
                    && self.with_seconds_required == other.with_seconds_required
                    && self.with_alternative_weekdays == other.with_alternative_weekdays
            }
            (false, false) => true,
            _ => false,
        }
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
        // First, compare by the `is_parsed` status.
        self.is_parsed
            .cmp(&other.is_parsed)
            // If both have the same parsed status, compare the time components
            // in logical order, from most to least significant.
            .then_with(|| self.seconds.cmp(&other.seconds))
            .then_with(|| self.minutes.cmp(&other.minutes))
            .then_with(|| self.hours.cmp(&other.hours))
            .then_with(|| self.days.cmp(&other.days))
            .then_with(|| self.months.cmp(&other.months))
            .then_with(|| self.days_of_week.cmp(&other.days_of_week))
            // Finally, compare the boolean flags to ensure a stable order
            // for patterns that are otherwise identical.
            .then_with(|| self.star_dom.cmp(&other.star_dom))
            .then_with(|| self.star_dow.cmp(&other.star_dow))
            .then_with(|| self.dom_and_dow.cmp(&other.dom_and_dow))
            .then_with(|| self.with_seconds_optional.cmp(&other.with_seconds_optional))
            .then_with(|| self.with_seconds_required.cmp(&other.with_seconds_required))
            .then_with(|| {
                self.with_alternative_weekdays
                    .cmp(&other.with_alternative_weekdays)
            })
    }
}
impl std::hash::Hash for CronPattern {
    /// Hashes the functionally significant fields of the CronPattern.
    ///
    /// This implementation is consistent with the `PartialEq` implementation,
    /// ensuring that functionally identical patterns produce the same hash.
    /// The original pattern string is not included in the hash.
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Only hash the fields that are used for equality checks.
        // Also include `is_parsed` to differentiate between parsed and unparsed states.
        self.is_parsed.hash(state);
        if self.is_parsed {
            self.seconds.hash(state);
            self.minutes.hash(state);
            self.hours.hash(state);
            self.days.hash(state);
            self.months.hash(state);
            self.days_of_week.hash(state);
            self.star_dom.hash(state);
            self.star_dow.hash(state);
            self.dom_and_dow.hash(state);
            self.with_seconds_optional.hash(state);
            self.with_seconds_required.hash(state);
            self.with_alternative_weekdays.hash(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_pattern_new() {
        let pattern = CronPattern::new("*/5 * * * *").parse().unwrap();
        assert_eq!(pattern.pattern, "*/5 * * * *");
        assert!(pattern.seconds.is_bit_set(0, ALL_BIT).unwrap());
        assert!(pattern.minutes.is_bit_set(5, ALL_BIT).unwrap());
    }

    #[test]
    fn test_cron_pattern_new_with_seconds_optional() {
        let pattern = CronPattern::new("* */5 * * * *")
            .with_seconds_optional()
            .parse()
            .expect("Success");
        assert_eq!(pattern.pattern, "* */5 * * * *");
        assert!(pattern.seconds.is_bit_set(5, ALL_BIT).unwrap());
    }

    #[test]
    fn test_cron_pattern_new_with_seconds_required() {
        let mut pattern = CronPattern::new("* */5 * * * *");
        pattern.with_seconds_optional();
        let result = pattern.parse();
        assert!(result.is_ok());
        assert_eq!(pattern.pattern, "* */5 * * * *");
        assert!(pattern.seconds.is_bit_set(5, ALL_BIT).unwrap());
    }

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
    fn test_cron_pattern_tostring() {
        let mut pattern = CronPattern::new("*/5 * * * *");
        let result = pattern.parse();
        assert!(result.is_ok());
        assert_eq!(pattern.to_string(), "*/5 * * * *");
    }

    #[test]
    fn test_cron_pattern_short() {
        let mut pattern = CronPattern::new("5/5 * * * *");
        let result = pattern.parse();
        assert!(result.is_ok());
        assert_eq!(pattern.pattern, "5/5 * * * *");
        assert!(pattern.seconds.is_bit_set(0, ALL_BIT).unwrap());
        assert!(!pattern.seconds.is_bit_set(5, ALL_BIT).unwrap());
        assert!(pattern.minutes.is_bit_set(5, ALL_BIT).unwrap());
        assert!(!pattern.minutes.is_bit_set(0, ALL_BIT).unwrap());
    }

    #[test]
    fn test_cron_pattern_parse() {
        let mut pattern = CronPattern::new("*/15 1 1,15 1 1-5");
        assert!(pattern.parse().is_ok());
        assert!(pattern.minutes.is_bit_set(0, ALL_BIT).unwrap());
        assert!(pattern.hours.is_bit_set(1, ALL_BIT).unwrap());
        assert!(
            pattern.days.is_bit_set(1, ALL_BIT).unwrap()
                && pattern.days.is_bit_set(15, ALL_BIT).unwrap()
        );
        assert!(
            pattern.months.is_bit_set(1, ALL_BIT).unwrap()
                && !pattern.months.is_bit_set(2, ALL_BIT).unwrap()
        );
        assert!(
            pattern.days_of_week.is_bit_set(1, ALL_BIT).unwrap()
                && pattern.days_of_week.is_bit_set(5, ALL_BIT).unwrap()
        );
    }

    #[test]
    fn test_cron_pattern_extra_whitespace() {
        let mut pattern = CronPattern::new("  */15  1 1,15 1    1-5    ");
        assert!(pattern.parse().is_ok());
        assert!(pattern.minutes.is_bit_set(0, ALL_BIT).unwrap());
        assert!(pattern.hours.is_bit_set(1, ALL_BIT).unwrap());
        assert!(
            pattern.days.is_bit_set(1, ALL_BIT).unwrap()
                && pattern.days.is_bit_set(15, ALL_BIT).unwrap()
        );
        assert!(
            pattern.months.is_bit_set(1, ALL_BIT).unwrap()
                && !pattern.months.is_bit_set(2, ALL_BIT).unwrap()
        );
        assert!(
            pattern.days_of_week.is_bit_set(1, ALL_BIT).unwrap()
                && pattern.days_of_week.is_bit_set(5, ALL_BIT).unwrap()
        );
    }

    #[test]
    fn test_cron_pattern_leading_zeros() {
        let mut pattern = CronPattern::new("  */15  01 01,15 01    01-05    ");
        assert!(pattern.parse().is_ok());
        assert!(pattern.minutes.is_bit_set(0, ALL_BIT).unwrap());
        assert!(pattern.hours.is_bit_set(1, ALL_BIT).unwrap());
        assert!(
            pattern.days.is_bit_set(1, ALL_BIT).unwrap()
                && pattern.days.is_bit_set(15, ALL_BIT).unwrap()
        );
        assert!(
            pattern.months.is_bit_set(1, ALL_BIT).unwrap()
                && !pattern.months.is_bit_set(2, ALL_BIT).unwrap()
        );
        assert!(
            pattern.days_of_week.is_bit_set(1, ALL_BIT).unwrap()
                && pattern.days_of_week.is_bit_set(5, ALL_BIT).unwrap()
        );
    }

    #[test]
    fn test_cron_pattern_handle_nicknames() {
        assert_eq!(CronPattern::handle_nicknames("@yearly", false), "0 0 1 1 *");
        assert_eq!(
            CronPattern::handle_nicknames("@monthly", false),
            "0 0 1 * *"
        );
        assert_eq!(CronPattern::handle_nicknames("@weekly", false), "0 0 * * 0");
        assert_eq!(CronPattern::handle_nicknames("@daily", false), "0 0 * * *");
        assert_eq!(CronPattern::handle_nicknames("@hourly", false), "0 * * * *");
    }

    #[test]
    fn test_cron_pattern_handle_nicknames_with_seconds_required() {
        assert_eq!(
            CronPattern::handle_nicknames("@yearly", true),
            "0 0 0 1 1 *"
        );
        assert_eq!(
            CronPattern::handle_nicknames("@monthly", true),
            "0 0 0 1 * *"
        );
        assert_eq!(
            CronPattern::handle_nicknames("@weekly", true),
            "0 0 0 * * 0"
        );
        assert_eq!(CronPattern::handle_nicknames("@daily", true), "0 0 0 * * *");
        assert_eq!(
            CronPattern::handle_nicknames("@hourly", true),
            "0 0 * * * *"
        );
    }

    #[test]
    fn test_month_nickname_range() {
        let mut pattern = CronPattern::new("0 0 * FEB-MAR *");
        assert!(pattern.parse().is_ok());
        assert!(!pattern.months.is_bit_set(1, ALL_BIT).unwrap());
        assert!(pattern.months.is_bit_set(2, ALL_BIT).unwrap()); // February
        assert!(pattern.months.is_bit_set(3, ALL_BIT).unwrap()); // March
        assert!(!pattern.months.is_bit_set(4, ALL_BIT).unwrap());
    }

    #[test]
    fn test_weekday_range_sat_sun() {
        let mut pattern = CronPattern::new("0 0 * * SAT-SUN");
        assert!(pattern.parse().is_ok());
        assert!(pattern.days_of_week.is_bit_set(0, ALL_BIT).unwrap()); // Sunday
        assert!(pattern.days_of_week.is_bit_set(6, ALL_BIT).unwrap()); // Saturday
    }

    #[test]
    fn test_closest_weekday() -> Result<(), CronError> {
        // Example cron pattern: "0 0 15W * *" which means at 00:00 on the closest weekday to the 15th of each month
        let mut pattern = CronPattern::new("0 0 0 15W * *");
        pattern.with_seconds_optional();
        assert!(pattern.parse().is_ok());

        // Test a month where the 15th is a weekday
        // Assuming 15th is Wednesday (a weekday), the closest weekday is the same day.
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).expect("To work"); // 15th June 2023
        assert!(pattern.day_match(date.year(), date.month(), date.day())?);

        // Test a month where the 15th is a Saturday
        // The closest weekday would be Friday, 14th.
        let date = NaiveDate::from_ymd_opt(2024, 6, 14).expect("To work"); // 14th May 2023
        assert!(pattern.day_match(date.year(), date.month(), date.day())?);

        // Test a month where the 15th is a Sunday
        // The closest weekday would be Monday, 16th.
        let date = NaiveDate::from_ymd_opt(2023, 10, 16).expect("To work"); // 16th October 2023
        assert!(pattern.day_match(date.year(), date.month(), date.day())?);

        // Test a non-matching date
        let date = NaiveDate::from_ymd_opt(2023, 6, 16).expect("To work"); // 16th June 2023
        assert!(!pattern.day_match(date.year(), date.month(), date.day())?);

        Ok(())
    }

    #[test]
    fn test_closest_weekday_with_alternative_weekdays() -> Result<(), CronError> {
        // Example cron pattern: "0 0 15W * *" which means at 00:00 on the closest weekday to the 15th of each month
        let mut pattern = CronPattern::new("0 0 0 15W * *");
        pattern.with_seconds_required();
        pattern.with_alternative_weekdays();
        assert!(pattern.parse().is_ok());

        // Test a month where the 15th is a weekday
        // Assuming 15th is Wednesday (a weekday), the closest weekday is the same day.
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).expect("To work"); // 15th June 2023
        assert!(pattern.day_match(date.year(), date.month(), date.day())?);

        // Test a month where the 15th is a Saturday
        // The closest weekday would be Friday, 14th.
        let date = NaiveDate::from_ymd_opt(2024, 6, 14).expect("To work"); // 14th May 2023
        assert!(pattern.day_match(date.year(), date.month(), date.day())?);

        // Test a month where the 15th is a Sunday
        // The closest weekday would be Monday, 16th.
        let date = NaiveDate::from_ymd_opt(2023, 10, 16).expect("To work"); // 16th October 2023
        assert!(pattern.day_match(date.year(), date.month(), date.day())?);

        // Test a non-matching date
        let date = NaiveDate::from_ymd_opt(2023, 6, 16).expect("To work"); // 16th June 2023
        assert!(!pattern.day_match(date.year(), date.month(), date.day())?);

        Ok(())
    }

    #[test]
    fn test_with_seconds_false() {
        // Test with a 6-part pattern when seconds are not allowed
        let mut pattern = CronPattern::new("* * * * * *");
        assert!(matches!(pattern.parse(), Err(CronError::InvalidPattern(_))));

        // Test with a 5-part pattern when seconds are not allowed
        let mut no_seconds_pattern = CronPattern::new("*/10 * * * *");

        assert!(no_seconds_pattern.parse().is_ok());

        assert_eq!(no_seconds_pattern.to_string(), "*/10 * * * *");

        // Ensure seconds are defaulted to 0 for a 5-part pattern
        assert!(no_seconds_pattern.seconds.is_bit_set(0, ALL_BIT).unwrap());
    }

    #[test]
    fn test_with_seconds_required() {
        // Test with a 5-part pattern when seconds are required
        let mut no_seconds_pattern = CronPattern::new("*/10 * * * *");
        no_seconds_pattern.with_seconds_required();

        assert!(matches!(
            no_seconds_pattern.parse(),
            Err(CronError::InvalidPattern(_))
        ));

        // Test with a 6-part pattern when seconds are required
        let mut pattern = CronPattern::new("* * * * * *");
        pattern.with_seconds_required();

        assert!(pattern.parse().is_ok());

        // Ensure the 6-part pattern retains seconds information
        // (This assertion depends on how your CronPattern is structured and how it stores seconds information)
        assert!(pattern.seconds.is_bit_set(0, ALL_BIT).unwrap());
    }

    #[test]
    fn test_with_alternative_weekdays() {
        // Test with alternative weekdays enabled
        let mut pattern = CronPattern::new("* * * * MON-FRI");
        pattern.with_alternative_weekdays();

        // Parsing should succeed
        assert!(pattern.parse().is_ok());

        // Ensure that the days of the week are offset correctly
        // Note: In this scenario, "MON-FRI" should be treated as "SUN-THU"
        assert!(pattern.days_of_week.is_bit_set(1, ALL_BIT).unwrap()); // Monday
        assert!(pattern.days_of_week.is_bit_set(5, ALL_BIT).unwrap()); // Friday
        assert!(!pattern.days_of_week.is_bit_set(6, ALL_BIT).unwrap()); // Saturday should not be set
    }

    #[test]
    fn test_with_alternative_weekdays_numeric() {
        // Test with alternative weekdays enabled
        let mut pattern = CronPattern::new("* * * * 2-6");
        pattern.with_alternative_weekdays();

        // Parsing should succeed
        assert!(pattern.parse().is_ok());

        // Ensure that the days of the week are offset correctly
        // Note: In this scenario, "MON-FRI" should be treated as "SUN-THU"
        assert!(pattern.days_of_week.is_bit_set(1, ALL_BIT).unwrap()); // Monday
        assert!(pattern.days_of_week.is_bit_set(5, ALL_BIT).unwrap()); // Friday
        assert!(!pattern.days_of_week.is_bit_set(6, ALL_BIT).unwrap()); // Saturday should not be set
    }

    #[test]
    fn test_seven_to_zero() {
        // Test with alternative weekdays enabled
        let mut pattern = CronPattern::new("* * * * 7");

        // Parsing should succeed
        assert!(pattern.parse().is_ok());

        // Ensure that the days of the week are offset correctly
        // Note: In this scenario, "MON-FRI" should be treated as "SUN-THU"
        assert!(pattern.days_of_week.is_bit_set(0, ALL_BIT).unwrap()); // Monday
    }

    #[test]
    fn test_one_is_monday_alternative() {
        // Test with alternative weekdays enabled
        let mut pattern = CronPattern::new("* * * * 1");
        pattern.with_alternative_weekdays();

        // Parsing should succeed
        assert!(pattern.parse().is_ok());

        // Ensure that the days of the week are offset correctly
        // Note: In this scenario, "MON-FRI" should be treated as "SUN-THU"
        assert!(pattern.days_of_week.is_bit_set(0, ALL_BIT).unwrap()); // Monday
    }

    #[test]
    fn test_zero_with_alternative_weekdays_fails() {
        // Test with alternative weekdays enabled
        let mut pattern = CronPattern::new("* * * * 0");
        pattern.with_alternative_weekdays();

        // Parsing should raise a ComponentError
        assert!(matches!(pattern.parse(), Err(CronError::ComponentError(_))));
    }
}
