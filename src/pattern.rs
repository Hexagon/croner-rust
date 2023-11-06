// use chrono::{DateTime, Datelike, Local, Timelike, TimeZone};

// This struct is used for representing and validating cron pattern strings.
// It supports parsing cron patterns with optional seconds field and provides functionality to check pattern matching against specific datetime.
#[derive(Debug)]
pub struct CronPattern {
    pattern: String,
    second: Vec<u32>,
    minute: Vec<u32>,
    hour: Vec<u32>,
    day: Vec<u32>,
    month: Vec<u32>,
    day_of_week: Vec<u32>,
    last_day_of_month: bool,
}

// Implementation block for CronPattern struct, providing methods for creating and parsing cron pattern strings.
impl CronPattern {
    pub fn new(pattern: &str) -> Result<Self, &'static str> {
        let mut cron_pattern = CronPattern {
            pattern: pattern.to_string(),
            second: Vec::new(),
            minute: Vec::new(),
            hour: Vec::new(),
            day: Vec::new(),
            month: Vec::new(),
            day_of_week: Vec::new(),
            last_day_of_month: false,
        };
        cron_pattern.parse()?;
        Ok(cron_pattern)
    }

    // Parses the cron pattern string into its respective fields.
    // Handles optional seconds field, named shortcuts, and determines if 'L' flag is used for last day of the month.
    pub fn parse(&mut self) -> Result<(), &'static str> {
        if self.pattern.trim().is_empty() {
            return Err("CronPattern: Pattern cannot be an empty string.");
        }

        if self.pattern.contains('@') {
            self.pattern = Self::handle_nicknames(&self.pattern).trim().to_string();
        }

        let mut parts: Vec<&str> = self.pattern.split_whitespace().map(|s| s.trim()).collect();

        if parts.len() < 5 || parts.len() > 6 {
            return Err("Pattern must consist of five or six fields (minute, hour, day, month, day of week, and optional second).");
        }

        if parts.len() == 5 {
            parts.insert(0, "0"); // prepend "0" if the seconds part is missing
        }

        let mut parts_owned: Vec<String> = Vec::new();
        for (_i, part) in parts.iter().enumerate() {
            // Convert 'L' to lastDayOfMonth flag in day-of-month field
            if *part == parts[3] && part.contains('L') {
                self.last_day_of_month = true;
                parts_owned.push(part.replace('L', ""));
            } else {
                parts_owned.push(part.to_string());
            }
        }

        // Convert Vec<String> back to Vec<&str> for the parts without 'L'
        let parts_slices: Vec<&str> = parts.iter().map(|&s| s).collect();

        self.throw_at_illegal_characters(&parts_slices)?;

        // Since parts_owned is Vec<String>, we need to convert them back to slices when passing to parse_field.
        self.second = Self::parse_field(&parts_owned[0], 0, 59)?;
        self.minute = Self::parse_field(&parts_owned[1], 0, 59)?;
        self.hour = Self::parse_field(&parts_owned[2], 0, 23)?;
        self.day = Self::parse_field(&parts_owned[3], 1, 31)?;
        self.month = Self::parse_field(&parts_owned[4], 1, 12)?;
        self.day_of_week = Self::parse_field(&parts_owned[5], 0, 7)?;

        // Handle conversion of 7 to 0 for day_of_week if necessary
        if self.day_of_week.contains(&7) {
            self.day_of_week = self
                .day_of_week
                .iter()
                .map(|&d| if d == 7 { 0 } else { d })
                .collect();
        }

        Ok(())
    }

    // Validates that the cron pattern only contains legal characters for each field.
    pub fn throw_at_illegal_characters(&self, parts: &[&str]) -> Result<(), &'static str> {
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
                    return Err("CronPattern contains illegal characters.");
                }
            }
        }

        Ok(())
    }

    fn parse_field(field: &str, min: u32, max: u32) -> Result<Vec<u32>, &'static str> {
        let mut results = Vec::new();

        if field == "*" {
            return Ok((min..=max).collect());
        }

        for part in field.split(',') {
            if !part.is_empty() {
                let sub_result = if part.contains('/') {
                    Self::handle_stepping(part, min, max)?
                } else if part.contains('-') {
                    Self::handle_range(part, min, max)?
                } else {
                    Self::handle_number(part, min, max)?
                };
                results.extend(sub_result);
            }
        }

        Ok(results)
    }

    fn handle_range(range: &str, min: u32, max: u32) -> Result<Vec<u32>, &'static str> {
        let parts: Vec<&str> = range.split('-').collect();
        if parts.len() != 2 {
            return Err("Invalid range syntax.");
        }

        let start = parts[0]
            .parse::<u32>()
            .map_err(|_| "Invalid start of range.")?;
        let end = parts[1]
            .parse::<u32>()
            .map_err(|_| "Invalid end of range.")?;
        if start > end || start < min || end > max {
            return Err("Range out of bounds.");
        }

        Ok((start..=end).collect())
    }

    fn handle_stepping(stepped_range: &str, min: u32, max: u32) -> Result<Vec<u32>, &'static str> {
        let parts: Vec<&str> = stepped_range.split('/').collect();
        if parts.len() != 2 {
            return Err("Invalid stepped range syntax.");
        }

        let range_part = parts[0];
        let step_str = parts[1];
        let step = step_str.parse::<usize>().map_err(|_| "Invalid step.")?;
        if step == 0 {
            return Err("Step cannot be zero.");
        }

        let mut range = Vec::new();

        if range_part == "*" {
            range.extend(min..=max);
        } else if range_part.contains('-') {
            let bounds: Vec<&str> = range_part.split('-').collect();
            if bounds.len() != 2 {
                return Err("Invalid range syntax in stepping.");
            }

            let start = bounds[0]
                .parse::<u32>()
                .map_err(|_| "Invalid range start.")?;
            let end = bounds[1].parse::<u32>().map_err(|_| "Invalid range end.")?;

            if start < min || end > max || start > end {
                return Err("Range is out of bounds in stepping.");
            }

            range.extend(start..=end);
        } else {
            let start = range_part.parse::<u32>().map_err(|_| "Invalid start.")?;
            if start < min || start > max {
                return Err("Start is out of range in stepping.");
            }
            range.push(start);
        }

        // Apply stepping
        let stepped_range = range
            .into_iter()
            .enumerate()
            .filter_map(|(i, value)| if i % step == 0 { Some(value) } else { None })
            .collect::<Vec<u32>>();

        Ok(stepped_range)
    }

    fn handle_number(value: &str, min: u32, max: u32) -> Result<Vec<u32>, &'static str> {
        let num = value.parse::<u32>().map_err(|_| "Invalid number.")?;
        if num < min || num > max {
            return Err("Number out of bounds.");
        }

        Ok(vec![num])
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

    /*
    // Checks if the provided datetime matches the cron pattern fields.
    // This function takes into account seconds, minutes, hours, days, months, and day of the week, as well as the 'L' flag for last day of the month.
    pub fn is_time_matching(&self, time: &DateTime<Local>) -> bool {
        self.second.contains(&(time.second() as u32)) &&
        self.minute.contains(&(time.minute() as u32)) &&
        self.hour.contains(&(time.hour() as u32)) &&
        (self.day.contains(&(time.day() as u32)) || self.last_day_of_month && time.day() == last_day_of_month(time.year(), time.month())) &&
        self.month.contains(&(time.month() as u32)) &&
        self.day_of_week.contains(&(time.weekday().number_from_sunday() as u32 - 1))
    }
    */
}

