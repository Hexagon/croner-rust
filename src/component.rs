use crate::errors::CronError;

// Constants for flags
pub const NONE_BIT: u8 = 0;
pub const ALL_BIT: u8 = 1;

// Used for nth weekday
pub const NTH_1ST_BIT: u8 = 1 << 1;
pub const NTH_2ND_BIT: u8 = 1 << 2;
pub const NTH_3RD_BIT: u8 = 1 << 3;
pub const NTH_4TH_BIT: u8 = 1 << 4;
pub const NTH_5TH_BIT: u8 = 1 << 5;
pub const NTH_ALL: u8 = NTH_1ST_BIT | NTH_2ND_BIT | NTH_3RD_BIT | NTH_4TH_BIT | NTH_5TH_BIT;

// Used for last day of month
pub const LAST_BIT: u8 = 1 << 6;

#[derive(Debug, Default)]
pub struct CronComponent {
    bitfields: Vec<u8>, // Vector of u8 to act as multiple bitfields
    pub min: u8,        // Minimum value this component can take
    pub max: u8,        // Maximum value this component can take
    features: u8,       // Single u8 bitfield to indicate supported special bits, like LAST_BIT
    enabled_features: u8,        // Bitfield to hold component-wide special bits like LAST_BIT
}

impl CronComponent {
    // Initialize a new CronComponent with min/max values and features.
    pub fn new(min: u8, max: u8, features: u8) -> Self {
        Self {
            bitfields: vec![NONE_BIT; (max + 1) as usize], // Initialize bitfields with NONE_BIT for each element.
            min,
            max,
            features: features | ALL_BIT | LAST_BIT, // Store the features bitfield, always allow NONE and LAST
            enabled_features: 0,
        }
    }

    // Set a bit at a given position (0 to 59)
    pub fn set_bit(&mut self, pos: u8, bit: u8) -> Result<(), CronError> {
        if pos < self.min || pos > self.max {
            return Err(CronError::ComponentError(format!(
                "Position {} is out of bounds for the current range ({}-{}).",
                pos, self.min, self.max
            )));
        }
        if self.features & bit != bit {
            return Err(CronError::ComponentError(format!(
                "Bit 0b{:08b} is not supported by the current features 0b{:08b}.",
                bit, self.features
            )));
        }
        let index = pos as usize; // Convert the position to an index
        if index >= self.bitfields.len() {
            // In case the index is somehow out of the vector's bounds
            return Err(CronError::ComponentError(format!(
                "Position {} is out of the bitfields vector's bounds.",
                pos
            )));
        }
        self.bitfields[index] |= bit; // Set the specific bit at the position
        Ok(())
    }


    // Unset a specific bit at a given position
    pub fn unset_bit(&mut self, pos: u8, bit: u8) -> Result<(), CronError> {
        if pos < self.min || pos > self.max {
            return Err(CronError::ComponentError(format!(
                "Position {} is out of bounds for the current range ({}-{}).",
                pos, self.min, self.max
            )));
        }
        if self.features & bit != bit {
            return Err(CronError::ComponentError(format!(
                "Bit 0b{:08b} is not supported by the current features 0b{:08b}.",
                bit, self.features
            )));
        }
        let index = pos as usize; // Convert the position to an index
        if index >= self.bitfields.len() {
            // In case the index is somehow out of the vector's bounds
            return Err(CronError::ComponentError(format!(
                "Position {} is out of the bitfields vector's bounds.",
                pos
            )));
        }
        self.bitfields[index] &= !bit; // Unset the specific bit at the position
        Ok(())
    }

    // Check if a specific bit at a given position is set
    pub fn is_bit_set(&self, pos: u8, bit: u8) -> Result<bool, CronError> {
        if pos < self.min || pos > self.max {
            Err(CronError::ComponentError(format!(
                "Position {} is out of bounds for the current range ({}-{}).",
                pos, self.min, self.max
            )))
        } else if self.features & bit != bit {
            Err(CronError::ComponentError(format!(
                "Bit 0b{:08b} is not supported by the current features 0b{:08b}.",
                bit, self.features
            )))
        } else {
            let index = pos as usize;
            if index >= self.bitfields.len() {
                Err(CronError::ComponentError(format!(
                    "Position {} is out of the bitfields vector's bounds.",
                    pos
                )))
            } else {
                Ok((self.bitfields[index] & bit) != 0)
            }
        }
    }

    // Method to enable a feature
    pub fn enable_feature(&mut self, feature: u8) -> Result<(), CronError> {
        if self.features & feature == feature {
            self.enabled_features |= feature;
            Ok(())
        } else {
            Err(CronError::ComponentError(format!(
                "Feature 0b{:08b} is not supported by the current features 0b{:08b}.",
                feature, self.features
            )))
        }
    }

