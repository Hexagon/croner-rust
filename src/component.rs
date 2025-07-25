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

// Used for closest weekday
pub const CLOSEST_WEEKDAY_BIT: u8 = 1 << 7;

// Used for last day of month
pub const LAST_BIT: u8 = 1 << 6;

/// Represents a component of a cron pattern, such as minute, hour, or day of week.
///
/// Each `CronComponent` holds information about permissible values (min, max),
/// features supported (like last day of the month), and specific bits set
/// for scheduling purposes.
///
/// # Examples (for internal use only, CronComponent isn't exported)
///
/// let mut minute_component = CronComponent::new(0, 59, CronComponent::LAST_BIT);
/// // Represents a minute component that supports the 'last' feature.
///
/// // Parsing a cron expression for minute component
/// // This sets specific bits in the component according to the cron syntax
/// minute_component.parse("*/15").expect("Parsing failed");
/// // Sets the minute component to trigger at every 15th minute
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CronComponent {
    bitfields: Vec<u8>,   // Vector of u8 to act as multiple bitfields
    pub min: u16,         // Minimum value this component can take
    pub max: u16,         // Maximum value this component can take
    pub step: u16,        // Steps to skip in this component
    pub from_wildcard: bool, // Wildcard used
    features: u8,         // Single u8 bitfield to indicate supported special bits, like LAST_BIT
    enabled_features: u8, // Bitfield to hold component-wide special bits like LAST_BIT
    input_offset: u16,    // Offset for numerical representation
}

impl CronComponent {
    /// Creates a new `CronComponent` with specified minimum and maximum values and features.
    ///
    /// `min` and `max` define the range of values this component can take.
    /// `features` is a bitfield specifying supported special features.
    ///
    /// # Parameters
    ///
    /// - `min`: The minimum permissible value for this component.
    /// - `max`: The maximum permissible value for this component.
    /// - `features`: Bitfield indicating special features like `LAST_BIT`.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `CronComponent`.
    pub fn new(min: u16, max: u16, features: u8, input_offset: u16) -> Self {
        // Handle the case where max might make usize overflow if not checked
        let bitfields_size = if max > 0 { max as usize + 1 } else { 0 };
        Self {
            // Vector of u8 to act as multiple bitfields.
            // - Initialized with NONE_BIT for each element.
            bitfields: vec![NONE_BIT; bitfields_size],

            // Minimum value this component can take.
            // - Example: 0 for the minute-field
            min,

            // Maximum value this component can take.
            // - Example: 59 for the minute-field
            max,

            // Bitfield to indicate _supported_ special bits, like LAST_BIT.
            // - ALL_BIT and LAST_BIT is always allowed
            features: features | ALL_BIT | LAST_BIT,

            // Bitfield to indicate _enabled_ component-wide special bits like LAST_BIT.
            // - No features are enabled by default
            enabled_features: 0,

            // Offset for numerical representation of weekdays. normally 0=SUN,1=MON etc, setting this to 1 makes 1=SUN...
            input_offset,

            step: 1, // Used by .describe()

            from_wildcard: false, // Used by .describe()
        }
    }

    // Method primarily used by .describe() to evaluate if all bits are set
    pub fn is_all_set(&self) -> bool {
        // A component is "all set" if it's a '*' with no step.
        // We check if all bits in its range are set for the ALL_BIT flag.
        for i in self.min..=self.max {
            if !self.is_bit_set(i, ALL_BIT).unwrap_or(false) {
                return false;
            }
        }
        true
    }
    
