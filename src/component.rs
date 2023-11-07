use crate::errors::CronError;

pub const MAX_SIZE: usize = 60; // Maximum size for the actual values (0-59)

// Constants for special flags
pub const NONE_BIT: u64 = 0;
pub const LAST_BIT: u64 = 1 << 60; // Up to tree additional special bits can be added with 1 << 61..

#[derive(Debug, Default)]
pub struct CronComponent {
    bitfield: u64, // Single u64 to act as a bitfield
    pub min: u8,   // Minimum value this component can take
    pub max: u8,   // Maximum value this component can take
    features: u64, // Single u64 to indicate supported special bits, like LAST_BIT
}

impl CronComponent {
    // Initialize a new CronComponent with min/max values.
    pub fn new(min: u8, max: u8, features: u64) -> Self {
        Self {
            bitfield: 0, // Initialize all bits to 0
            min,
            max,
            features: features,
        }
    }

    // Set a bit at a given position (0 to 59)
    pub fn set_bit(&mut self, pos: u8) -> Result<(), CronError> {
        if pos < self.min || pos > self.max {
            return Err(CronError::ComponentError(format!(
                "Bit position {} is out of bounds.",
                pos
            )));
        }
        self.bitfield |= 1 << pos;
        Ok(())
    }

    // Unset a bit at a given position (0 to 59)
    pub fn unset_bit(&mut self, pos: u8) -> Result<(), CronError> {
        if pos < self.min || pos > self.max {
            return Err(CronError::ComponentError(format!(
                "Bit position {} is out of bounds.",
                pos
            )));
        }
        self.bitfield &= !(1 << pos);
        Ok(())
    }

    // Check if a bit at a given position is set
    pub fn is_bit_set(&self, pos: u8) -> bool {
        if usize::from(pos) < MAX_SIZE {
            (self.bitfield & (1 << pos)) != 0
        } else {
            false
        }
    }

    // Check if a special bit is set
    pub fn is_special_bit_set(&self, flag: u64) -> bool {
        (self.bitfield & flag) != 0
    }

    // Set or clear a special bit if it is supported
    pub fn set_special_bit(&mut self, flag: u64, set: bool) -> Result<(), CronError> {
        // Check if the bit is within the supported features
        if self.features & flag == 0 {
            return Err(CronError::UnsupportedSpecialBit);
        }

        if set {
            self.bitfield |= flag;
        } else {
            self.bitfield &= !flag;
        }
        Ok(())
    }

