//! Parser for Cron patterns.
//!
//! Croner uses [`CronParser`] to parse the cron expression. Invoking
//!
//! ```rust
//! # use std::str::FromStr as _;
//! #
//! # use croner::{Cron, parser::CronParser};
//! #
//! Cron::from_str("pattern");
//! ```
//!
//! is equivalent to
//!
//! ```rust
//! # use std::str::FromStr as _;
//! #
//! # use croner::{Cron, parser::CronParser};
//! #
//! CronParser::new().parse("pattern");
//! ```
//!
//! You can customise the parser by creating a parser builder using
//! [`CronParser::builder`]. So, for example, to parse cron patterns with
//! optional seconds do something like this:
//!
//! ```rust
//! use croner::parser::{CronParser, Seconds};
//!
//! // Configure the parser to allow seconds.
//! let parser = CronParser::builder().seconds(Seconds::Optional).build();
//!
//! let cron_with_seconds = parser
//!     .parse("*/10 * * * * *")
//!     .unwrap();
//! let cron_without_seconds = parser
//!     .parse("* * * * *")
//!     .unwrap();
//! ```

use derive_builder::Builder;
use strum::EnumIs;

use crate::{
    component::{
        CronComponent, ALL_BIT, CLOSEST_WEEKDAY_BIT, LAST_BIT, NONE_BIT, NTH_1ST_BIT, NTH_2ND_BIT,
        NTH_3RD_BIT, NTH_4TH_BIT, NTH_5TH_BIT, NTH_ALL,
    },
    errors::CronError,
    pattern::CronPattern,
    Cron, YEAR_LOWER_LIMIT, YEAR_UPPER_LIMIT,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, EnumIs)]
pub enum Seconds {
    #[default]
    Optional,
    Required,
    Disallowed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, EnumIs)]
pub enum Year {
    #[default]
    Optional,
    Required,
    Disallowed,
}

/// Parser for Cron patterns.
///
/// In order to build a custom cron parser use [`CronParser::builder`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Builder)]
#[builder(default, build_fn(skip), pattern = "owned")]
pub struct CronParser {
    /// Configure how seconds should be handled.
    seconds: Seconds,
    /// Configure how years should be handled.
    year: Year,
    /// Enable the combination of Day of Month (DOM) and Day of Week (DOW) conditions.
    dom_and_dow: bool,
    /// Use the Quartz-style weekday mode.
    alternative_weekdays: bool,
}

impl CronParser {
    /// Create a new parser.
    ///
    /// You should probably be using [`Cron`]'s implementation of
    /// [`FromStr`][std::str::FromStr] instead of invoking this.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a builder for custom parsing.
    ///
    /// Equivalent to [`CronParserBuilder::default`].
    pub fn builder() -> CronParserBuilder {
        CronParserBuilder::default()
    }