    // Set a bit at a given position (e.g., 0 to 9999 for year)
    pub fn set_bit(&mut self, mut pos: u16, bit: u8) -> Result<(), CronError> {
        if pos < self.input_offset {
            return Err(CronError::ComponentError(format!(
                "Position {} is less than the input offset {}.",
                pos, self.input_offset
            )));
        }
        pos -= self.input_offset;
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
                "Position {pos} is out of the bitfields vector's bounds."
            )));
        }
        self.bitfields[index] |= bit; // Set the specific bit at the position
        Ok(())
    }

    // Unset a specific bit at a given position
    pub fn unset_bit(&mut self, mut pos: u16, bit: u8) -> Result<(), CronError> {
        if pos < self.input_offset {
            return Err(CronError::ComponentError(format!(
                "Position {} is less than the input offset {}.",
                pos, self.input_offset
            )));
        }
        pos -= self.input_offset;
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
                "Position {pos} is out of the bitfields vector's bounds."
            )));
        }
        self.bitfields[index] &= !bit; // Unset the specific bit at the position
        Ok(())
    }

    // Check if a specific bit at a given position is set
    pub fn is_bit_set(&self, pos: u16, bit: u8) -> Result<bool, CronError> {
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
                    "Position {pos} is out of the bitfields vector's bounds."
                )))
            } else {
                Ok((self.bitfields[index] & bit) != 0)
            }
        }
    }

    // Method to enable a feature
    pub fn enable_feature(&mut self, feature: u8) -> Result<(), CronError> {
        if self.is_feature_allowed(feature) {
            self.enabled_features |= feature;
            Ok(())
        } else {
            Err(CronError::ComponentError(format!(
                "Feature 0b{:08b} is not supported by the current features 0b{:08b}.",
                feature, self.features
            )))
        }
    }

    pub fn is_feature_allowed(&mut self, feature: u8) -> bool {
        self.features & feature == feature
    }

    // Method to check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: u8) -> bool {
        (self.enabled_features & feature) == feature
    }

    /// Parses a part of a cron expression string and sets the corresponding bits in the component.
    ///
    /// This method interprets the cron syntax provided in `field` and sets
    /// the relevant bits in the component. It supports standard cron patterns
    /// like '*', '-', '/', and 'w'. For example, '*/15' in a minute component
    /// would set the bits for every 15th minute.
    ///
    /// # Parameters
    ///
    /// - `field`: A string slice containing the cron expression part to parse.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if parsing is successful, or `CronError` if the parsing fails.
    ///
    /// # Errors
    ///
    /// Returns `CronError::ComponentError` if the input string contains invalid
    /// cron syntax or values outside the permissible range of the component.
    ///
    /// # Examples (for internal use only, CronComponent isn't exported)
    ///
    /// use crate::component::CronComponent;
    /// let mut hour_component = CronComponent::new(0, 23, 0);
    /// hour_component.parse("*/3").expect("Parsing failed");
    /// // Sets the hour component to trigger at every 3rd hour
    pub fn parse(&mut self, field: &str) -> Result<(), CronError> {
        if field == "*" {
            self.from_wildcard = true;
            for value in self.min..=self.max {
                self.set_bit(value + self.input_offset, ALL_BIT)?;
            }
            return Ok(());
        }

        for part in field.split(',') {
            let trimmed_part = part.trim();
            if trimmed_part.is_empty() {
                continue;
            }

            let mut parsed_part = trimmed_part.to_string();

            if parsed_part.contains('/') {
                self.handle_stepping(&parsed_part)?;
            } else if parsed_part.contains('-') {
                self.handle_range(&parsed_part)?;
            } else if parsed_part.contains('W') {
                self.handle_closest_weekday(&parsed_part)?;
            } else if parsed_part.eq_ignore_ascii_case("L") {
                // Handle "L" for the last bit
                self.enable_feature(LAST_BIT)?;
            } else {
                // If 'L' is contained without '#', like "5L", add the missing '#'
                if parsed_part.ends_with('L') && !parsed_part.contains('#') {
                    parsed_part = parsed_part.replace('L', "#L");
                }

                // If '#' is contained in the number, require feature NTH_ALL to be set
                if parsed_part.contains('#') && !self.is_feature_allowed(NTH_ALL) {
                    return Err(CronError::ComponentError(
                        "Nth specifier # not allowed in the current field.".to_string(),
                    ));
                }

                // If 'L' is contained in the number, require feature NTH_ALL to be set
                if parsed_part.contains('L') && !self.is_feature_allowed(NTH_ALL) {
                    return Err(CronError::ComponentError(
                        "L not allowed in the current field.".to_string(),
                    ));
                }

                self.handle_number(&parsed_part)?;
            }
        }

        Ok(())
    }

    /// Returns a vector of u16 values for all bits set in the component for a given bitflag.
    pub fn get_set_values(&self, bit: u8) -> Vec<u16> {
        (self.min..=self.max)
            .filter(|i| self.is_bit_set(*i, bit).unwrap_or(false))
            .collect()
    }

    fn get_nth_bit(value: &str) -> Result<u8, CronError> {
        // If value ends with 'L', we set the LAST_BIT and exit early
        if value.ends_with('L') {
            return Ok(LAST_BIT);
        }
        if let Some(nth_pos) = value.find('#') {
            let nth = value[nth_pos + 1..]
                .parse::<u8>()
                .map_err(|_| CronError::ComponentError("Invalid nth specifier.".to_string()))?;

            if nth == 0 || nth > 5 {
                Err(CronError::ComponentError(
                    "Nth specifier out of bounds.".to_string(),
                ))
            } else {
                match nth {
                    1 => Ok(NTH_1ST_BIT),
                    2 => Ok(NTH_2ND_BIT),
                    3 => Ok(NTH_3RD_BIT),
                    4 => Ok(NTH_4TH_BIT),
                    5 => Ok(NTH_5TH_BIT),
                    _ => Err(CronError::ComponentError(
                        "Invalid nth specifier.".to_string(),
                    )),
                }
            }
        } else {
            Ok(ALL_BIT)
        }
    }

    // Removes everything after #
    fn strip_nth_part(value: &str) -> &str {
        value.split('#').next().unwrap_or("")
    }

    fn handle_closest_weekday(&mut self, value: &str) -> Result<(), CronError> {
        if let Some(day_pos) = value.find('W') {
            // Use a slice
            let day_str = &value[..day_pos];

            // Parse the day from the slice
            let day = day_str.parse::<u16>().map_err(|_| {
                CronError::ComponentError("Invalid day for closest weekday.".to_string())
            })?;

            // Check if the day is within the allowed range
            if day < self.min || day > self.max {
                return Err(CronError::ComponentError(
                    "Day for closest weekday out of bounds.".to_string(),
                ));
            }

            // Set the bit for the closest weekday
            self.set_bit(day, CLOSEST_WEEKDAY_BIT)?;
        } else {
            // If 'W' is not found, handle the value as a regular number
            self.handle_number(value)?;
        }
        Ok(())
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
            .parse::<u16>()
            .map_err(|_| CronError::ComponentError("Invalid start of range.".to_string()))?;
        let end = parts[1]
            .parse::<u16>()
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
            .parse::<u16>()
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
            .parse::<u16>()
            .map_err(|_| CronError::ComponentError("Invalid step.".to_string()))?;

        self.step = step;

        if step == 0 {
            return Err(CronError::ComponentError(
                "Step cannot be zero.".to_string(),
            ));
        }

        let (start, end) = if range_part == "*" {
            self.from_wildcard = true;
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
                    .parse::<u16>()
                    .map_err(|_| CronError::ComponentError("Invalid range start.".to_string()))?,
                bounds[1]
                    .parse::<u16>()
                    .map_err(|_| CronError::ComponentError("Invalid range end.".to_string()))?,
            )
        } else {
            let single_start = range_part
                .parse::<u16>()
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
        let component = CronComponent::new(0, 59, ALL_BIT | LAST_BIT, 0);
        assert_eq!(component.min, 0);
        assert_eq!(component.max, 59);
        // Ensure all bitfields are initialized to NONE_BIT
        assert!(component.bitfields.iter().all(|&b| b == NONE_BIT));
        // Check that ALL_BIT and LAST_BIT are included in features
        assert!(component.features & (ALL_BIT | LAST_BIT) == (ALL_BIT | LAST_BIT));
    }

    #[test]
    fn test_set_bit() {
        let mut component = CronComponent::new(0, 59, ALL_BIT, 0);
        assert!(component.set_bit(10, ALL_BIT).is_ok());
        assert!(component.is_bit_set(10, ALL_BIT).unwrap());
    }

    #[test]
    fn test_set_bit_out_of_bounds() {
        let mut component = CronComponent::new(0, 59, ALL_BIT, 0);
        assert!(matches!(
            component.set_bit(60, ALL_BIT),
            Err(CronError::ComponentError(_))
        ));
    }

    #[test]
    fn test_unset_bit() {
        let mut component = CronComponent::new(0, 59, ALL_BIT, 0);
        component.set_bit(10, ALL_BIT).unwrap();
        assert!(component.unset_bit(10, ALL_BIT).is_ok());
        assert!(!component.is_bit_set(10, ALL_BIT).unwrap());
    }

    #[test]
    fn test_is_feature_enabled() {
        let mut component = CronComponent::new(0, 59, LAST_BIT, 0);
        assert!(!component.is_feature_enabled(LAST_BIT));
        component.enable_feature(LAST_BIT).unwrap();
        assert!(component.is_feature_enabled(LAST_BIT));
    }

    #[test]
    fn test_enable_feature_unsupported() {
        let mut component = CronComponent::new(0, 59, NONE_BIT, 0);
        assert!(matches!(
            component.enable_feature(NTH_1ST_BIT),
            Err(CronError::ComponentError(_))
        ));
    }

    #[test]
    fn test_parse_asterisk() {
        let mut component = CronComponent::new(0, 59, ALL_BIT, 0);
        component.parse("*").unwrap();
        for i in 0..=59 {
            assert!(component.is_bit_set(i, ALL_BIT).unwrap());
        }
    }

    #[test]
    fn test_parse_range() {
        let mut component = CronComponent::new(0, 59, ALL_BIT, 0);
        component.parse("10-15").unwrap();
        for i in 10..=15 {
            assert!(component.is_bit_set(i, ALL_BIT).unwrap());
        }
    }

    #[test]
    fn test_parse_stepping() {
        let mut component = CronComponent::new(0, 59, ALL_BIT, 0);
        component.parse("*/5").unwrap();
        for i in (0..=59).filter(|n| n % 5 == 0) {
            assert!(component.is_bit_set(i, ALL_BIT).unwrap());
        }
    }

    #[test]
    fn test_parse_list() {
        let mut component = CronComponent::new(0, 59, ALL_BIT, 0);
        component.parse("5,10,15").unwrap();
        for i in [5, 10, 15].iter() {
            assert!(component.is_bit_set(*i, ALL_BIT).unwrap());
        }
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let mut component = CronComponent::new(0, 59, ALL_BIT, 0);
        assert!(component.parse("10-").is_err());
        assert!(component.parse("*/").is_err());
        assert!(component.parse("60").is_err()); // out of bounds for the minute field
    }

    #[test]
    fn test_parse_closest_weekday() {
        let mut component = CronComponent::new(1, 31, CLOSEST_WEEKDAY_BIT, 0);
        component.parse("15W").unwrap();
        assert!(component.is_bit_set(15, CLOSEST_WEEKDAY_BIT).unwrap());
        // You might want to add more tests for edge cases
    }
}
