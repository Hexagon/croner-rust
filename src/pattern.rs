use crate::component::{
    CronComponent, ALL_BIT, LAST_BIT, NONE_BIT, NTH_1ST_BIT, NTH_2ND_BIT, NTH_3RD_BIT, NTH_4TH_BIT,
    NTH_5TH_BIT, NTH_ALL,
};
use crate::errors::CronError;
use chrono::{Datelike, Duration, NaiveDate, Weekday};

// This struct is used for representing and validating cron pattern strings.
// It supports parsing cron patterns with optional seconds field and provides functionality to check pattern matching against specific datetime.
#[derive(Debug, Clone)]
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

    dom_and_dow: bool, // Setting to alter how dom_and_dow is combined
}

// Implementation block for CronPattern struct, providing methods for creating and parsing cron pattern strings.
impl CronPattern {
    pub fn new(pattern: &str) -> Result<Self, CronError> {
        let mut cron_pattern = CronPattern {
            pattern: pattern.to_string(),
            seconds: CronComponent::new(0, 59, NONE_BIT),
            minutes: CronComponent::new(0, 59, NONE_BIT),
            hours: CronComponent::new(0, 23, NONE_BIT),
            days: CronComponent::new(1, 31, LAST_BIT), // Special bit LAST_BIT is available
            months: CronComponent::new(1, 12, NONE_BIT),
            days_of_week: CronComponent::new(0, 7, LAST_BIT | NTH_ALL), // Actually 0-7 in pattern, but 7 is converted to 0
            star_dom: false,
            star_dow: false,
            dom_and_dow: false,
        };
        cron_pattern.parse()?;
        Ok(cron_pattern)
    }