    /// Parses the cron pattern string.
    pub fn parse(&self, pattern: &str) -> Result<Cron, CronError> {
        // Ensure upper case in parsing, and trim it
        let mut pattern: String = pattern.to_uppercase().trim().to_string();

        // Should already be trimmed
        if pattern.is_empty() {
            return Err(CronError::EmptyPattern);
        }

        // Handle @nicknames
        if pattern.contains('@') {
            pattern = Self::handle_nicknames(&pattern, self.seconds.is_required(), self.year.is_required()).to_string();
        }

        // Handle day-of-week and month aliases (MON... and JAN...)
        pattern = Self::replace_alpha_weekdays(&pattern, self.alternative_weekdays).to_string();
        pattern = Self::replace_alpha_months(&pattern).to_string();

        // Split the pattern into parts
        let mut parts: Vec<&str> = pattern.split_whitespace().collect();
        let num_parts = parts.len();

        // Default seconds to "0" if omitted in an optional context
        if num_parts == 5 {
            parts.insert(0, "0");
        } else if self.seconds.is_disallowed() {
            return Err(CronError::InvalidPattern("Pattern must have 5 fields when seconds are disallowed.".to_string()));
        }
        
        // Default year to "*" if omitted in an optional context
        if parts.len() == 6 {
            parts.push("*");
        } else if self.year.is_disallowed()  {
            return Err(CronError::InvalidPattern("Pattern must have 5 or 6 fields when years are disallowed.".to_string()));
        }

        // Validate pattern length based on configuration
        if self.seconds.is_required() {
            if self.year.is_required() && num_parts != 7 {
                return Err(CronError::InvalidPattern("Pattern must have 7 fields when seconds and years are required.".to_string()));
            }
            if self.year.is_disallowed() && num_parts != 6 {
                return Err(CronError::InvalidPattern("Pattern must have 6 fields when seconds are required and years are disallowed.".to_string()));
            }
            if self.year.is_optional() && !(6..=7).contains(&num_parts) {
                 return Err(CronError::InvalidPattern("Pattern must have 6 or 7 fields when seconds are required and years are optional.".to_string()));
            }
        } else if self.year.is_required() && num_parts != 7 {
            return Err(CronError::InvalidPattern("Pattern must have 7 fields when years are required.".to_string()));
        } else if !(5..=7).contains(&num_parts) {
             return Err(CronError::InvalidPattern("Pattern must have between 5 and 7 fields.".to_string()));
        }

        // Replace ? with * in day-of-month and day-of-week
        let mut owned_parts = parts.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        if owned_parts.get(3).is_some_and(|p| p.contains('?')) {
            owned_parts[3] = owned_parts[3].replace('?', "*");
        }
        if owned_parts.get(5).is_some_and(|p| p.contains('?')) {
            owned_parts[5] = owned_parts[5].replace('?', "*");
        }
        parts = owned_parts.iter().map(|s| s.as_str()).collect();

        // Throw at illegal characters
        self.throw_at_illegal_characters(&parts)?;

        // Handle star-dom and star-dow
        let star_dom = parts.get(3).is_some_and(|&p| p == "*");
        let star_dow = parts.get(5).is_some_and(|&p| p == "*");

        // Parse the individual components
        let mut seconds = CronComponent::new(0, 59, NONE_BIT, 0);
        seconds.parse(parts[0])?;
        
        let mut minutes = CronComponent::new(0, 59, NONE_BIT, 0);
        minutes.parse(parts[1])?;

        let mut hours = CronComponent::new(0, 23, NONE_BIT, 0);
        hours.parse(parts[2])?;
        let mut days = CronComponent::new(1, 31, LAST_BIT | CLOSEST_WEEKDAY_BIT, 0);
        days.parse(parts[3])?;
        let mut months = CronComponent::new(1, 12, NONE_BIT, 0);
        months.parse(parts[4])?;
        
        let mut days_of_week = if self.alternative_weekdays {
            CronComponent::new(0, 7, LAST_BIT | NTH_ALL, 1)
        } else {
            CronComponent::new(0, 7, LAST_BIT | NTH_ALL, 0)
        };
        days_of_week.parse(parts[5])?;

        let mut years = CronComponent::new(YEAR_LOWER_LIMIT as u16, YEAR_UPPER_LIMIT as u16, NONE_BIT, 0); // Placeholder, real limits are i32
        years.parse(parts[6])?;

        // Handle conversion of 7 to 0 for day_of_week if necessary
        if !self.alternative_weekdays {
            for nth_bit in [
                ALL_BIT,
                NTH_1ST_BIT,
                NTH_2ND_BIT,
                NTH_3RD_BIT,
                NTH_4TH_BIT,
                NTH_5TH_BIT,
            ] {
                if days_of_week.is_bit_set(7, nth_bit)? {
                    days_of_week.unset_bit(7, nth_bit)?;
                    days_of_week.set_bit(0, nth_bit)?;
                }
            }
        }

        Ok(Cron {
            pattern: CronPattern {
                pattern,
                seconds,
                minutes,
                hours,
                days,
                months,
                days_of_week,
                years,
                star_dom,
                star_dow,
                dom_and_dow: self.dom_and_dow,
            },
        })
    }
    
    // Validates that the cron pattern only contains legal characters for each field.
    fn throw_at_illegal_characters(&self, parts: &[&str]) -> Result<(), CronError> {
        let base_allowed_characters = [
            '*', '/', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ',', '-',
        ];
        let day_of_week_additional_characters = ['#', 'L', '?'];
        let day_of_month_additional_characters = ['L', 'W', '?'];

        for (i, part) in parts.iter().enumerate() {
            // Decide which set of allowed characters to use
            let allowed = match i {
                3 => [base_allowed_characters.as_ref(), day_of_month_additional_characters.as_ref()].concat(),
                5 => [base_allowed_characters.as_ref(), day_of_week_additional_characters.as_ref()].concat(),
                // All other fields, including year (index 6) use base characters
                _ => base_allowed_characters.to_vec(),
            };

            for ch in part.chars() {
                if !allowed.contains(&ch) {
                    return Err(CronError::IllegalCharacters(format!(
                        "CronPattern contains illegal character '{ch}' in part '{part}'"
                    )));
                }
            }
        }
        Ok(())
    }

