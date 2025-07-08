// OCPS Compliance Test Suite
//
// This file contains a separate test suite for verifying compliance
// with the Open Cron Pattern Specification (OCPS) 1.4 draft.
//
// Specification Reference: github.com/open-source-cron/ocsp
//
// Each module in this suite corresponds to a specific version of the OCPS,
// allowing for targeted testing of features as they were introduced.

use croner::parser::{CronParser};
use croner::Cron;
use std::str::FromStr;

/// Helper function to parse with a specific configuration.
fn custom_parse(pattern: &str, dom_and_dow: bool) -> Result<Cron, croner::errors::CronError> {
    CronParser::builder()
        .dom_and_dow(dom_and_dow)
        .build()
        .parse(pattern)
}

#[cfg(test)]
mod ocps_1_0_tests {
    use super::*;
    use chrono::{Local, TimeZone}; // Import trait for this module

    #[test]
    fn test_5_field_baseline() {
        assert!(Cron::from_str("15 10 1 10 *").is_ok(), "Should parse a 5-field pattern.");
    }

    #[test]
    fn test_special_chars_wildcard_list_range_step() {
        assert!(Cron::from_str("*/15 0-4,8-12 * JAN-MAR,DEC MON-FRI").is_ok(), "Should handle *, /, -, and , correctly.");
    }

    #[test]
    fn test_logical_or_for_date_fields() {
        // Should match on the 1st AND on every Monday.
        let cron = Cron::from_str("0 12 1 * MON").unwrap();
        let first_of_month = Local.with_ymd_and_hms(2025, 7, 1, 12, 0, 0).unwrap(); // A Tuesday
        let a_monday = Local.with_ymd_and_hms(2025, 7, 14, 12, 0, 0).unwrap(); // Not the 1st

        assert!(cron.is_time_matching(&first_of_month).unwrap());
        assert!(cron.is_time_matching(&a_monday).unwrap());
    }
}

#[cfg(test)]
mod ocps_1_1_tests {
    use super::*;

    #[test]
    fn test_nicknames() {
        assert_eq!(custom_parse("@yearly", false).unwrap().pattern.to_string().to_uppercase(), "0 0 1 1 *");
        assert_eq!(custom_parse("@monthly", false).unwrap().pattern.to_string().to_uppercase(), "0 0 1 * *");
        assert_eq!(custom_parse("@weekly", false).unwrap().pattern.to_string().to_uppercase(), "0 0 * * 0");
        assert_eq!(custom_parse("@daily", false).unwrap().pattern.to_string().to_uppercase(), "0 0 * * *");
        assert_eq!(custom_parse("@hourly", false).unwrap().pattern.to_string().to_uppercase(), "0 * * * *");
    }

    #[test]
    #[ignore] // Ignored until @reboot is implemented
    fn test_reboot_nickname() {
        // The parser should accept @reboot without crashing.
        // A call to find_next_occurrence could then return a specific error if not supported at runtime.
        assert!(custom_parse("@reboot", false).is_ok());
    }
}

#[cfg(test)]
mod ocps_1_2_tests {
    use super::*;

    #[test]
    fn test_6_field_with_seconds() {
        let cron = custom_parse("30 15 10 1 10 *", false).unwrap();
        assert!(cron.pattern.seconds.is_bit_set(30, 1).unwrap());
        assert!(!cron.pattern.seconds.is_bit_set(0, 1).unwrap());
    }

    #[test]
    fn test_7_field_with_year() {
        let cron = custom_parse("0 0 12 1 1 * 2025", false).unwrap();
        assert!(cron.pattern.years.is_bit_set(2025, 1).unwrap());
    }
}

#[cfg(test)]
mod ocps_1_3_tests {
    use super::*;
    use chrono::{Local, TimeZone}; // Import trait for this module

    #[test]
    fn test_last_day_of_month() {
        let cron = Cron::from_str("0 0 L * *").unwrap();
        let last_of_july = Local.with_ymd_and_hms(2025, 7, 31, 0, 0, 0).unwrap();
        let not_last_of_july = Local.with_ymd_and_hms(2025, 7, 30, 0, 0, 0).unwrap();
        assert!(cron.is_time_matching(&last_of_july).unwrap());
        assert!(!cron.is_time_matching(&not_last_of_july).unwrap());
    }