    // Parses the cron pattern string into its respective fields.
    // Handles optional seconds field, named shortcuts, and determines if 'L' flag is used for last day of the month.
    pub fn parse(&mut self) -> Result<(), CronError> {
        if self.pattern.trim().is_empty() {
            return Err(CronError::EmptyPattern);
        }

        // Replace any '?' with '*' in the cron pattern
        self.pattern = self.pattern.replace("?", "*");

        // Handle @nicknames
        if self.pattern.contains('@') {
            self.pattern = Self::handle_nicknames(&self.pattern).trim().to_string();
        }

        // Handle day-of-week and month aliases (MON... and JAN...)
        self.pattern = Self::replace_alpha_weekdays(&self.pattern)
            .trim()
            .to_string();
        self.pattern = Self::replace_alpha_months(&self.pattern).trim().to_string();

        // Check that the pattern contains 5 or 6 parts
        let mut parts: Vec<&str> = self.pattern.split_whitespace().collect();
        if parts.len() < 5 || parts.len() > 6 {
            return Err(CronError::InvalidPattern(String::from("Pattern must consist of five or six fields (minute, hour, day, month, day of week, and optional second).")));
        }

        // Default seconds to "0" if omitted
        if parts.len() == 5 {
            parts.insert(0, "0"); // prepend "0" if the seconds part is missing
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
        if self.days_of_week.is_bit_set(7, ALL_BIT)? {
            self.days_of_week.unset_bit(7, ALL_BIT)?;
            self.days_of_week.set_bit(0, ALL_BIT)?;
        }

        // Success!
        Ok(())
    }

    // Validates that the cron pattern only contains legal characters for each field.
    // - ? is replaced with * before parsing, so it does not need to be included
    pub fn throw_at_illegal_characters(&self, parts: &[&str]) -> Result<(), CronError> {
        let base_allowed_characters = [
            '*', '/', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ',', '-',
        ];
        let day_of_week_additional_characters = ['#'];
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
    fn handle_nicknames(pattern: &str) -> String {
        let clean_pattern = pattern.trim().to_lowercase();
        match clean_pattern.as_str() {
            "@yearly" | "@annually" => "0 0 1 1 *".to_string(),
            "@monthly" => "0 0 1 * *".to_string(),
            "@weekly" => "0 0 * * 0".to_string(),
            "@daily" => "0 0 * * *".to_string(),
            "@hourly" => "0 * * * *".to_string(),
            _ => pattern.to_string(),
        }
    }

    // Converts day-of-week nicknames into their equivalent standard cron pattern.
    fn replace_alpha_weekdays(pattern: &str) -> String {
        // Day-of-week nicknames to their numeric values.
        let nicknames = [
            ("-sun", "-7"), // Use 7 for upper range sunday
            ("sun", "0"),
            ("mon", "1"),
            ("tue", "2"),
            ("wed", "3"),
            ("thu", "4"),
            ("fri", "5"),
            ("sat", "6"),
        ];

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
        if self.days.is_feature_enabled(LAST_BIT) {
            let last_day = CronPattern::last_day_of_month(year, month)?;
            if day == last_day {
                day_matches = true;
            }
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
            {
                if CronPattern::is_nth_weekday_of_month(date, nth, date.weekday()) {
                    dow_matches = true;
                }
            }
        }

        // If the 'L' flag is used for the day of the week, check if it's the last one of the month
        if self
            .days_of_week
            .is_bit_set(date.weekday().num_days_from_sunday() as u8, LAST_BIT)?
        {
            let next_weekday = date + chrono::Duration::days(7);
            if next_weekday.month() != date.month() {
                // If adding 7 days changes the month, then it is the last occurrence of the day of the week
                dow_matches = true;
            }
        }

        // Check if the specific day of the week is set in the bitset
        // Note: In chrono, Sunday is 0, Monday is 1, and so on...
        let day_of_week = date.weekday().num_days_from_sunday() as u8; // Adjust as necessary for your bitset
        dow_matches = dow_matches || self.days_of_week.is_bit_set(day_of_week, ALL_BIT)?;

        // The day matches if it's set in the days bitset or the days of the week bitset
        if day_matches && self.star_dow {
            Ok(true)
        } else if dow_matches && self.star_dom {
            Ok(true)
        } else if !self.star_dom && !self.star_dow {
            if self.dom_and_dow == false {
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
}

impl ToString for CronPattern {
    fn to_string(&self) -> String {
        self.pattern.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_pattern_new() {
        let pattern = CronPattern::new("* */5 * * * *").unwrap();
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
        let pattern = CronPattern::new("* */5 * * * *").unwrap();
        assert_eq!(pattern.to_string(), "* */5 * * * *");
    }

    #[test]
    fn test_cron_pattern_short() {
        let pattern = CronPattern::new("5/5 * * * *").unwrap();
        assert_eq!(pattern.pattern, "5/5 * * * *");
        assert!(pattern.seconds.is_bit_set(0, ALL_BIT).unwrap());
        assert!(!pattern.seconds.is_bit_set(5, ALL_BIT).unwrap());
        assert!(pattern.minutes.is_bit_set(5, ALL_BIT).unwrap());
        assert!(!pattern.minutes.is_bit_set(0, ALL_BIT).unwrap());
    }

    #[test]
    fn test_cron_pattern_parse() {
        let mut pattern = CronPattern::new("*/15 1 1,15 1 1-5").unwrap();
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
        let mut pattern = CronPattern::new("  */15  1 1,15 1    1-5    ").unwrap();
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
        assert_eq!(CronPattern::handle_nicknames("@yearly"), "0 0 1 1 *");
        assert_eq!(CronPattern::handle_nicknames("@monthly"), "0 0 1 * *");
        assert_eq!(CronPattern::handle_nicknames("@weekly"), "0 0 * * 0");
        assert_eq!(CronPattern::handle_nicknames("@daily"), "0 0 * * *");
        assert_eq!(CronPattern::handle_nicknames("@hourly"), "0 * * * *");
    }

    #[test]
    fn test_month_nickname_range() {
        let mut pattern = CronPattern::new("0 0 * FEB-MAR *").unwrap();
        assert!(pattern.parse().is_ok());
        assert!(!pattern.months.is_bit_set(1, ALL_BIT).unwrap());
        assert!(pattern.months.is_bit_set(2, ALL_BIT).unwrap()); // February
        assert!(pattern.months.is_bit_set(3, ALL_BIT).unwrap()); // March
        assert!(!pattern.months.is_bit_set(4, ALL_BIT).unwrap());
    }

    #[test]
    fn test_weekday_range_sat_sun() {
        let mut pattern = CronPattern::new("0 0 * * SAT-SUN").unwrap();
        assert!(pattern.parse().is_ok());
        assert!(pattern.days_of_week.is_bit_set(0, ALL_BIT).unwrap()); // Sunday
        assert!(pattern.days_of_week.is_bit_set(6, ALL_BIT).unwrap()); // Saturday
    }
}
