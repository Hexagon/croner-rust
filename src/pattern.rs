use crate::component::{CronComponent, LAST_BIT, NONE_BIT};
use crate::errors::CronError;

// This struct is used for representing and validating cron pattern strings.
// It supports parsing cron patterns with optional seconds field and provides functionality to check pattern matching against specific datetime.
#[derive(Debug)]
pub struct CronPattern {
    pattern: String, // The original pattern
    //
    pub seconds: CronComponent,      // -
    pub minutes: CronComponent,      // --
    pub hours: CronComponent,        // --- Each individual part of the cron expression
    pub days: CronComponent,         // --- represented by a bitmask, min and max value
    pub months: CronComponent,       // --
    pub days_of_week: CronComponent, // -
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
            days_of_week: CronComponent::new(0, 6, NONE_BIT), // Actually 0-7 in pattern, but 7 is converted to 0
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

        if self.pattern.contains('@') {
            self.pattern = Self::handle_nicknames(&self.pattern).trim().to_string();
        }

        let mut parts: Vec<&str> = self.pattern.split_whitespace().collect();
        if parts.len() < 5 || parts.len() > 6 {
            return Err(CronError::InvalidPattern(String::from("Pattern must consist of five or six fields (minute, hour, day, month, day of week, and optional second).")));
        }

        if parts.len() == 5 {
            parts.insert(0, "0"); // prepend "0" if the seconds part is missing
        }

        self.seconds.parse(parts[0])?;
        self.minutes.parse(parts[1])?;
        self.hours.parse(parts[2])?;
        self.days.parse(parts[3])?;
        self.months.parse(parts[4])?;
        self.days_of_week.parse(parts[5])?;

        // Handle conversion of 7 to 0 for day_of_week if necessary
        if self.days_of_week.is_bit_set(7) {
            self.days_of_week.unset_bit(7)?;
            self.days_of_week.set_bit(0)?;
        }

        Ok(())
    }

    // Validates that the cron pattern only contains legal characters for each field.
    pub fn throw_at_illegal_characters(&self, parts: &[&str]) -> Result<(), CronError> {
        // Base allowed characters for most fields
        let base_allowed_characters = [
            '*', '/', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ',', '-',
        ];
        // Additional characters allowed for the day-of-week field
        let day_of_week_allowed_characters = [
            '*', '/', '0', '1', '2', '3', '4', '5', '6', '7', '9', ',', '-', '#', 'L',
        ];
        // Additional characters allowed for the day-of-month field
        let day_of_month_allowed_characters = [
            '*', '/', '0', '1', '2', '3', '4', '5', '6', '7', '9', ',', '-', 'L',
        ];

        for (i, part) in parts.iter().enumerate() {
            // Decide which set of allowed characters to use
            let allowed = if i == 5 {
                &day_of_week_allowed_characters[..]
            } else if i == 3 {
                &day_of_month_allowed_characters[..]
            } else {
                &base_allowed_characters[..]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_pattern_new() {
        let pattern = CronPattern::new("* */5 * * * *").unwrap();
        assert_eq!(pattern.pattern, "* */5 * * * *");
        assert!(pattern.seconds.is_bit_set(5));
    }

    #[test]
    fn test_cron_pattern_short() {
        let pattern = CronPattern::new("5/5 * * * *").unwrap();
        assert_eq!(pattern.pattern, "5/5 * * * *");
        assert!(pattern.seconds.is_bit_set(0));
        assert!(!pattern.seconds.is_bit_set(5));
        assert!(pattern.minutes.is_bit_set(5));
        assert!(!pattern.minutes.is_bit_set(0));
    }

    #[test]
    fn test_cron_pattern_parse() {
        let mut pattern = CronPattern::new("*/15 1 1,15 1 1-5").unwrap();
        assert!(pattern.parse().is_ok());
        assert!(pattern.minutes.is_bit_set(0));
        assert!(pattern.hours.is_bit_set(1));
        assert!(pattern.days.is_bit_set(1) && pattern.days.is_bit_set(15));
        assert!(
            pattern.months.is_bit_set(1)
                && !pattern.months.is_bit_set(2)
                && !pattern.months.is_bit_set(0)
        );
        assert!(pattern.days_of_week.is_bit_set(1) && pattern.days_of_week.is_bit_set(5));
    }

    #[test]
    fn test_cron_pattern_extra_whitespace() {
        let mut pattern = CronPattern::new("  */15  1 1,15 1    1-5    ").unwrap();
        assert!(pattern.parse().is_ok());
        assert!(pattern.minutes.is_bit_set(0));
        assert!(pattern.hours.is_bit_set(1));
        assert!(pattern.days.is_bit_set(1) && pattern.days.is_bit_set(15));
        assert!(
            pattern.months.is_bit_set(1)
                && !pattern.months.is_bit_set(2)
                && !pattern.months.is_bit_set(0)
        );
        assert!(pattern.days_of_week.is_bit_set(1) && pattern.days_of_week.is_bit_set(5));
    }

    #[test]
    fn test_cron_pattern_handle_nicknames() {
        assert_eq!(CronPattern::handle_nicknames("@yearly"), "0 0 1 1 *");
        assert_eq!(CronPattern::handle_nicknames("@monthly"), "0 0 1 * *");
        assert_eq!(CronPattern::handle_nicknames("@weekly"), "0 0 * * 0");
        assert_eq!(CronPattern::handle_nicknames("@daily"), "0 0 * * *");
        assert_eq!(CronPattern::handle_nicknames("@hourly"), "0 * * * *");
    }
}
