use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::{format, vec};

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
//
// `PartialEq`, `Ord` and `Hash` are implemented below on the field's values, so a
// field compares the same whether or not its bitfields are allocated.
#[derive(Debug, Default, Clone)]
pub struct CronComponent {
    // One u8 of flags per value, indexed by value, allocated on the first set bit.
    bitfields: Vec<u8>,
    // A plain `*` stored without bitfields: every value from min to max has
    // ALL_BIT and nothing else. This keeps the year field, which has thousands
    // of values, from allocating anything in the common case.
    all_values: bool,
    pub min: u16,            // Minimum value this component can take
    pub max: u16,            // Maximum value this component can take
    pub step: u16,           // Steps to skip in this component
    pub from_wildcard: bool, // Wildcard used
    features: u8,            // Single u8 bitfield to indicate supported special bits, like LAST_BIT
    enabled_features: u8,    // Bitfield to hold component-wide special bits like LAST_BIT
    input_offset: u16,       // Offset for numerical representation
    sloppy_ranges: bool,     // Allow sloppy range syntax for backward compatibility
    wrap_ranges: bool,       // Let a range whose end is before its start wrap around, like FRI-MON
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
        Self {
            // Vector of u8 to act as multiple bitfields.
            // - Allocated on the first set bit, see `ensure_bitfields`.
            bitfields: Vec::new(),

            // Set by parsing a plain `*`, see `all_values`.
            all_values: false,

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

            sloppy_ranges: false, // Disabled by default for OCPS/vixie-cron compliance

            wrap_ranges: false, // Only the Quartz-style day-of-week field wraps
        }
    }

    /// Allocates the bitfields, filling in ALL_BIT for a compact `*`.
    fn ensure_bitfields(&mut self) {
        if !self.bitfields.is_empty() {
            return;
        }
        self.bitfields = vec![NONE_BIT; self.max as usize + 1];
        if self.all_values {
            self.all_values = false;
            for bits in &mut self.bitfields[self.min as usize..] {
                *bits = ALL_BIT;
            }
        }
    }

    /// Drops the bitfields of a wildcard field that ended up matching every value
    /// with ALL_BIT only, like `*/1`, so it is stored and compared like `*`.
    fn compact_if_full_wildcard(&mut self) {
        if !self.from_wildcard || self.enabled_features != NONE_BIT {
            return;
        }
        let min = self.min as usize;
        let full = self.bitfields.len() == self.max as usize + 1
            && self.bitfields[..min].iter().all(|&bits| bits == NONE_BIT)
            && self.bitfields[min..].iter().all(|&bits| bits == ALL_BIT);
        if full {
            self.bitfields = Vec::new();
            self.all_values = true;
        }
    }

    /// Sets whether to allow sloppy range syntax.
    pub fn set_sloppy_ranges(&mut self, sloppy: bool) {
        self.sloppy_ranges = sloppy;
    }

    /// Lets a range whose end is before its start wrap around past the maximum,
    /// as Quartz does for day-of-week ranges like `FRI-MON` or `SAT-SUN` (`7-1`).
    pub(crate) fn set_wrap_ranges(&mut self, wrap: bool) {
        self.wrap_ranges = wrap;
    }

    /// The lowest and highest values a pattern can write for this component.
    fn input_bounds(&self) -> (u16, u16) {
        (self.min + self.input_offset, self.max)
    }

    /// Whether `start-end` with `start > end` should wrap around rather than fail.
    fn wraps(&self, start: u16, end: u16) -> bool {
        let (low, high) = self.input_bounds();
        self.wrap_ranges && start > end && start <= high && end >= low
    }

    /// Sets `bit` on every `step`th value from `start` to `end`, wrapping
    /// around from the highest value to the lowest when `start > end`.
    fn set_wrapped_range(
        &mut self,
        start: u16,
        end: u16,
        step: u16,
        bit: u8,
    ) -> Result<(), CronError> {
        let (low, high) = self.input_bounds();
        let span = high - low + 1;
        let mut value = start;
        while value <= end + span {
            let wrapped = if value > high { value - span } else { value };
            self.set_bit(wrapped, bit)?;
            value = value.checked_add(step).ok_or_else(|| {
                CronError::ComponentError("Value exceeded max after stepping.".to_string())
            })?;
        }
        Ok(())
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
        self.ensure_bitfields();
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
        self.ensure_bitfields();
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
        } else if self.all_values {
            Ok((ALL_BIT & bit) != 0)
        } else if self.bitfields.is_empty() {
            // Nothing has been set yet.
            Ok(false)
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
            // Every value from min to max matches, with ALL_BIT only. When
            // nothing else has been set, store that without bitfields.
            if self.bitfields.is_empty() {
                self.all_values = true;
                return Ok(());
            }
            for bits in &mut self.bitfields[self.min as usize..] {
                *bits |= ALL_BIT;
            }
            self.compact_if_full_wildcard();
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
                // A bare "L" only means something in day-of-month. In day-of-week
                // (the field that allows "#"), "L" must follow a weekday, like "5L".
                if self.is_feature_allowed(NTH_ALL) {
                    return Err(CronError::ComponentError(
                        "L in the day-of-week field must follow a weekday, like 5L or 5#L."
                            .to_string(),
                    ));
                }
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

        self.compact_if_full_wildcard();
        Ok(())
    }

    /// Returns the values this field matches without a modifier, in ascending order.
    ///
    /// For an hour field of `1-5,10` this is `[1, 2, 3, 4, 5, 10]`. Values selected only
    /// through `L`, `W` or `#` are not included; [`Self::has_modifiers`] reports those.
    /// Weekdays use `0` for Sunday, whichever weekday numbering the pattern was parsed with.
    ///
    /// ```
    /// use std::str::FromStr;
    /// use croner::Cron;
    ///
    /// let cron = Cron::from_str("0 9-11 * * 1-5").unwrap();
    /// assert_eq!(cron.pattern.hours.values(), vec![9, 10, 11]);
    /// assert_eq!(cron.pattern.days_of_week.values(), vec![1, 2, 3, 4, 5]);
    /// ```
    pub fn values(&self) -> Vec<u16> {
        self.get_set_values(ALL_BIT)
    }

    /// Returns whether the field uses `L`, `W` or `#`, whose matches depend on the month.
    pub fn has_modifiers(&self) -> bool {
        self.enabled_features != NONE_BIT
            || self
                .bitfields
                .iter()
                .any(|bits| bits & !ALL_BIT != NONE_BIT)
    }

    /// Returns a vector of u16 values for all bits set in the component for a given bitflag.
    pub fn get_set_values(&self, bit: u8) -> Vec<u16> {
        self.set_values(bit).collect()
    }

    /// Returns how many bits are set in the component for a given bitflag.
    pub fn count_set_values(&self, bit: u8) -> usize {
        self.set_values(bit).count()
    }

    /// Iterates the values whose bit is set in the component for a given bitflag.
    fn set_values(&self, bit: u8) -> impl Iterator<Item = u16> + '_ {
        // An unsupported bit is never set, and only values from min to max
        // that the bitfields hold can be, so scan that slice directly.
        let supported = self.features & bit == bit;
        let span = if supported {
            (self.max as usize + 1).saturating_sub(self.min as usize)
        } else {
            0
        };
        // A compact `*` has ALL_BIT, and only that, on every value.
        let compact = if self.all_values && bit & ALL_BIT != 0 {
            span
        } else {
            0
        };
        (self.min..).take(compact).chain(
            self.bitfields
                .get(self.min as usize..)
                .unwrap_or(&[])
                .iter()
                .take(span)
                .zip(self.min..)
                .filter(move |(bits, _)| *bits & bit != 0)
                .map(|(_, value)| value),
        )
    }

    /// Returns the first value from `from` up to max whose `bit` is set.
    pub(crate) fn next_set_value(&self, from: u16, bit: u8) -> Option<u16> {
        let start = from.max(self.min);
        if self.all_values {
            return (start <= self.max && bit & ALL_BIT != 0).then_some(start);
        }
        self.bitfields
            .get(start as usize..=self.max as usize)?
            .iter()
            .position(|bits| bits & bit != 0)
            .map(|offset| start + offset as u16)
    }

    /// Returns the last value from min up to `upto` whose `bit` is set.
    pub(crate) fn previous_set_value(&self, upto: u16, bit: u8) -> Option<u16> {
        if self.all_values {
            return (self.min <= upto && upto <= self.max && bit & ALL_BIT != 0).then_some(upto);
        }
        self.bitfields
            .get(self.min as usize..=upto as usize)?
            .iter()
            .rposition(|bits| bits & bit != 0)
            .map(|offset| self.min + offset as u16)
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

            // Check for "LW" - last weekday of the month
            if day_str.eq_ignore_ascii_case("L") {
                // Enable both LAST_BIT and CLOSEST_WEEKDAY_BIT to indicate "last weekday"
                self.enable_feature(LAST_BIT)?;
                self.enable_feature(CLOSEST_WEEKDAY_BIT)?;
                return Ok(());
            }

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

        if self.wraps(start, end) {
            return self.set_wrapped_range(start, end, 1, bit_to_set);
        }

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
            (self.min + self.input_offset, self.max)
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
        } else if range_part.is_empty() {
            // Empty range part (e.g., /10)
            if self.sloppy_ranges {
                // In sloppy mode, treat /10 as */10
                self.from_wildcard = true;
                (self.min + self.input_offset, self.max)
            } else {
                return Err(CronError::ComponentError(
                    format!("Invalid step syntax: omitting the starting point like \"/{0}\" is not allowed. Use \"*/{0}\" for wildcard steps or \"X-Y/{0}\" for range steps.", step_str)
                ));
            }
        } else {
            // Single number with step (e.g., 0/10, 30/10)
            if self.sloppy_ranges {
                // In sloppy mode, allow single number as starting point
                let single_start = range_part
                    .parse::<u16>()
                    .map_err(|_| CronError::ComponentError("Invalid start.".to_string()))?;
                (single_start, self.max)
            } else {
                // Strict mode: not allowed per OCPS/vixie/cronie standards
                return Err(CronError::ComponentError(
                    format!("Invalid step syntax: single number steps like \"{}/{}\" are not allowed. Use \"*/{1}\" for wildcard steps or \"X-Y/{1}\" for range steps.", range_part, step_str)
                ));
            }
        };

        if self.wraps(start, end) {
            return self.set_wrapped_range(start, end, step, bit_to_set);
        }

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