    // Converts named cron pattern shortcuts into their equivalent standard cron pattern.
    fn handle_nicknames(pattern: &str, with_seconds: bool, with_year: bool) -> String {
        let pattern = pattern.trim();
        let eq_ignore_case = |a: &str, b: &str| a.eq_ignore_ascii_case(b);

        let base_pattern = match pattern {
            p if eq_ignore_case(p, "@yearly") || eq_ignore_case(p, "@annually") => "0 0 1 1 *",
            p if eq_ignore_case(p, "@monthly") => "0 0 1 * *",
            p if eq_ignore_case(p, "@weekly") => "0 0 * * 0",
            p if eq_ignore_case(p, "@daily") => "0 0 * * *",
            p if eq_ignore_case(p, "@hourly") => "0 * * * *",
            _ => pattern,
        };
        
        let mut final_pattern = String::new();
        if with_seconds {
            final_pattern.push_str("0 ");
        }
        final_pattern.push_str(base_pattern);
        if with_year {
            final_pattern.push_str(" *");
        }

        final_pattern
    }


    // Converts day-of-week nicknames into their equivalent standard cron pattern.
    fn replace_alpha_weekdays(pattern: &str, alternative_weekdays: bool) -> String {
        let nicknames = if !alternative_weekdays {
            [
                ("-SUN", "-7"),
                ("SUN", "0"),
                ("MON", "1"),
                ("TUE", "2"),
                ("WED", "3"),
                ("THU", "4"),
                ("FRI", "5"),
                ("SAT", "6"),
            ]
        } else {
            [
                ("-SUN", "-1"),
                ("SUN", "1"),
                ("MON", "2"),
                ("TUE", "3"),
                ("WED", "4"),
                ("THU", "5"),
                ("FRI", "6"),
                ("SAT", "7"),
            ]
        };
        let mut replaced = pattern.to_string();

        // Replace nicknames with their numeric values
        for &(nickname, value) in &nicknames {
            replaced = replaced.replace(nickname, value);
        }

        replaced
    }

    // Converts month nicknames into their equivalent standard cron pattern.
    fn replace_alpha_months(pattern: &str) -> String {
        let nicknames = [
            ("JAN", "1"),
            ("FEB", "2"),
            ("MAR", "3"),
            ("APR", "4"),
            ("MAY", "5"),
            ("JUN", "6"),
            ("JUL", "7"),
            ("AUG", "8"),
            ("SEP", "9"),
            ("OCT", "10"),
            ("NOV", "11"),
            ("DEC", "12"),
        ];

        let mut replaced = pattern.to_string();

        // Replace nicknames with their numeric values
        for &(nickname, value) in &nicknames {
            replaced = replaced.replace(nickname, value);
        }

        replaced
    }
}