    pub fn parse(&mut self, field: &str) -> Result<(), CronError> {
        if field == "*" {
            for value in self.min..=self.max {
                self.set_bit(value)?;
            }
        } else {
            // Split the field into parts and handle each part
            for part in field.split(',') {
                let trimmed_part = part.trim();
                if !trimmed_part.is_empty() {
                    if trimmed_part.contains('/') {
                        self.handle_stepping(trimmed_part)?;
                    } else if trimmed_part.contains('-') {
                        self.handle_range(trimmed_part)?;
                    } else if trimmed_part.eq_ignore_ascii_case("l") {
                        // Handle "L" for the last bit
                        self.set_special_bit(LAST_BIT, true)?;
                    } else {
                        self.handle_number(trimmed_part)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_range(&mut self, range: &str) -> Result<(), CronError> {
        let parts: Vec<&str> = range.split('-').map(str::trim).collect();
        if parts.len() != 2 {
            return Err(CronError::ComponentError(
                "Invalid range syntax.".to_string(),
            ));
        }

        let start = parts[0]
            .parse::<u8>()
            .map_err(|_| CronError::ComponentError("Invalid start of range.".to_string()))?;
        let end = parts[1]
            .parse::<u8>()
            .map_err(|_| CronError::ComponentError("Invalid end of range.".to_string()))?;

        if start > end || start < self.min || end > self.max {
            return Err(CronError::ComponentError(
                "Range out of bounds.".to_string(),
            ));
        }

        // Calculate the bitmask for the range in one operation
        let mask: u64 = ((1 << (end - start + 1)) - 1) << start;
        self.bitfield |= mask;

        Ok(())
    }

    fn handle_number(&mut self, value: &str) -> Result<(), CronError> {
        let num = value
            .parse::<u8>()
            .map_err(|_| CronError::ComponentError("Invalid number.".to_string()))?;
        if num < self.min || num > self.max {
            return Err(CronError::ComponentError(
                "Number out of bounds.".to_string(),
            ));
        }

        self.set_bit(num)?;
        Ok(())
    }

    pub fn handle_stepping(&mut self, stepped_range: &str) -> Result<(), CronError> {
        let parts: Vec<&str> = stepped_range.split('/').collect();
        if parts.len() != 2 {
            return Err(CronError::ComponentError(
                "Invalid stepped range syntax.".to_string(),
            ));
        }

        let range_part = parts[0];
        let step_str = parts[1];
        let step = step_str
            .parse::<u8>()
            .map_err(|_| CronError::ComponentError("Invalid step.".to_string()))?;
        if step == 0 {
            return Err(CronError::ComponentError(
                "Step cannot be zero.".to_string(),
            ));
        }

        let (start, end) = if range_part == "*" {
            (self.min, self.max)
        } else if range_part.contains('-') {
            let bounds: Vec<&str> = range_part.split('-').collect();
            if bounds.len() != 2 {
                return Err(CronError::ComponentError(
                    "Invalid range syntax in stepping.".to_string(),
                ));
            }
            (
                bounds[0]
                    .parse::<u8>()
                    .map_err(|_| CronError::ComponentError("Invalid range start.".to_string()))?,
                bounds[1]
                    .parse::<u8>()
                    .map_err(|_| CronError::ComponentError("Invalid range end.".to_string()))?,
            )
        } else {
            let single_start = range_part
                .parse::<u8>()
                .map_err(|_| CronError::ComponentError("Invalid start.".to_string()))?;
            // If only one number is provided, set the range to go from the start value to the max value.
            (single_start, self.max)
        };

        if start < self.min || end > self.max || start > end {
            return Err(CronError::ComponentError(
                "Range is out of bounds in stepping.".to_string(),
            ));
        }

        // Apply stepping within the range
        let mut value = start;
        while value <= end {
            self.set_bit(value)?;
            value = value.checked_add(step).ok_or_else(|| {
                CronError::ComponentError("Value exceeded max after stepping.".to_string())
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cron_component() {
        let component = CronComponent::new(0, 59, NONE_BIT);
        assert_eq!(component.min, 0);
        assert_eq!(component.max, 59);
        assert_eq!(component.bitfield, 0);
    }

    #[test]
    fn test_set_bit() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        assert!(component.set_bit(10).is_ok());
        assert!(component.is_bit_set(10));
    }

    #[test]
    fn test_set_bit_out_of_bounds() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        assert!(component.set_bit(60).is_err());
    }

    #[test]
    fn test_unset_bit() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        component.set_bit(10).unwrap();
        assert!(component.unset_bit(10).is_ok());
        assert!(!component.is_bit_set(10));
    }

    #[test]
    fn test_is_special_bit_set() {
        let mut component = CronComponent::new(0, 59, LAST_BIT);
        assert!(!component.is_special_bit_set(LAST_BIT));
        component.set_special_bit(LAST_BIT, true).unwrap();
        assert!(component.is_special_bit_set(LAST_BIT));
    }

    #[test]
    fn test_set_special_bit_unsupported() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        assert!(component.set_special_bit(LAST_BIT, true).is_err());
    }

    #[test]
    fn test_parse_asterisk() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        component.parse("*").unwrap();
        for i in 0..=59 {
            assert!(component.is_bit_set(i));
        }
    }

    #[test]
    fn test_parse_range() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        component.parse("10-15").unwrap();
        for i in 10..=15 {
            assert!(component.is_bit_set(i));
        }
    }

    #[test]
    fn test_parse_stepping() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        component.parse("*/5").unwrap();
        for i in (0..=59).filter(|n| n % 5 == 0) {
            assert!(component.is_bit_set(i));
        }
    }

    #[test]
    fn test_parse_list() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        component.parse("5,10,15").unwrap();
        for i in [5, 10, 15].iter() {
            assert!(component.is_bit_set(*i));
        }
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        assert!(component.parse("10-").is_err());
        assert!(component.parse("*/").is_err());
        assert!(component.parse("60").is_err()); // out of bounds for the minute field
    }

    #[test]
    fn test_parse_in_range_week() {
        let mut component = CronComponent::new(0, 6, NONE_BIT);
        assert!(component.parse("6").is_ok());
    }

    #[test]
    fn test_parse_out_of_range_week() {
        let mut component = CronComponent::new(0, 6, NONE_BIT);
        assert!(component.parse("7").is_err());
    }

    #[test]
    fn test_parse_stepping_from_nine() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        component.parse("9/5").unwrap();

        let mut value = 9;
        while value <= component.max {
            assert!(
                component.is_bit_set(value),
                "Bit should be set for value {}",
                value
            );
            value += 5;
        }

        // Ensure no other bits are set
        for i in 0..=59 {
            if i < 9 || (i - 9) % 5 != 0 {
                assert!(
                    !component.is_bit_set(i),
                    "Bit should not be set for value {}",
                    i
                );
            }
        }
    }
}