impl CronComponent {
    /// The flags of every value from 0 to max, whether or not they are allocated.
    fn logical_bitfields(&self) -> impl Iterator<Item = u8> + '_ {
        let min = self.min as usize;
        // Unallocated, this is the length `new` used to allocate.
        let len = if self.bitfields.is_empty() {
            if self.max > 0 {
                self.max as usize + 1
            } else {
                0
            }
        } else {
            self.bitfields.len()
        };
        (0..len).map(move |index| match self.bitfields.get(index) {
            Some(&bits) => bits,
            None if self.all_values && index >= min => ALL_BIT,
            None => NONE_BIT,
        })
    }
}

impl PartialEq for CronComponent {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for CronComponent {}

impl PartialOrd for CronComponent {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CronComponent {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let bitfields = if !self.bitfields.is_empty() && !other.bitfields.is_empty() {
            self.bitfields.cmp(&other.bitfields)
        } else {
            self.logical_bitfields().cmp(other.logical_bitfields())
        };
        bitfields
            .then_with(|| self.min.cmp(&other.min))
            .then_with(|| self.max.cmp(&other.max))
            .then_with(|| self.step.cmp(&other.step))
            .then_with(|| self.from_wildcard.cmp(&other.from_wildcard))
            .then_with(|| self.features.cmp(&other.features))
            .then_with(|| self.enabled_features.cmp(&other.enabled_features))
            .then_with(|| self.input_offset.cmp(&other.input_offset))
            .then_with(|| self.sloppy_ranges.cmp(&other.sloppy_ranges))
            .then_with(|| self.wrap_ranges.cmp(&other.wrap_ranges))
    }
}

impl core::hash::Hash for CronComponent {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        for bits in self.logical_bitfields() {
            state.write_u8(bits);
        }
        self.min.hash(state);
        self.max.hash(state);
        self.step.hash(state);
        self.from_wildcard.hash(state);
        self.features.hash(state);
        self.enabled_features.hash(state);
        self.input_offset.hash(state);
        self.sloppy_ranges.hash(state);
        self.wrap_ranges.hash(state);
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

