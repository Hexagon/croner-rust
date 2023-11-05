//use chrono::{DateTime, Datelike, Local, Timelike, TimeZone};

// A struct to hold our cron pattern
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

    pub fn parse(&mut self) -> Result<(), &'static str> {
        let parts: Vec<&str> = self.pattern.split_whitespace().collect();

        if parts.len() != 6 {
            return Err("Pattern must consist of six fields (second, minute, hour, day, month, day of week).");
        }

        self.second = Self::parse_field(parts[0], 0, 59)?;
        self.minute = Self::parse_field(parts[1], 0, 59)?;
        self.hour = Self::parse_field(parts[2], 0, 23)?;
        self.day = Self::parse_field(parts[3], 1, 31)?;
        self.month = Self::parse_field(parts[4], 1, 12)?;
        self.day_of_week = Self::parse_field(parts[5], 0, 6)?;
        self.last_day_of_month = parts[3] == "L";

        Ok(())
    }

    fn parse_field(field: &str, min: u32, max: u32) -> Result<Vec<u32>, &'static str> {
        let mut results = Vec::new();
        
        if field == "*" {
            return Ok((min..=max).collect());
        }

        for part in field.split(',') {
            let sub_result = if part.contains('-') {
                Self::parse_range(part, min, max)?
            } else if part.contains('/') {
                Self::parse_stepped_range(part, min, max)?
            } else {
                Self::parse_single_value(part, min, max)?
            };
            results.extend(sub_result);
        }

        Ok(results)
    }

    fn parse_range(range: &str, min: u32, max: u32) -> Result<Vec<u32>, &'static str> {
        let parts: Vec<&str> = range.split('-').collect();
        if parts.len() != 2 {
            return Err("Invalid range syntax.");
        }

        let start = parts[0].parse::<u32>().map_err(|_| "Invalid start of range.")?;
        let end = parts[1].parse::<u32>().map_err(|_| "Invalid end of range.")?;
        if start > end || start < min || end > max {
            return Err("Range out of bounds.");
        }

        Ok((start..=end).collect())
    }

    fn parse_stepped_range(stepped_range: &str, min: u32, max: u32) -> Result<Vec<u32>, &'static str> {
        let parts: Vec<&str> = stepped_range.split('/').collect();
        if parts.len() != 2 {
            return Err("Invalid stepped range syntax.");
        }

        let range = parts[0];
        let step_str = parts[1];
        let step = step_str.parse::<usize>().map_err(|_| "Invalid step.")?;
        if step == 0 {
            return Err("Step cannot be zero.");
        }

        let numbers = if range == "*" {
            (min..=max).collect::<Vec<u32>>()
        } else {
            Self::parse_range(range, min, max)?
        };

        Ok(numbers.into_iter().step_by(step).collect())
    }

    fn parse_single_value(value: &str, min: u32, max: u32) -> Result<Vec<u32>, &'static str> {
        let num = value.parse::<u32>().map_err(|_| "Invalid number.")?;
        if num < min || num > max {
            return Err("Number out of bounds.");
        }

        Ok(vec![num])
    }

    /*
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