    #[test]
    fn test_last_weekday_of_month() {
        // The last Friday in July 2025 is the 25th.
        let cron = Cron::from_str("0 0 * * 5L").unwrap();
        let last_friday = Local.with_ymd_and_hms(2025, 7, 25, 0, 0, 0).unwrap();
        let not_last_friday = Local.with_ymd_and_hms(2025, 7, 18, 0, 0, 0).unwrap();
        assert!(cron.is_time_matching(&last_friday).unwrap());
        assert!(!cron.is_time_matching(&not_last_friday).unwrap());
    }

    #[test]
    fn test_nth_weekday_of_month() {
        // The second Tuesday in July 2025 is the 8th.
        let cron = Cron::from_str("0 0 * * 2#2").unwrap();
        let second_tuesday = Local.with_ymd_and_hms(2025, 7, 8, 0, 0, 0).unwrap();
        let first_tuesday = Local.with_ymd_and_hms(2025, 7, 1, 0, 0, 0).unwrap();
        assert!(cron.is_time_matching(&second_tuesday).unwrap());
        assert!(!cron.is_time_matching(&first_tuesday).unwrap());
    }

    #[test]
    fn test_closest_weekday() {
        // July 5th, 2025 is a Saturday. The closest weekday is Friday the 4th.
        let cron = Cron::from_str("0 0 5W 7 *").unwrap();
        let closest_weekday = Local.with_ymd_and_hms(2025, 7, 4, 0, 0, 0).unwrap();
        assert!(cron.is_time_matching(&closest_weekday).unwrap());
    }
}

#[cfg(test)]
mod ocps_1_4_tests {
    use super::*;
    use chrono::{Local, TimeZone}; // Import trait for this module

    #[test]
    fn test_question_mark_is_alias_for_wildcard() {
        let cron_star = Cron::from_str("0 0 1 * *").unwrap();
        let cron_q = Cron::from_str("0 0 1 * ?").unwrap();
        assert_eq!(cron_star, cron_q);
    }

    #[test]
    fn test_and_modifier() {
        // Should ONLY match if the 1st of the month is a Monday.
        let cron = custom_parse("0 12 1 * +MON", false).unwrap();

        // September 1st, 2025 is a Monday.
        let first_is_monday = Local.with_ymd_and_hms(2025, 9, 1, 12, 0, 0).unwrap();
        // July 1st, 2025 is a Tuesday.
        let first_is_not_monday = Local.with_ymd_and_hms(2025, 7, 1, 12, 0, 0).unwrap();

        assert!(cron.is_time_matching(&first_is_monday).unwrap());
        assert!(!cron.is_time_matching(&first_is_not_monday).unwrap());
    }

    #[test]
    fn test_global_and_mode() {
        let cron = custom_parse("0 12 1 * MON", true).unwrap();

        // Should ONLY match if the 1st of the month is a Monday (due to global setting).
        let first_is_monday = Local.with_ymd_and_hms(2025, 9, 1, 12, 0, 0).unwrap();
        let first_is_not_monday = Local.with_ymd_and_hms(2025, 7, 1, 12, 0, 0).unwrap();
        let a_monday_not_first = Local.with_ymd_and_hms(2025, 7, 14, 12, 0, 0).unwrap();

        assert!(cron.is_time_matching(&first_is_monday).unwrap());
        assert!(!cron.is_time_matching(&first_is_not_monday).unwrap());
        assert!(!cron.is_time_matching(&a_monday_not_first).unwrap(), "Should not match a Monday that is not the 1st in AND mode.");
    }

    #[test]
    fn test_plus_modifier_invalid_field() {
        // Using '+' in the day-of-month field should result in an error.
        let result = custom_parse("0 0 +1 * *", false);
        assert!(matches!(result, Err(croner::errors::CronError::IllegalCharacters(_))));
    }

}