// Helper function to find the last day of a given month
/*fn last_day_of_month(year: i32, month: u32) -> u32 {
    chrono::Utc.ymd(year, month + 1, 1)
        .pred()
        .day()
}*/

// Unit tests for the CronPattern struct to ensure correct behavior across various cron pattern strings and scenarios.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_pattern_new() {
        let pattern = "* */15 0 1,5 * 1-5";
        let cron_pattern = CronPattern::new(pattern);
        assert!(cron_pattern.is_ok());
    }

    #[test]
    fn test_invalid_cron_pattern() {
        let pattern = "*/15";
        let cron_pattern = CronPattern::new(pattern);
        assert!(cron_pattern.is_err());
    }

    #[test]
    fn test_parse_field_star() {
        let result = CronPattern::parse_field("*", 0, 59);
        assert_eq!(result.unwrap(), (0..=59).collect::<Vec<u32>>());
    }

    #[test]
    fn test_parse_field_range() {
        let result = CronPattern::parse_field("5-10", 0, 59);
        assert_eq!(result.unwrap(), vec![5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_parse_field_step() {
        let result = CronPattern::parse_field("*/15", 0, 59);
        assert_eq!(result.unwrap(), vec![0, 15, 30, 45]);
    }

    #[test]
    fn test_parse_field_single() {
        let result = CronPattern::parse_field("30", 0, 59);
        assert_eq!(result.unwrap(), vec![30]);
    }

    #[test]
    fn test_parse_field_invalid() {
        let result = CronPattern::parse_field("60", 0, 59);
        assert!(result.is_err());
    }

    /*#[test]
    fn test_is_time_matching() {
        let pattern = "0 30 8 15 7 *";
        let cron_pattern = CronPattern::new(pattern).unwrap();
        let time = Local.ymd(2023, 7, 15).and_hms(8, 30, 0);
        assert!(cron_pattern.is_time_matching(&time));
    }*/
}
