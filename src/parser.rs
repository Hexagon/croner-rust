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

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::{
    component::{
        CronComponent, ALL_BIT, CLOSEST_WEEKDAY_BIT, LAST_BIT, NONE_BIT, NTH_1ST_BIT, NTH_2ND_BIT,
        NTH_3RD_BIT, NTH_4TH_BIT, NTH_5TH_BIT, NTH_ALL,
    },
    errors::CronError,
    pattern::CronPattern,
    Cron, YEAR_LOWER_LIMIT, YEAR_UPPER_LIMIT,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Seconds {
    #[default]
    Optional,
    Required,
    Disallowed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Year {
    #[default]
    Optional,
    Required,
    Disallowed,
}

impl Seconds {
    pub const fn is_optional(&self) -> bool {
        matches!(self, Self::Optional)
    }

    pub const fn is_required(&self) -> bool {
        matches!(self, Self::Required)
    }

    pub const fn is_disallowed(&self) -> bool {
        matches!(self, Self::Disallowed)
    }
}

impl Year {
    pub const fn is_optional(&self) -> bool {
        matches!(self, Self::Optional)
    }

    pub const fn is_required(&self) -> bool {
        matches!(self, Self::Required)
    }

    pub const fn is_disallowed(&self) -> bool {
        matches!(self, Self::Disallowed)
    }
}

/// Parser for Cron patterns.
///
/// In order to build a custom cron parser use [`CronParser::builder`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CronParser {
    /// Configure how seconds should be handled.
    seconds: Seconds,
    /// Configure how years should be handled.
    year: Year,
    /// Enable the combination of Day of Month (DOM) and Day of Week (DOW) conditions.
    dom_and_dow: bool,
    /// Use the Quartz-style weekday mode.
    alternative_weekdays: bool,
    /// Allow sloppy range syntax (e.g., `0/10` or `/10`) for backward compatibility.
    /// When enabled, patterns like `0/10` (start at 0, step by 10) and `/10` (same as `*/10`)
    /// are accepted. This is not compliant with OCPS/vixie-cron standards.
    sloppy_ranges: bool,
}

impl CronParser {
    /// Create a new parser.
    ///
    /// You should probably be using [`Cron`]'s implementation of
    /// [`FromStr`][core::str::FromStr] instead of invoking this.
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
            pattern = Self::handle_nicknames(
                &pattern,
                self.seconds.is_required(),
                self.year.is_required(),
            )
            .to_string();
        }

        // Handle day-of-week and month aliases (MON... and JAN...). Names are
        // only replaced in their own field, so a name in any other field is
        // rejected as an illegal character instead of being read as a number.
        // A pattern without letters has no names to replace, so skip the work.
        if pattern.bytes().any(|b| b.is_ascii_alphabetic()) {
            let mut fields: Vec<String> = pattern.split_whitespace().map(str::to_string).collect();
            let (month_idx, dow_idx) = match fields.len() {
                5 => (Some(3), Some(4)),
                6 | 7 => (Some(4), Some(5)),
                _ => (None, None),
            };
            let mut changed = false;
            if let Some(i) = month_idx {
                let replaced = Self::replace_alpha_months(&fields[i]);
                changed |= replaced != fields[i];
                fields[i] = replaced;
            }
            if let Some(i) = dow_idx {
                let replaced = Self::replace_alpha_weekdays(&fields[i], self.alternative_weekdays);
                changed |= replaced != fields[i];
                fields[i] = replaced;
            }
            if changed {
                pattern = fields.join(" ");
            }
        }

        // Split the pattern into parts
        let mut parts: Vec<&str> = pattern.split_whitespace().collect();
        let num_parts = parts.len();

        // Default seconds to "0" if omitted in an optional context
        if num_parts == 5 {
            parts.insert(0, "0");
        } else if self.seconds.is_disallowed() {
            return Err(CronError::InvalidPattern(
                "Pattern must have 5 fields when seconds are disallowed.".to_string(),
            ));
        }

        // Default year to "*" if omitted in an optional context
        if parts.len() == 6 {
            parts.push("*");
        } else if self.year.is_disallowed() {
            return Err(CronError::InvalidPattern(
                "Pattern must have 5 or 6 fields when years are disallowed.".to_string(),
            ));
        }

        // Validate pattern length based on configuration
        if self.seconds.is_required() {
            if self.year.is_required() && num_parts != 7 {
                return Err(CronError::InvalidPattern(
                    "Pattern must have 7 fields when seconds and years are required.".to_string(),
                ));
            }
            if self.year.is_disallowed() && num_parts != 6 {
                return Err(CronError::InvalidPattern("Pattern must have 6 fields when seconds are required and years are disallowed.".to_string()));
            }
            if self.year.is_optional() && !(6..=7).contains(&num_parts) {
                return Err(CronError::InvalidPattern("Pattern must have 6 or 7 fields when seconds are required and years are optional.".to_string()));
            }
        } else if self.year.is_required() && num_parts != 7 {
            return Err(CronError::InvalidPattern(
                "Pattern must have 7 fields when years are required.".to_string(),
            ));
        } else if !(5..=7).contains(&num_parts) {
            return Err(CronError::InvalidPattern(
                "Pattern must have between 5 and 7 fields.".to_string(),
            ));
        }

        // Replace ? with * in day-of-month and day-of-week
        let mut owned_parts = parts.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        if owned_parts.get(3).is_some_and(|p| p.contains('?')) {
            owned_parts[3] = owned_parts[3].replace('?', "*");
        }
        if owned_parts.get(5).is_some_and(|p| p.contains('?')) {
            owned_parts[5] = owned_parts[5].replace('?', "*");
        }

        // Check for the '+' (AND) modifier in the day-of-week field.
        // This must be done before illegal character validation.
        let mut dom_and_dow_from_pattern = false;
        if let Some(dow_part) = owned_parts.get_mut(5) {
            if dow_part.starts_with('+') {
                dom_and_dow_from_pattern = true;
                // Remove the '+' so the rest of the field can be parsed normally.
                *dow_part = dow_part[1..].to_string();
            }
        }
        parts = owned_parts.iter().map(|s| s.as_str()).collect();

        // Throw at illegal characters
        self.throw_at_illegal_characters(&parts)?;

        // Handle star-dom and star-dow
        let star_dom = parts.get(3).is_some_and(|&p| p == "*");
        let star_dow = parts.get(5).is_some_and(|&p| p == "*");

        // Parse the individual components
        let mut seconds = CronComponent::new(0, 59, NONE_BIT, 0);
        seconds.set_sloppy_ranges(self.sloppy_ranges);
        seconds.parse(parts[0])?;

        let mut minutes = CronComponent::new(0, 59, NONE_BIT, 0);
        minutes.set_sloppy_ranges(self.sloppy_ranges);
        minutes.parse(parts[1])?;

        let mut hours = CronComponent::new(0, 23, NONE_BIT, 0);
        hours.set_sloppy_ranges(self.sloppy_ranges);
        hours.parse(parts[2])?;
        let mut days = CronComponent::new(1, 31, LAST_BIT | CLOSEST_WEEKDAY_BIT, 0);
        days.set_sloppy_ranges(self.sloppy_ranges);
        days.parse(parts[3])?;
        let mut months = CronComponent::new(1, 12, NONE_BIT, 0);
        months.set_sloppy_ranges(self.sloppy_ranges);
        months.parse(parts[4])?;

        let mut days_of_week = if self.alternative_weekdays {
            CronComponent::new(0, 7, LAST_BIT | NTH_ALL, 1)
        } else {
            CronComponent::new(0, 7, LAST_BIT | NTH_ALL, 0)
        };
        days_of_week.set_sloppy_ranges(self.sloppy_ranges);
        // Quartz wraps day-of-week ranges around the week, so SAT-SUN (7-1)
        // is Saturday and Sunday, and FRI-MON (6-2) is Friday to Monday.
        days_of_week.set_wrap_ranges(self.alternative_weekdays);
        days_of_week.parse(parts[5])?;

        let mut years = CronComponent::new(
            YEAR_LOWER_LIMIT as u16,
            YEAR_UPPER_LIMIT as u16,
            NONE_BIT,
            0,
        ); // Placeholder, real limits are i32
        years.set_sloppy_ranges(self.sloppy_ranges);
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
                LAST_BIT,
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
                dom_and_dow: self.dom_and_dow || dom_and_dow_from_pattern,
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
                3 => [
                    base_allowed_characters.as_ref(),
                    day_of_month_additional_characters.as_ref(),
                ]
                .concat(),
                5 => [
                    base_allowed_characters.as_ref(),
                    day_of_week_additional_characters.as_ref(),
                ]
                .concat(),
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
            p if eq_ignore_case(p, "@weekly") => "0 0 * * SUN",
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
                // SUN is 1 here, so X-SUN is a range that wraps around the week
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

        // `-SUN` maps to 7 so that ranges like SAT-SUN work, but SUN-SUN would then
        // become 0-7 (every day). Treat SUN-SUN as Sunday only.
        if !alternative_weekdays {
            replaced = replaced.replace("SUN-SUN", "0-0");
        }

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

/// Builder for a custom [`CronParser`], created with [`CronParser::builder`].
///
/// Every option defaults to the same value as [`CronParser::new`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[must_use]
pub struct CronParserBuilder {
    parser: CronParser,
}

impl CronParserBuilder {
    /// Configure how seconds should be handled.
    pub fn seconds(mut self, value: Seconds) -> Self {
        self.parser.seconds = value;
        self
    }

    /// Configure how years should be handled.
    pub fn year(mut self, value: Year) -> Self {
        self.parser.year = value;
        self
    }

    /// Enable the combination of Day of Month (DOM) and Day of Week (DOW) conditions.
    pub fn dom_and_dow(mut self, value: bool) -> Self {
        self.parser.dom_and_dow = value;
        self
    }

    /// Use the Quartz-style weekday mode.
    pub fn alternative_weekdays(mut self, value: bool) -> Self {
        self.parser.alternative_weekdays = value;
        self
    }

    /// Allow sloppy range syntax (e.g., `0/10` or `/10`) for backward compatibility.
    ///
    /// When enabled, patterns like `0/10` (start at 0, step by 10) and `/10` (same as `*/10`)
    /// are accepted. This is not compliant with OCPS/vixie-cron standards.
    pub fn sloppy_ranges(mut self, value: bool) -> Self {
        self.parser.sloppy_ranges = value;
        self
    }

    /// Build the configured [`CronParser`].
    pub fn build(self) -> CronParser {
        self.parser
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
        let cron = Cron::from_str("5-59/5 * * * *").unwrap();
        assert_eq!(cron.pattern.pattern, "5-59/5 * * * *");
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
        assert_eq!(
            CronParser::handle_nicknames("@yearly", false, false),
            "0 0 1 1 *"
        );
        assert_eq!(
            CronParser::handle_nicknames("@monthly", false, false),
            "0 0 1 * *"
        );
        assert_eq!(
            CronParser::handle_nicknames("@weekly", false, false),
            "0 0 * * SUN"
        );
        assert_eq!(
            CronParser::handle_nicknames("@daily", false, false),
            "0 0 * * *"
        );
        assert_eq!(
            CronParser::handle_nicknames("@hourly", false, false),
            "0 * * * *"
        );
    }

    #[test]
    fn test_cron_pattern_handle_nicknames_with_seconds_required() {
        assert_eq!(
            CronParser::handle_nicknames("@yearly", true, false),
            "0 0 0 1 1 *"
        );
        assert_eq!(
            CronParser::handle_nicknames("@monthly", true, false),
            "0 0 0 1 * *"
        );
        assert_eq!(
            CronParser::handle_nicknames("@weekly", true, false),
            "0 0 0 * * SUN"
        );
        assert_eq!(
            CronParser::handle_nicknames("@daily", true, false),
            "0 0 0 * * *"
        );
        assert_eq!(
            CronParser::handle_nicknames("@hourly", true, false),
            "0 0 * * * *"
        );
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

    fn weekday_parser(alternative_weekdays: bool) -> CronParser {
        CronParser::builder()
            .alternative_weekdays(alternative_weekdays)
            .build()
    }

    #[cfg(feature = "chrono")]
    /// The weekdays a pattern runs on during one week, Monday first.
    fn weekdays_in_week(cron: &Cron) -> Vec<chrono::Weekday> {
        use chrono::{TimeZone, Utc};
        // 2026-11-23 is a Monday
        let before = Utc.with_ymd_and_hms(2026, 11, 22, 23, 59, 59).unwrap();
        let next_week = Utc.with_ymd_and_hms(2026, 11, 30, 0, 0, 0).unwrap();
        cron.iter_after(before)
            .take_while(|run| *run < next_week)
            .map(|run| chrono::Datelike::weekday(&run))
            .collect()
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn test_weekday_ranges_ending_in_sun_both_modes() {
        use chrono::Weekday::*;
        let all = vec![Mon, Tue, Wed, Thu, Fri, Sat, Sun];
        for alternative_weekdays in [false, true] {
            let parser = weekday_parser(alternative_weekdays);
            for (field, expected) in [
                ("SAT-SUN", vec![Sat, Sun]),
                ("FRI-SUN", vec![Fri, Sat, Sun]),
                ("MON-SUN", all.clone()),
                ("SUN-SAT", all.clone()),
                ("MON-FRI", vec![Mon, Tue, Wed, Thu, Fri]),
                ("SAT-SUN,WED", vec![Wed, Sat, Sun]),
                ("sat-sun", vec![Sat, Sun]),
            ] {
                let pattern = format!("0 0 * * {field}");
                let cron = parser.parse(&pattern).unwrap_or_else(|e| {
                    panic!("{pattern} (alternative_weekdays={alternative_weekdays}): {e:?}")
                });
                assert_eq!(
                    weekdays_in_week(&cron),
                    expected,
                    "{pattern} (alternative_weekdays={alternative_weekdays})"
                );
            }
        }
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn test_weekday_range_sat_sun_nth_both_modes() {
        use chrono::{TimeZone, Utc};
        for alternative_weekdays in [false, true] {
            let cron = weekday_parser(alternative_weekdays)
                .parse("0 0 * * SAT-SUN#2")
                .unwrap();
            let start = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
            let runs: Vec<_> = cron.iter_after(start).take(4).collect();
            assert_eq!(
                runs,
                vec![
                    Utc.with_ymd_and_hms(2026, 12, 12, 0, 0, 0).unwrap(),
                    Utc.with_ymd_and_hms(2026, 12, 13, 0, 0, 0).unwrap(),
                    Utc.with_ymd_and_hms(2027, 1, 9, 0, 0, 0).unwrap(),
                    Utc.with_ymd_and_hms(2027, 1, 10, 0, 0, 0).unwrap(),
                ],
                "alternative_weekdays={alternative_weekdays}"
            );
        }
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn test_weekday_wrapping_ranges_quartz_only() {
        use chrono::Weekday::*;
        let quartz = weekday_parser(true);
        for (field, expected) in [
            ("FRI-MON", vec![Mon, Fri, Sat, Sun]),
            ("7-1", vec![Sat, Sun]),
            ("6-2", vec![Mon, Fri, Sat, Sun]),
            ("FRI-TUE/2", vec![Tue, Fri, Sun]),
            ("7-1/1", vec![Sat, Sun]),
        ] {
            let cron = quartz.parse(&format!("0 0 * * {field}")).unwrap();
            assert_eq!(weekdays_in_week(&cron), expected, "Quartz {field}");
        }

        // Out-of-range ends are still rejected, wrapping or not
        for field in ["8-1", "7-0", "0-7"] {
            assert!(
                quartz.parse(&format!("0 0 * * {field}")).is_err(),
                "Quartz {field} should fail"
            );
        }

        // Standard mode does not wrap
        let standard = weekday_parser(false);
        for field in ["FRI-MON", "6-0", "5-1", "FRI-TUE/2"] {
            assert!(
                standard.parse(&format!("0 0 * * {field}")).is_err(),
                "standard {field} should fail"
            );
        }
    }

    #[test]
    fn test_wrapping_only_applies_to_quartz_weekdays() {
        let quartz = weekday_parser(true);
        for pattern in ["0 23-1 * * *", "0 0 30-2 * *", "0 0 * 12-1 *"] {
            assert!(quartz.parse(pattern).is_err(), "{pattern} should fail");
        }
    }

    #[test]
    fn test_weekday_range_sun_sun_is_sunday_only() {
        for (named, numeric) in [
            ("0 0 * * SUN-SUN", "0 0 * * 0"),
            ("0 0 * * sun-sun,WED", "0 0 * * 0,3"),
            ("0 0 * * SUN-SUN#2", "0 0 * * 0#2"),
            ("0 0 * * SUN-SUN/2", "0 0 * * 0-0/2"),
            ("0 0 * * SUN-TUE,FRI", "0 0 * * 0-2,5"),
            ("0 0 * * SAT-SUN", "0 0 * * 6-7"),
        ] {
            let named_cron = Cron::from_str(named).unwrap();
            let numeric_cron = Cron::from_str(numeric).unwrap();
            assert_eq!(
                named_cron.pattern.days_of_week, numeric_cron.pattern.days_of_week,
                "{named} should equal {numeric}"
            );
        }
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn test_weekday_range_sun_sun_next_runs() {
        use chrono::{TimeZone, Utc};
        let cron = Cron::from_str("0 0 * * SUN-SUN").unwrap();
        let start = Utc.with_ymd_and_hms(2026, 11, 23, 0, 0, 0).unwrap();
        let runs: Vec<_> = cron.iter_after(start).take(3).collect();
        assert_eq!(
            runs,
            vec![
                Utc.with_ymd_and_hms(2026, 11, 29, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 12, 6, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 12, 13, 0, 0, 0).unwrap(),
            ]
        );
    }

    #[test]
    fn test_weekday_range_sun_sun_alternative_weekdays() {
        let parser = CronParser::builder()
            .seconds(Seconds::Optional)
            .alternative_weekdays(true)
            .build();
        let named = parser.parse("0 0 0 ? * SUN-SUN").unwrap();
        let numeric = parser.parse("0 0 0 ? * 1").unwrap();
        assert_eq!(named.pattern.days_of_week, numeric.pattern.days_of_week);
    }

    #[test]
    fn test_with_seconds_false() {
        // Explicitly create a parser that disallows seconds
        let parser = CronParser::builder().seconds(Seconds::Disallowed).build();

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
    fn test_with_alternative_weekdays_wildcard_step() {
        for (pattern, sloppy_ranges) in [("* * * * */2", false), ("* * * * /2", true)] {
            let cron = CronParser::builder()
                .alternative_weekdays(true)
                .sloppy_ranges(sloppy_ranges)
                .build()
                .parse(pattern)
                .unwrap();

            // Steps count from 1 (Sunday), so this is 1,3,5,7: Sunday, Tuesday, Thursday, Saturday
            assert_eq!(
                cron.pattern.days_of_week.get_set_values(ALL_BIT),
                vec![0, 2, 4, 6]
            );
        }
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

    // Tests for case insensitivity of aliases and modifiers
    #[test]
    fn test_nickname_aliases_case_insensitive() {
        // Test each nickname alias in different cases
        let nicknames = [
            ("@yearly", "@YEARLY", "@YeArLy"),
            ("@annually", "@ANNUALLY", "@AnNuAlLy"),
            ("@monthly", "@MONTHLY", "@MoNtHlY"),
            ("@weekly", "@WEEKLY", "@WeEkLy"),
            ("@daily", "@DAILY", "@DaIlY"),
            ("@hourly", "@HOURLY", "@HoUrLy"),
        ];

        for (lower, upper, mixed) in nicknames {
            let result_lower = Cron::from_str(lower);
            let result_upper = Cron::from_str(upper);
            let result_mixed = Cron::from_str(mixed);

            assert!(result_lower.is_ok(), "Should parse lowercase {}", lower);
            assert!(result_upper.is_ok(), "Should parse uppercase {}", upper);
            assert!(result_mixed.is_ok(), "Should parse mixed case {}", mixed);

            // Verify they all produce the same result
            let cron_lower = result_lower.unwrap();
            let cron_upper = result_upper.unwrap();
            let cron_mixed = result_mixed.unwrap();

            assert_eq!(
                cron_lower.pattern, cron_upper.pattern,
                "Lowercase and uppercase {} should produce the same pattern",
                lower
            );
            assert_eq!(
                cron_lower.pattern, cron_mixed.pattern,
                "Lowercase and mixed case {} should produce the same pattern",
                lower
            );
        }
    }

    #[test]
    fn test_month_aliases_case_insensitive() {
        // Test each month alias in different cases
        let months = [
            ("JAN", "jan", "Jan"),
            ("FEB", "feb", "Feb"),
            ("MAR", "mar", "Mar"),
            ("APR", "apr", "Apr"),
            ("MAY", "may", "May"),
            ("JUN", "jun", "Jun"),
            ("JUL", "jul", "Jul"),
            ("AUG", "aug", "Aug"),
            ("SEP", "sep", "Sep"),
            ("OCT", "oct", "Oct"),
            ("NOV", "nov", "Nov"),
            ("DEC", "dec", "Dec"),
        ];

        for (upper, lower, mixed) in months {
            let pattern_upper = format!("0 0 1 {} *", upper);
            let pattern_lower = format!("0 0 1 {} *", lower);
            let pattern_mixed = format!("0 0 1 {} *", mixed);

            let result_upper = Cron::from_str(&pattern_upper);
            let result_lower = Cron::from_str(&pattern_lower);
            let result_mixed = Cron::from_str(&pattern_mixed);

            assert!(result_upper.is_ok(), "Should parse uppercase {}", upper);
            assert!(result_lower.is_ok(), "Should parse lowercase {}", lower);
            assert!(result_mixed.is_ok(), "Should parse mixed case {}", mixed);

            // Verify they all produce the same result
            let cron_upper = result_upper.unwrap();
            let cron_lower = result_lower.unwrap();
            let cron_mixed = result_mixed.unwrap();

            assert_eq!(
                cron_upper.pattern.months, cron_lower.pattern.months,
                "Uppercase and lowercase {} should produce the same months",
                upper
            );
            assert_eq!(
                cron_upper.pattern.months, cron_mixed.pattern.months,
                "Uppercase and mixed case {} should produce the same months",
                upper
            );
        }
    }

    #[test]
    fn test_weekday_aliases_case_insensitive() {
        // Test each weekday alias in different cases
        let weekdays = [
            ("SUN", "sun", "Sun"),
            ("MON", "mon", "Mon"),
            ("TUE", "tue", "Tue"),
            ("WED", "wed", "Wed"),
            ("THU", "thu", "Thu"),
            ("FRI", "fri", "Fri"),
            ("SAT", "sat", "Sat"),
        ];

        for (upper, lower, mixed) in weekdays {
            let pattern_upper = format!("0 0 * * {}", upper);
            let pattern_lower = format!("0 0 * * {}", lower);
            let pattern_mixed = format!("0 0 * * {}", mixed);

            let result_upper = Cron::from_str(&pattern_upper);
            let result_lower = Cron::from_str(&pattern_lower);
            let result_mixed = Cron::from_str(&pattern_mixed);

            assert!(result_upper.is_ok(), "Should parse uppercase {}", upper);
            assert!(result_lower.is_ok(), "Should parse lowercase {}", lower);
            assert!(result_mixed.is_ok(), "Should parse mixed case {}", mixed);

            // Verify they all produce the same result
            let cron_upper = result_upper.unwrap();
            let cron_lower = result_lower.unwrap();
            let cron_mixed = result_mixed.unwrap();

            assert_eq!(
                cron_upper.pattern.days_of_week, cron_lower.pattern.days_of_week,
                "Uppercase and lowercase {} should produce the same days_of_week",
                upper
            );
            assert_eq!(
                cron_upper.pattern.days_of_week, cron_mixed.pattern.days_of_week,
                "Uppercase and mixed case {} should produce the same days_of_week",
                upper
            );
        }
    }

    #[test]
    fn test_last_modifier_case_insensitive() {
        // Test L modifier for last day of month
        let patterns = ["0 0 L * *", "0 0 l * *"];
        let results: Vec<_> = patterns.iter().map(|p| Cron::from_str(p)).collect();

        for (i, result) in results.iter().enumerate() {
            assert!(
                result.is_ok(),
                "Should parse L modifier pattern: {}",
                patterns[i]
            );
        }

        let cron_upper = results[0].as_ref().unwrap();
        let cron_lower = results[1].as_ref().unwrap();
        assert_eq!(
            cron_upper.pattern.days, cron_lower.pattern.days,
            "L and l modifiers should produce the same days"
        );
    }

    #[test]
    fn test_last_weekday_modifier_case_insensitive() {
        // Test xL modifier for last occurrence of weekday
        let patterns = ["0 0 * * 5L", "0 0 * * 5l"];
        let results: Vec<_> = patterns.iter().map(|p| Cron::from_str(p)).collect();

        for (i, result) in results.iter().enumerate() {
            assert!(
                result.is_ok(),
                "Should parse weekday L modifier pattern: {}",
                patterns[i]
            );
        }

        let cron_upper = results[0].as_ref().unwrap();
        let cron_lower = results[1].as_ref().unwrap();
        assert_eq!(
            cron_upper.pattern.days_of_week, cron_lower.pattern.days_of_week,
            "5L and 5l modifiers should produce the same days_of_week"
        );
    }

    #[test]
    fn test_closest_weekday_modifier_case_insensitive() {
        // Test W modifier for closest weekday
        let patterns = ["0 0 15W * *", "0 0 15w * *"];
        let results: Vec<_> = patterns.iter().map(|p| Cron::from_str(p)).collect();

        for (i, result) in results.iter().enumerate() {
            assert!(
                result.is_ok(),
                "Should parse W modifier pattern: {}",
                patterns[i]
            );
        }

        let cron_upper = results[0].as_ref().unwrap();
        let cron_lower = results[1].as_ref().unwrap();
        assert_eq!(
            cron_upper.pattern.days, cron_lower.pattern.days,
            "15W and 15w modifiers should produce the same days"
        );
    }

    #[test]
    fn test_last_weekday_of_month_lw_case_insensitive() {
        // Test LW modifier for last weekday of month
        let patterns = ["0 0 LW * *", "0 0 lw * *", "0 0 Lw * *", "0 0 lW * *"];
        let results: Vec<_> = patterns.iter().map(|p| Cron::from_str(p)).collect();

        for (i, result) in results.iter().enumerate() {
            assert!(
                result.is_ok(),
                "Should parse LW modifier pattern: {}",
                patterns[i]
            );
        }

        // Verify they all produce the same result
        let cron_first = results[0].as_ref().unwrap();
        for (i, result) in results.iter().enumerate().skip(1) {
            let cron = result.as_ref().unwrap();
            assert_eq!(
                cron_first.pattern.days, cron.pattern.days,
                "LW pattern {} should produce the same days as LW",
                patterns[i]
            );
        }
    }

    #[test]
    fn test_weekday_alias_with_nth_modifier_case_insensitive() {
        // Test weekday aliases with # modifier in different cases
        let patterns = ["0 0 * * MON#2", "0 0 * * mon#2", "0 0 * * Mon#2"];
        let results: Vec<_> = patterns.iter().map(|p| Cron::from_str(p)).collect();

        for (i, result) in results.iter().enumerate() {
            assert!(
                result.is_ok(),
                "Should parse weekday with # pattern: {}",
                patterns[i]
            );
        }

        let cron_first = results[0].as_ref().unwrap();
        for (i, result) in results.iter().enumerate().skip(1) {
            let cron = result.as_ref().unwrap();
            assert_eq!(
                cron_first.pattern.days_of_week, cron.pattern.days_of_week,
                "Pattern {} should produce the same days_of_week as MON#2",
                patterns[i]
            );
        }
    }

    #[test]
    fn test_month_alias_range_case_insensitive() {
        // Test month ranges with different cases
        let patterns = ["0 0 * JAN-MAR *", "0 0 * jan-mar *", "0 0 * Jan-Mar *"];
        let results: Vec<_> = patterns.iter().map(|p| Cron::from_str(p)).collect();

        for (i, result) in results.iter().enumerate() {
            assert!(
                result.is_ok(),
                "Should parse month range pattern: {}",
                patterns[i]
            );
        }

        let cron_first = results[0].as_ref().unwrap();
        for (i, result) in results.iter().enumerate().skip(1) {
            let cron = result.as_ref().unwrap();
            assert_eq!(
                cron_first.pattern.months, cron.pattern.months,
                "Pattern {} should produce the same months as JAN-MAR",
                patterns[i]
            );
        }
    }

    #[test]
    fn test_weekday_alias_range_case_insensitive() {
        // Test weekday ranges with different cases
        let patterns = ["0 0 * * MON-FRI", "0 0 * * mon-fri", "0 0 * * Mon-Fri"];
        let results: Vec<_> = patterns.iter().map(|p| Cron::from_str(p)).collect();

        for (i, result) in results.iter().enumerate() {
            assert!(
                result.is_ok(),
                "Should parse weekday range pattern: {}",
                patterns[i]
            );
        }

        let cron_first = results[0].as_ref().unwrap();
        for (i, result) in results.iter().enumerate().skip(1) {
            let cron = result.as_ref().unwrap();
            assert_eq!(
                cron_first.pattern.days_of_week, cron.pattern.days_of_week,
                "Pattern {} should produce the same days_of_week as MON-FRI",
                patterns[i]
            );
        }
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

        assert!(
            matches!(result, Err(CronError::InvalidPattern(_))),
            "Should fail when year is required but not provided."
        );
    }

    #[test]
    fn test_names_only_accepted_in_their_own_field() {
        // Weekday names outside the day-of-week field (#72)
        for pattern in [
            "mon * * * *",
            "* mon * * *",
            "* * mon * *",
            "0 0 * mon *",
            "0 0 0 * mon * *",
            "* * * * * * mon",
        ] {
            assert!(
                matches!(
                    CronParser::builder()
                        .seconds(Seconds::Optional)
                        .year(Year::Optional)
                        .build()
                        .parse(pattern),
                    Err(CronError::IllegalCharacters(_))
                ),
                "{pattern} should be rejected"
            );
        }
        // Month names outside the month field
        for pattern in [
            "jan * * * *",
            "0 jan * * *",
            "0 0 jan * *",
            "0 0 * * jan",
            "0 0 0 * * jan",
        ] {
            assert!(
                matches!(
                    Cron::from_str(pattern),
                    Err(CronError::IllegalCharacters(_))
                ),
                "{pattern} should be rejected"
            );
        }
        // Alternative weekdays don't change that
        let alt = CronParser::builder().alternative_weekdays(true).build();
        assert!(alt.parse("0 0 * mon *").is_err());
        assert!(alt.parse("0 0 * * jan").is_err());
    }

    #[test]
    fn test_names_accepted_in_their_own_field() {
        let cron = Cron::from_str("0 0 * jan-mar,dec mon-fri").unwrap();
        assert_eq!(cron.pattern.pattern, "0 0 * 1-3,12 1-5");

        let cron = Cron::from_str("0 0 0 * feb sun 2030").unwrap();
        assert_eq!(cron.pattern.pattern, "0 0 0 * 2 0 2030");

        let cron = Cron::from_str("0 0 * * fri-sun").unwrap();
        assert_eq!(cron.pattern.pattern, "0 0 * * 5-7");

        let cron = CronParser::builder()
            .alternative_weekdays(true)
            .build()
            .parse("0 0 0 * dec mon")
            .unwrap();
        assert_eq!(cron.pattern.pattern, "0 0 0 * 12 2");

        // Plain patterns keep their original spacing
        let cron = Cron::from_str("0  0 * * *").unwrap();
        assert_eq!(cron.pattern.pattern, "0  0 * * *");
    }
}