impl CronParserBuilder {
    pub fn build(self) -> CronParser {
        let CronParserBuilder {
            seconds,
            year,
            dom_and_dow,
            alternative_weekdays,
        } = self;
        CronParser {
            seconds: seconds.unwrap_or_default(),
            year: year.unwrap_or_default(),
            dom_and_dow: dom_and_dow.unwrap_or_default(),
            alternative_weekdays: alternative_weekdays.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;


    #[test]
    fn test_cron_pattern_new() {
        let cron = Cron::from_str("*/5 * * * *").unwrap();
        assert_eq!(cron.pattern.pattern, "*/5 * * * *");
        assert!(cron.pattern.seconds.is_bit_set(0, ALL_BIT).unwrap());
        assert!(cron.pattern.minutes.is_bit_set(5, ALL_BIT).unwrap());
    }

    #[test]
    fn test_cron_pattern_new_with_seconds_optional() {
        let cron = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("* */5 * * * *")
            .expect("Success");
        assert_eq!(cron.pattern.pattern, "* */5 * * * *");
        assert!(cron.pattern.seconds.is_bit_set(5, ALL_BIT).unwrap());
    }

    #[test]
    fn test_cron_pattern_new_with_seconds_required() {
        let cron = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse("* */5 * * * *")
            .unwrap();
        assert_eq!(cron.pattern.pattern, "* */5 * * * *");
        assert!(cron.pattern.seconds.is_bit_set(5, ALL_BIT).unwrap());
    }

    #[test]
    fn test_cron_pattern_tostring() {
        let cron = Cron::from_str("*/5 * * * *").unwrap();
        assert_eq!(cron.to_string(), "*/5 * * * *");
    }

    #[test]
    fn test_cron_pattern_short() {
        let cron = Cron::from_str("5/5 * * * *").unwrap();
        assert_eq!(cron.pattern.pattern, "5/5 * * * *");
        assert!(cron.pattern.seconds.is_bit_set(0, ALL_BIT).unwrap());
        assert!(!cron.pattern.seconds.is_bit_set(5, ALL_BIT).unwrap());
        assert!(cron.pattern.minutes.is_bit_set(5, ALL_BIT).unwrap());
        assert!(!cron.pattern.minutes.is_bit_set(0, ALL_BIT).unwrap());
    }

    #[test]
    fn test_cron_pattern_parse() {
        let cron = Cron::from_str("*/15 1 1,15 1 1-5").unwrap();
        assert!(cron.pattern.minutes.is_bit_set(0, ALL_BIT).unwrap());
        assert!(cron.pattern.hours.is_bit_set(1, ALL_BIT).unwrap());
        assert!(
            cron.pattern.days.is_bit_set(1, ALL_BIT).unwrap()
                && cron.pattern.days.is_bit_set(15, ALL_BIT).unwrap()
        );
        assert!(
            cron.pattern.months.is_bit_set(1, ALL_BIT).unwrap()
                && !cron.pattern.months.is_bit_set(2, ALL_BIT).unwrap()
        );
        assert!(
            cron.pattern.days_of_week.is_bit_set(1, ALL_BIT).unwrap()
                && cron.pattern.days_of_week.is_bit_set(5, ALL_BIT).unwrap()
        );
    }

    #[test]
    fn test_cron_pattern_extra_whitespace() {
        let cron = Cron::from_str("  */15  1 1,15 1    1-5    ").unwrap();
        assert!(cron.pattern.minutes.is_bit_set(0, ALL_BIT).unwrap());
        assert!(cron.pattern.hours.is_bit_set(1, ALL_BIT).unwrap());
        assert!(
            cron.pattern.days.is_bit_set(1, ALL_BIT).unwrap()
                && cron.pattern.days.is_bit_set(15, ALL_BIT).unwrap()
        );
        assert!(
            cron.pattern.months.is_bit_set(1, ALL_BIT).unwrap()
                && !cron.pattern.months.is_bit_set(2, ALL_BIT).unwrap()
        );
        assert!(
            cron.pattern.days_of_week.is_bit_set(1, ALL_BIT).unwrap()
                && cron.pattern.days_of_week.is_bit_set(5, ALL_BIT).unwrap()
        );
    }

    #[test]
    fn test_cron_pattern_leading_zeros() {
        let cron = Cron::from_str("  */15  01 01,15 01    01-05    ").unwrap();
        assert!(cron.pattern.minutes.is_bit_set(0, ALL_BIT).unwrap());
        assert!(cron.pattern.hours.is_bit_set(1, ALL_BIT).unwrap());
        assert!(
            cron.pattern.days.is_bit_set(1, ALL_BIT).unwrap()
                && cron.pattern.days.is_bit_set(15, ALL_BIT).unwrap()
        );
        assert!(
            cron.pattern.months.is_bit_set(1, ALL_BIT).unwrap()
                && !cron.pattern.months.is_bit_set(2, ALL_BIT).unwrap()
        );
        assert!(
            cron.pattern.days_of_week.is_bit_set(1, ALL_BIT).unwrap()
                && cron.pattern.days_of_week.is_bit_set(5, ALL_BIT).unwrap()
        );
    }

    #[test]
    fn test_cron_pattern_handle_nicknames() {
        assert_eq!(CronParser::handle_nicknames("@yearly", false, false), "0 0 1 1 *");
        assert_eq!(CronParser::handle_nicknames("@monthly", false, false), "0 0 1 * *");
        assert_eq!(CronParser::handle_nicknames("@weekly", false, false), "0 0 * * 0");
        assert_eq!(CronParser::handle_nicknames("@daily", false, false), "0 0 * * *");
        assert_eq!(CronParser::handle_nicknames("@hourly", false, false), "0 * * * *");
    }

    #[test]
    fn test_cron_pattern_handle_nicknames_with_seconds_required() {
        assert_eq!(CronParser::handle_nicknames("@yearly", true, false), "0 0 0 1 1 *");
        assert_eq!(
            CronParser::handle_nicknames("@monthly", true, false),
            "0 0 0 1 * *"
        );
        assert_eq!(CronParser::handle_nicknames("@weekly", true, false), "0 0 0 * * 0");
        assert_eq!(CronParser::handle_nicknames("@daily", true, false), "0 0 0 * * *");
        assert_eq!(CronParser::handle_nicknames("@hourly", true, false), "0 0 * * * *");
    }

    #[test]
    fn test_month_nickname_range() {
        let cron = Cron::from_str("0 0 * FEB-MAR *").unwrap();
        assert!(!cron.pattern.months.is_bit_set(1, ALL_BIT).unwrap());
        assert!(cron.pattern.months.is_bit_set(2, ALL_BIT).unwrap()); // February
        assert!(cron.pattern.months.is_bit_set(3, ALL_BIT).unwrap()); // March
        assert!(!cron.pattern.months.is_bit_set(4, ALL_BIT).unwrap());
    }

    #[test]
    fn test_weekday_range_sat_sun() {
        let cron = Cron::from_str("0 0 * * SAT-SUN").unwrap();
        assert!(cron.pattern.days_of_week.is_bit_set(0, ALL_BIT).unwrap()); // Sunday
        assert!(cron.pattern.days_of_week.is_bit_set(6, ALL_BIT).unwrap()); // Saturday
    }

    #[test]
    fn test_with_seconds_false() {
        // Explicitly create a parser that disallows seconds
        let parser = CronParser::builder()
            .seconds(Seconds::Disallowed)
            .build();

        // Test with a 6-part pattern when seconds are not allowed
        let error = parser.parse("* * * * * *").unwrap_err();
        assert!(matches!(error, CronError::InvalidPattern(_)));

        // Test with a 5-part pattern when seconds are not allowed
        let no_seconds_pattern = parser.parse("*/10 * * * *").unwrap();

        assert_eq!(no_seconds_pattern.to_string(), "*/10 * * * *");

        // Ensure seconds are defaulted to 0 for a 5-part pattern
        assert!(no_seconds_pattern
            .pattern
            .seconds
            .is_bit_set(0, ALL_BIT)
            .unwrap());
    }

    #[test]
    fn test_with_seconds_required() {
        // Test with a 5-part pattern when seconds are required
        let no_seconds_pattern = CronParser::builder()
            .seconds(Seconds::Required)
            .build()
            .parse("*/10 * * * *")
            .unwrap_err();

        assert!(matches!(no_seconds_pattern, CronError::InvalidPattern(_)));

        // Test with a 6-part pattern when seconds are required
        let cron = CronParser::builder()
            .seconds(Seconds::Required)
            .build()
            .parse("* * * * * *")
            .unwrap();

        // Ensure the 6-part pattern retains seconds information
        // (This assertion depends on how your CronPattern is structured and how it stores seconds information)
        assert!(cron.pattern.seconds.is_bit_set(0, ALL_BIT).unwrap());
    }

    #[test]
    fn test_with_alternative_weekdays() {
        // Test with alternative weekdays enabled
        let cron = CronParser::builder()
            .alternative_weekdays(true)
            .build()
            .parse("* * * * MON-FRI")
            .unwrap();

        // Ensure that the days of the week are offset correctly
        // Note: In this scenario, "MON-FRI" should be treated as "SUN-THU"
        assert!(cron.pattern.days_of_week.is_bit_set(1, ALL_BIT).unwrap()); // Monday
        assert!(cron.pattern.days_of_week.is_bit_set(5, ALL_BIT).unwrap()); // Friday
        assert!(!cron.pattern.days_of_week.is_bit_set(6, ALL_BIT).unwrap()); // Saturday should not be set
    }

    #[test]
    fn test_with_alternative_weekdays_numeric() {
        // Test with alternative weekdays enabled
        let cron = CronParser::builder()
            .alternative_weekdays(true)
            .build()
            .parse("* * * * 2-6")
            .unwrap();

        // Ensure that the days of the week are offset correctly
        // Note: In this scenario, "MON-FRI" should be treated as "SUN-THU"
        assert!(cron.pattern.days_of_week.is_bit_set(1, ALL_BIT).unwrap()); // Monday
        assert!(cron.pattern.days_of_week.is_bit_set(5, ALL_BIT).unwrap()); // Friday
        assert!(!cron.pattern.days_of_week.is_bit_set(6, ALL_BIT).unwrap()); // Saturday should not be set
    }

    #[test]
    fn test_seven_to_zero() {
        // Test with alternative weekdays enabled
        let cron = Cron::from_str("* * * * 7").unwrap();

        // Ensure that the days of the week are offset correctly
        // Note: In this scenario, "MON-FRI" should be treated as "SUN-THU"
        assert!(cron.pattern.days_of_week.is_bit_set(0, ALL_BIT).unwrap()); // Monday
    }

    #[test]
    fn test_one_is_monday_alternative() {
        // Test with alternative weekdays enabled
        let cron = CronParser::builder()
            .alternative_weekdays(true)
            .build()
            .parse("* * * * 1")
            .unwrap();

        // Ensure that the days of the week are offset correctly
        // Note: In this scenario, "MON-FRI" should be treated as "SUN-THU"
        assert!(cron.pattern.days_of_week.is_bit_set(0, ALL_BIT).unwrap()); // Monday
    }

    #[test]
    fn test_zero_with_alternative_weekdays_fails() {
        // Test with alternative weekdays enabled
        let error = CronParser::builder()
            .alternative_weekdays(true)
            .build()
            .parse("* * * * 0")
            .unwrap_err();

        // Parsing should raise a ComponentError
        assert!(matches!(error, CronError::ComponentError(_)));
    }

    #[test]
    fn test_question_mark_allowed_in_day_of_month() {
        let pattern = "* * ? * *";
        assert!(
            Cron::from_str(pattern).is_ok(),
            "Should allow '?' in the day-of-month field."
        );
    }

    #[test]
    fn test_question_mark_allowed_in_day_of_week() {
        let pattern = "* * * * ?";
        assert!(
            Cron::from_str(pattern).is_ok(),
            "Should allow '?' in the day-of-week field."
        );
    }

    #[test]
    fn test_question_mark_disallowed_in_minute() {
        let pattern = "? * * * *";
        let result = Cron::from_str(pattern);
        assert!(
            matches!(result.err(), Some(CronError::IllegalCharacters(_))),
            "Should not allow '?' in the minute field."
        );
    }

    #[test]
    fn test_question_mark_disallowed_in_hour() {
        let pattern = "* ? * * *";
        let result = Cron::from_str(pattern);
        assert!(
            matches!(result.err(), Some(CronError::IllegalCharacters(_))),
            "Should not allow '?' in the hour field."
        );
    }

    #[test]
    fn test_question_mark_disallowed_in_month() {
        let pattern = "* * * ? *";
        let result = Cron::from_str(pattern);
        assert!(
            matches!(result.err(), Some(CronError::IllegalCharacters(_))),
            "Should not allow '?' in the month field."
        );
    }

    #[test]
    fn test_case_sensitivity_lowercase_special_character_ok() {
        let pattern = "* * 15w * *";
        let result = Cron::from_str(pattern);
        assert!(
            result.is_ok(),
            "Should allow lowercase special character w."
        );
    }

    #[test]
    fn test_case_sensitivity_uppercase_special_character_ok() {
        let pattern = "* * 15W * *";
        let result: Result<Cron, CronError> = Cron::from_str(pattern);
        assert!(
            result.is_ok(),
            "Should allow uppercase special character W."
        );
    }

    #[test]
    fn test_year_support() {
        let parser = CronParser::builder()
            .seconds(Seconds::Optional)
            .year(Year::Optional)
            .build();
        // 7-field pattern
        assert!(parser.parse("0 0 0 1 1 * 2025").is_ok());
        // 6-field pattern (year defaults to *)
        assert!(parser.parse("0 0 0 1 1 *").is_ok());
        // 5-field pattern (seconds defaults to 0, year to *)
        assert!(parser.parse("0 0 1 1 *").is_ok());
    }

    #[test]
    fn test_year_required() {
        let parser = CronParser::builder()
            .seconds(Seconds::Required)
            .year(Year::Required)
            .build();
        // Must have 7 fields
        assert!(parser.parse("0 0 0 1 1 * 2025").is_ok());
        // 6 fields should fail
        assert!(parser.parse("0 0 0 1 1 *").is_err());
    }

    #[test]
    fn test_optional_seconds_and_required_year_fails_on_six_parts() {
        // This parser configuration should only accept 7-part patterns.
        let parser = CronParser::builder()
            .seconds(Seconds::Optional)
            .year(Year::Required)
            .build();

        // A 6-part pattern should fail because the year is missing but required.
        let result = parser.parse("* * * * * *");
        
        assert!(matches!(result, Err(CronError::InvalidPattern(_))), "Should fail when year is required but not provided.");
    }
    
}