    // Method to check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: u8) -> bool {
        (self.enabled_features & feature) == feature
    }
    
    pub fn parse(&mut self, field: &str) -> Result<(), CronError> {
        if field == "*" {
            for value in self.min..=self.max {
                self.set_bit(value, ALL_BIT)?;
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
                        self.enable_feature(LAST_BIT)?;
                    } else {
                        self.handle_number(trimmed_part)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn get_nth_bit(value: &str) -> Result<u8, CronError> {
        if let Some(nth_pos) = value.find('#') {
            // If value ends with 'L', we set the LAST_BIT and exit early
            if value.ends_with('L') || value.ends_with('l') {
                return Ok(LAST_BIT);
            }
            let nth = value[nth_pos+1..].parse::<u8>()
                .map_err(|_| CronError::ComponentError("Invalid nth specifier.".to_string()))?;

            if nth == 0 || nth > 5 {
                Err(CronError::ComponentError("Nth specifier out of bounds.".to_string()))
            } else {
                match nth {
                    1 => Ok(NTH_1ST_BIT),
                    2 => Ok(NTH_2ND_BIT),
                    3 => Ok(NTH_3RD_BIT),
                    4 => Ok(NTH_4TH_BIT),
                    5 => Ok(NTH_5TH_BIT),
                    _ => Err(CronError::ComponentError("Invalid nth specifier.".to_string())),
                }
            }
        } else {
            Ok(ALL_BIT)
        }
    }

    fn strip_nth_part(value: &str) -> &str {
        value.split('#').next().unwrap_or("")
    }

    fn handle_range(&mut self, range: &str) -> Result<(), CronError> {

        let bit_to_set = CronComponent::get_nth_bit(range)?;
        let str_clean = CronComponent::strip_nth_part(range);

        let parts: Vec<&str> = str_clean.split('-').map(str::trim).collect();
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

        for value in start..=end {
            self.set_bit(value, bit_to_set)?;
        }
        Ok(())
    }

    fn handle_number(&mut self, value: &str) -> Result<(), CronError> {
        
        let bit_to_set = CronComponent::get_nth_bit(value)?;
        let value_clean = CronComponent::strip_nth_part(value);

        let num = value_clean
            .parse::<u8>()
            .map_err(|_| CronError::ComponentError("Invalid number.".to_string()))?;
        if num < self.min || num > self.max {
            return Err(CronError::ComponentError(
                "Number out of bounds.".to_string(),
            ));
        }

        self.set_bit(num, bit_to_set)?; 
        Ok(())
    }

    pub fn handle_stepping(&mut self, stepped_range: &str) -> Result<(), CronError> {
        
        let bit_to_set = CronComponent::get_nth_bit(stepped_range)?;
        let stepped_range_clean = CronComponent::strip_nth_part(stepped_range);

        let parts: Vec<&str> = stepped_range_clean.split('/').collect();
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
            self.set_bit(value, bit_to_set)?;
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
    use crate::errors::CronError;

    #[test]
    fn test_new_cron_component() {
        let component = CronComponent::new(0, 59, ALL_BIT | LAST_BIT);
        assert_eq!(component.min, 0);
        assert_eq!(component.max, 59);
        // Ensure all bitfields are initialized to NONE_BIT
        assert!(component.bitfields.iter().all(|&b| b == NONE_BIT));
        // Check that ALL_BIT and LAST_BIT are included in features
        assert!(component.features & (ALL_BIT | LAST_BIT) == (ALL_BIT | LAST_BIT));
    }

    #[test]
    fn test_set_bit() {
        let mut component = CronComponent::new(0, 59, ALL_BIT);
        assert!(component.set_bit(10, ALL_BIT).is_ok());
        assert!(component.is_bit_set(10, ALL_BIT).unwrap());
    }

    #[test]
    fn test_set_bit_out_of_bounds() {
        let mut component = CronComponent::new(0, 59, ALL_BIT);
        assert!(matches!(
            component.set_bit(60, ALL_BIT),
            Err(CronError::ComponentError(_))
        ));
    }

    #[test]
    fn test_unset_bit() {
        let mut component = CronComponent::new(0, 59, ALL_BIT);
        component.set_bit(10, ALL_BIT).unwrap();
        assert!(component.unset_bit(10, ALL_BIT).is_ok());
        assert!(!component.is_bit_set(10, ALL_BIT).unwrap());
    }

    #[test]
    fn test_is_feature_enabled() {
        let mut component = CronComponent::new(0, 59, LAST_BIT);
        assert!(!component.is_feature_enabled(LAST_BIT));
        component.enable_feature(LAST_BIT).unwrap();
        assert!(component.is_feature_enabled(LAST_BIT));
    }

    #[test]
    fn test_enable_feature_unsupported() {
        let mut component = CronComponent::new(0, 59, NONE_BIT);
        assert!(matches!(
            component.enable_feature(NTH_1ST_BIT),
            Err(CronError::ComponentError(_))
        ));
    }

    #[test]
    fn test_parse_asterisk() {
        let mut component = CronComponent::new(0, 59, ALL_BIT);
        component.parse("*").unwrap();
        for i in 0..=59 {
            assert!(component.is_bit_set(i, ALL_BIT).unwrap());
        }
    }

    #[test]
    fn test_parse_range() {
        let mut component = CronComponent::new(0, 59, ALL_BIT);
        component.parse("10-15").unwrap();
        for i in 10..=15 {
            assert!(component.is_bit_set(i, ALL_BIT).unwrap());
        }
    }

    #[test]
    fn test_parse_stepping() {
        let mut component = CronComponent::new(0, 59, ALL_BIT);
        component.parse("*/5").unwrap();
        for i in (0..=59).filter(|n| n % 5 == 0) {
            assert!(component.is_bit_set(i, ALL_BIT).unwrap());
        }
    }

    #[test]
    fn test_parse_list() {
        let mut component = CronComponent::new(0, 59, ALL_BIT);
        component.parse("5,10,15").unwrap();
        for i in [5, 10, 15].iter() {
            assert!(component.is_bit_set(*i, ALL_BIT).unwrap());
        }
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let mut component = CronComponent::new(0, 59, ALL_BIT);
        assert!(component.parse("10-").is_err());
        assert!(component.parse("*/").is_err());
        assert!(component.parse("60").is_err()); // out of bounds for the minute field
    }

}