    fn year_component() -> CronComponent {
        CronComponent::new(1, 5000, NONE_BIT, 0)
    }

    #[test]
    fn test_wildcard_allocates_no_bitfields() {
        let mut years = year_component();
        assert!(years.bitfields.is_empty());
        years.parse("*").unwrap();
        assert!(years.bitfields.is_empty());

        // `*/1` matches the same values and is stored and compared like `*`.
        let mut stepped = year_component();
        stepped.parse("*/1").unwrap();
        assert!(stepped.bitfields.is_empty());
        assert_eq!(years, stepped);

        // A step that skips values still needs the bitfields.
        let mut every_other = year_component();
        every_other.parse("*/2").unwrap();
        assert_eq!(every_other.bitfields.len(), 5001);
        assert_ne!(years, every_other);
    }

    #[test]
    fn test_compact_wildcard_matches_like_allocated_bitfields() {
        let mut compact = CronComponent::new(1, 31, LAST_BIT | CLOSEST_WEEKDAY_BIT, 0);
        compact.parse("*").unwrap();
        assert!(compact.bitfields.is_empty());

        // The same field, with its bits written out.
        let mut allocated = compact.clone();
        allocated.ensure_bitfields();
        assert_eq!(allocated.bitfields.len(), 32);
        assert_eq!(compact, allocated);
        assert_eq!(compact.cmp(&allocated), core::cmp::Ordering::Equal);
        let hash = |c: &CronComponent| {
            use core::hash::{Hash, Hasher};
            let mut hasher = std::hash::DefaultHasher::new();
            c.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash(&compact), hash(&allocated));

        for bit in [ALL_BIT, LAST_BIT, CLOSEST_WEEKDAY_BIT] {
            assert_eq!(compact.get_set_values(bit), allocated.get_set_values(bit));
            assert_eq!(
                compact.count_set_values(bit),
                allocated.count_set_values(bit)
            );
            for pos in 0..=40 {
                assert_eq!(
                    compact.is_bit_set(pos, bit).ok(),
                    allocated.is_bit_set(pos, bit).ok()
                );
                assert_eq!(
                    compact.next_set_value(pos, bit),
                    allocated.next_set_value(pos, bit),
                    "next from {pos}, bit {bit}"
                );
                assert_eq!(
                    compact.previous_set_value(pos, bit),
                    allocated.previous_set_value(pos, bit),
                    "previous up to {pos}, bit {bit}"
                );
            }
        }
        assert_eq!(compact.values(), (1..=31).collect::<Vec<_>>());
        assert!(!compact.has_modifiers());
    }

    #[test]
    fn test_set_bit_after_compact_wildcard() {
        let mut days = CronComponent::new(1, 31, CLOSEST_WEEKDAY_BIT, 0);
        days.parse("*").unwrap();
        days.set_bit(15, CLOSEST_WEEKDAY_BIT).unwrap();
        assert!(days.is_bit_set(1, ALL_BIT).unwrap());
        assert!(days.is_bit_set(15, CLOSEST_WEEKDAY_BIT).unwrap());
        assert!(!days.is_bit_set(16, CLOSEST_WEEKDAY_BIT).unwrap());
        days.unset_bit(1, ALL_BIT).unwrap();
        assert!(!days.is_bit_set(1, ALL_BIT).unwrap());
        assert!(days.is_bit_set(2, ALL_BIT).unwrap());
    }

    #[test]
    fn test_parse_closest_weekday() {
        let mut component = CronComponent::new(1, 31, CLOSEST_WEEKDAY_BIT, 0);
        component.parse("15W").unwrap();
        assert!(component.is_bit_set(15, CLOSEST_WEEKDAY_BIT).unwrap());
        // You might want to add more tests for edge cases
    }
}
