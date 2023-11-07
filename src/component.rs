

pub const MAX_SIZE: usize = 60; // Maximum size for the actual values (0-59)

// Constants for special flags
pub const NONE_BIT: u64 = 0;
pub const LAST_BIT: u64 = 1 << 60;
// Reserved for future use, feel free to rename
// pub const SPECIAL_BIT_1: u64 = 1 << 61;
// pub const SPECIAL_BIT_2: u64 = 1 << 62;
// pub const SPECIAL_BIT_3: u64 = 1 << 63;

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
    pub fn set_bit(&mut self, pos: u8) -> Result<(), CronComponentError> {
        if pos < self.min || pos > self.max {
            return Err(CronComponentError::OutOfBounds(format!(
                "Bit position {} is out of bounds.",
                pos
            )));
        }
        self.bitfield |= 1 << pos;
        Ok(())
    }

    // Unset a bit at a given position (0 to 59)
    pub fn unset_bit(&mut self, pos: u8) -> Result<(), CronComponentError> {
        if pos < self.min || pos > self.max {
            return Err(CronComponentError::OutOfBounds(format!(
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
    pub fn set_special_bit(&mut self, flag: u64, set: bool) -> Result<(), CronComponentError> {
        // Check if the bit is within the supported features
        if self.features & flag == 0 {
            return Err(CronComponentError::UnsupportedSpecialBit);
        }

        if set {
            self.bitfield |= flag;
        } else {
            self.bitfield &= !flag;
        }
        Ok(())
    }

    // Unset a special bit if it is supported
    pub fn unset_special_bit(&mut self, flag: u64) -> Result<(), CronComponentError> {
        // Check if the bit is within the supported features
        if self.features & flag == 0 {
            return Err(CronComponentError::UnsupportedSpecialBit);
        }

        self.bitfield &= !flag;
        Ok(())
    }

    pub fn parse(&mut self, field: &str) -> Result<(), CronComponentError> {
        if field == "*" {
            for value in self.min..=self.max {
                self.set_bit(value);
            }
        } else {
            // Split the field into parts and handle each part
            for part in field.split(',') {
                let trimmed_part = part.trim();
                println!("{}", trimmed_part);
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

    fn handle_range(&mut self, range: &str) -> Result<(), CronComponentError> {
        let parts: Vec<&str> = range.split('-').map(str::trim).collect();
        if parts.len() != 2 {
            return Err(CronComponentError::InvalidSyntax(
                "Invalid range syntax.".to_string(),
            ));
        }

        let start = parts[0].parse::<u8>().map_err(|_| {
            CronComponentError::InvalidSyntax("Invalid start of range.".to_string())
        })?;
        let end = parts[1]
            .parse::<u8>()
            .map_err(|_| CronComponentError::InvalidSyntax("Invalid end of range.".to_string()))?;

        if start > end || start < self.min || end > self.max {
            return Err(CronComponentError::OutOfBounds(
                "Range out of bounds.".to_string(),
            ));
        }

        // Calculate the bitmask for the range in one operation
        let mask: u64 = ((1 << (end - start + 1)) - 1) << start;
        self.bitfield |= mask;

        Ok(())
    }

    fn handle_number(&mut self, value: &str) -> Result<(), CronComponentError> {
        let num = value.parse::<u8>().map_err(|_| {
            CronComponentError::InvalidSyntax("Invalid number.".to_string())
        })?;
        if num < self.min || num > self.max {
            return Err(CronComponentError::OutOfBounds(
                "Number out of bounds.".to_string(),
            ));
        }
    
        self.set_bit(num)?;
        Ok(())
    }
    
    pub fn handle_stepping(
        &mut self,
        stepped_range: &str,
    ) -> Result<(), CronComponentError> {
        let parts: Vec<&str> = stepped_range.split('/').collect();
        if parts.len() != 2 {
            return Err(CronComponentError::InvalidSyntax(
                "Invalid stepped range syntax.".to_string(),
            ));
        }
    
        let range_part = parts[0];
        let step_str = parts[1];
        let step = step_str.parse::<u8>().map_err(|_| {
            CronComponentError::InvalidSyntax("Invalid step.".to_string())
        })?;
        if step == 0 {
            return Err(CronComponentError::StepError(
                "Step cannot be zero.".to_string(),
            ));
        }
    
        let (start, end) = if range_part == "*" {
            (self.min, self.max)
        } else if range_part.contains('-') {
            let bounds: Vec<&str> = range_part.split('-').collect();
            if bounds.len() != 2 {
                return Err(CronComponentError::InvalidSyntax(
                    "Invalid range syntax in stepping.".to_string(),
                ));
            }
            (
                bounds[0].parse::<u8>().map_err(|_| {
                    CronComponentError::InvalidSyntax("Invalid range start.".to_string())
                })?,
                bounds[1].parse::<u8>().map_err(|_| {
                    CronComponentError::InvalidSyntax("Invalid range end.".to_string())
                })?,
            )
        } else {
            let start = range_part.parse::<u8>().map_err(|_| {
                CronComponentError::InvalidSyntax("Invalid start.".to_string())
            })?;
            (start, start)
        };
    
        if start < self.min || end > self.max || start > end {
            return Err(CronComponentError::OutOfBounds(
                "Range is out of bounds in stepping.".to_string(),
            ));
        }
    
        // Apply stepping within the range
        let mut value = start;
        while value <= end {
            self.set_bit(value)?;
            value = value.checked_add(step).ok_or_else(|| {
                CronComponentError::OutOfBounds("Value exceeded max after stepping.".to_string())
            })?;
        }
    
        Ok(())
    }

}
