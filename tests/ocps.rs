// OCPS Compliance Test Suite
//
// This file contains a separate test suite for verifying compliance
// with the Open Cron Pattern Specification (OCPS) 1.4 draft.
//
// Specification Reference: github.com/open-source-cron/ocsp
//
// Each module in this suite corresponds to a specific version of the OCPS,
// allowing for targeted testing of features as they were introduced.

use croner::parser::CronParser;
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
        assert!(
            Cron::from_str("15 10 1 10 *").is_ok(),
            "Should parse a 5-field pattern."
        );
    }

    #[test]
    fn test_special_chars_wildcard_list_range_step() {
        assert!(
            Cron::from_str("*/15 0-4,8-12 * JAN-MAR,DEC MON-FRI").is_ok(),
            "Should handle *, /, -, and , correctly."
        );
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
        assert_eq!(
            custom_parse("@yearly", false)
                .unwrap()
                .pattern
                .to_string()
                .to_uppercase(),
            "0 0 1 1 *"
        );
        assert_eq!(
            custom_parse("@monthly", false)
                .unwrap()
                .pattern
                .to_string()
                .to_uppercase(),
            "0 0 1 * *"
        );
        assert_eq!(
            custom_parse("@weekly", false)
                .unwrap()
                .pattern
                .to_string()
                .to_uppercase(),
            "0 0 * * 0"
        );
        assert_eq!(
            custom_parse("@daily", false)
                .unwrap()
                .pattern
                .to_string()
                .to_uppercase(),
            "0 0 * * *"
        );
        assert_eq!(
            custom_parse("@hourly", false)
                .unwrap()
                .pattern
                .to_string()
                .to_uppercase(),
            "0 * * * *"
        );
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
    use chrono::{Datelike, Local, TimeZone}; // Import traits for this module

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

    #[test]
    fn test_last_weekday_lw() {
        // Test LW (last weekday of the month)
        let cron = Cron::from_str("0 0 LW * *").unwrap();

        // February 2025: last day is 28 (Friday), last weekday is 28 (Friday)
        let feb_last_weekday = Local.with_ymd_and_hms(2025, 2, 28, 0, 0, 0).unwrap();
        assert!(
            cron.is_time_matching(&feb_last_weekday).unwrap(),
            "Should match Feb 28 (Friday) as last weekday"
        );

        // August 2025: last day is 31 (Sunday), last weekday is 29 (Friday)
        let aug_last_weekday = Local.with_ymd_and_hms(2025, 8, 29, 0, 0, 0).unwrap();
        assert!(
            cron.is_time_matching(&aug_last_weekday).unwrap(),
            "Should match Aug 29 (Friday) as last weekday"
        );

        // August 2025: 31st is Sunday, should NOT match
        let aug_31 = Local.with_ymd_and_hms(2025, 8, 31, 0, 0, 0).unwrap();
        assert!(
            !cron.is_time_matching(&aug_31).unwrap(),
            "Should NOT match Aug 31 (Sunday)"
        );

        // November 2025: last day is 30 (Sunday), last weekday is 28 (Friday)
        let nov_last_weekday = Local.with_ymd_and_hms(2025, 11, 28, 0, 0, 0).unwrap();
        assert!(
            cron.is_time_matching(&nov_last_weekday).unwrap(),
            "Should match Nov 28 (Friday) as last weekday"
        );

        // November 2025: 30th is Sunday, should NOT match
        let nov_30 = Local.with_ymd_and_hms(2025, 11, 30, 0, 0, 0).unwrap();
        assert!(
            !cron.is_time_matching(&nov_30).unwrap(),
            "Should NOT match Nov 30 (Sunday)"
        );
    }

    #[test]
    fn test_31w_only_triggers_if_31st_exists() {
        // Test that 31W only triggers if the 31st is present in the current month
        let cron = Cron::from_str("0 0 31W * *").unwrap();

        // January 2025: has 31 days, 31st is Friday, so 31W should match Jan 31
        let jan_31 = Local.with_ymd_and_hms(2025, 1, 31, 0, 0, 0).unwrap();
        assert!(
            cron.is_time_matching(&jan_31).unwrap(),
            "Should match Jan 31 (Friday)"
        );

        // February 2025: has only 28 days, 31W should NOT trigger in February at all
        let feb_28 = Local.with_ymd_and_hms(2025, 2, 28, 0, 0, 0).unwrap();
        assert!(
            !cron.is_time_matching(&feb_28).unwrap(),
            "Should NOT match in February (no 31st)"
        );

        // April 2025: has only 30 days, 31W should NOT trigger in April at all
        let apr_30 = Local.with_ymd_and_hms(2025, 4, 30, 0, 0, 0).unwrap();
        assert!(
            !cron.is_time_matching(&apr_30).unwrap(),
            "Should NOT match in April (no 31st)"
        );

        // May 2025: has 31 days, 31st is Saturday, so 31W should match May 30 (Friday)
        let may_30 = Local.with_ymd_and_hms(2025, 5, 30, 0, 0, 0).unwrap();
        assert!(
            cron.is_time_matching(&may_30).unwrap(),
            "Should match May 30 (Friday) for 31W"
        );

        // August 2025: has 31 days, 31st is Sunday, so 31W should match Aug 29 (Friday)
        let aug_29 = Local.with_ymd_and_hms(2025, 8, 29, 0, 0, 0).unwrap();
        assert!(
            cron.is_time_matching(&aug_29).unwrap(),
            "Should match Aug 29 (Friday) for 31W"
        );
    }

    #[test]
    fn test_lw_find_next_occurrences() {
        // Test that LW correctly finds the last weekday of each month
        let cron = Cron::from_str("0 0 LW * *").expect("Failed to parse LW pattern");

        // Start from Jan 1, 2025
        let start_date = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        // Expected last weekdays for each month in 2025
        let expected = vec![
            (2025, 1, 31),  // January 31 (Friday)
            (2025, 2, 28),  // February 28 (Friday)
            (2025, 3, 31),  // March 31 (Monday)
            (2025, 4, 30),  // April 30 (Wednesday)
            (2025, 5, 30),  // May 30 (Friday)
            (2025, 6, 30),  // June 30 (Monday)
            (2025, 7, 31),  // July 31 (Thursday)
            (2025, 8, 29),  // August 29 (Friday) - 31st is Sunday
            (2025, 9, 30),  // September 30 (Tuesday)
            (2025, 10, 31), // October 31 (Friday)
            (2025, 11, 28), // November 28 (Friday) - 30th is Sunday
            (2025, 12, 31), // December 31 (Wednesday)
        ];

        let mut current = start_date;
        for (i, (year, month, day)) in expected.iter().enumerate() {
            match cron.find_next_occurrence(&current, false) {
                Ok(next) => {
                    assert_eq!(next.year(), *year, "Year mismatch at index {}", i);
                    assert_eq!(next.month(), *month, "Month mismatch at index {}", i);
                    assert_eq!(next.day(), *day, "Day mismatch at index {}", i);
                    current = next;
                }
                Err(e) => panic!("Error finding occurrence at index {}: {:?}", i, e),
            }
        }
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
        assert!(
            !cron.is_time_matching(&a_monday_not_first).unwrap(),
            "Should not match a Monday that is not the 1st in AND mode."
        );
    }

    #[test]
    fn test_plus_modifier_invalid_field() {
        // Using '+' in the day-of-month field should result in an error.
        let result = custom_parse("0 0 +1 * *", false);
        assert!(matches!(
            result,
            Err(croner::errors::CronError::IllegalCharacters(_))
        ));
    }

    #[test]
    fn test_invalid_step_syntax_single_number() {
        // Only */Z and X-Y/Z should be allowed, not X/Z
        // Test 0/10 in minute field
        let result = Cron::from_str("0/10 * * * *");
        assert!(
            result.is_err(),
            "Pattern '0/10 * * * *' should be rejected (invalid step syntax)"
        );

        // Test 30/10 in minute field
        let result = Cron::from_str("30/10 * * * *");
        assert!(
            result.is_err(),
            "Pattern '30/10 * * * *' should be rejected (invalid step syntax)"
        );

        // Test 5/15 in hour field
        let result = Cron::from_str("* 5/15 * * *");
        assert!(
            result.is_err(),
            "Pattern '* 5/15 * * *' should be rejected (invalid step syntax)"
        );

        // Test 15/5 in day-of-month field
        let result = Cron::from_str("* * 15/5 * *");
        assert!(
            result.is_err(),
            "Pattern '* * 15/5 * *' should be rejected (invalid step syntax)"
        );

        // Test 6/2 in month field
        let result = Cron::from_str("* * * 6/2 *");
        assert!(
            result.is_err(),
            "Pattern '* * * 6/2 *' should be rejected (invalid step syntax)"
        );

        // Test 1/2 in day-of-week field
        let result = Cron::from_str("* * * * 1/2");
        assert!(
            result.is_err(),
            "Pattern '* * * * 1/2' should be rejected (invalid step syntax)"
        );

        // Test /10 in minute field (omitting starting point)
        let result = Cron::from_str("/10 * * * *");
        assert!(
            result.is_err(),
            "Pattern '/10 * * * *' should be rejected (omitting starting point)"
        );

        // Test /5 in hour field
        let result = Cron::from_str("* /5 * * *");
        assert!(
            result.is_err(),
            "Pattern '* /5 * * *' should be rejected (omitting starting point)"
        );
    }

    #[test]
    fn test_valid_step_syntax() {
        // Verify that */Z syntax is still valid
        assert!(
            Cron::from_str("*/10 * * * *").is_ok(),
            "Pattern '*/10 * * * *' should be accepted (valid wildcard step)"
        );

        // Verify that X-Y/Z syntax is still valid
        assert!(
            Cron::from_str("0-30/10 * * * *").is_ok(),
            "Pattern '0-30/10 * * * *' should be accepted (valid range step)"
        );

        assert!(
            Cron::from_str("10-50/5 * * * *").is_ok(),
            "Pattern '10-50/5 * * * *' should be accepted (valid range step)"
        );
    }
}
