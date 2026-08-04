//! Calendar helpers that do not depend on a date and time library.
//!
//! Cron patterns are matched on the civil (wall clock) calendar, so the rules
//! in this module are the only calendar knowledge the matcher needs.

/// A day of the week.
///
/// The discriminants count from Sunday, which is the numbering that cron
/// patterns use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weekday {
    Sunday = 0,
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
}

impl Weekday {
    /// Returns the number of days from Sunday, in the range 0 to 6.
    pub const fn num_days_from_sunday(self) -> u32 {
        self as u32
    }

    /// Creates a weekday from the number of days from Sunday, wrapping every
    /// seven days. This is the inverse of [`num_days_from_sunday`].
    ///
    /// [`num_days_from_sunday`]: Weekday::num_days_from_sunday
    pub const fn from_days_from_sunday(days: u32) -> Weekday {
        match days % 7 {
            0 => Weekday::Sunday,
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            _ => Weekday::Saturday,
        }
    }

    /// Returns the weekday a signed number of days away.
    ///
    /// The matcher uses this to read the weekday of another day of the same
    /// month without consulting a calendar.
    pub const fn shift(self, days: i32) -> Weekday {
        Weekday::from_days_from_sunday((self as i32 + days).rem_euclid(7) as u32)
    }
}

/// Returns `true` if `year` is a leap year in the proleptic Gregorian calendar.
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Returns the number of days in a month.
///
/// The month must be in the range 1 to 12, and every caller checks it before
/// calling. Panics otherwise, as [`day_of_year`] does.
pub const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_moves_in_both_directions_and_wraps() {
        assert_eq!(Weekday::Wednesday.shift(0), Weekday::Wednesday);
        assert_eq!(Weekday::Wednesday.shift(1), Weekday::Thursday);
        assert_eq!(Weekday::Wednesday.shift(-1), Weekday::Tuesday);
        assert_eq!(Weekday::Saturday.shift(1), Weekday::Sunday);
        assert_eq!(Weekday::Sunday.shift(-1), Weekday::Saturday);
        // A whole number of weeks lands back on the same day.
        assert_eq!(Weekday::Monday.shift(70), Weekday::Monday);
        assert_eq!(Weekday::Monday.shift(-70), Weekday::Monday);
        // The longest shift the matcher makes is the length of a month.
        assert_eq!(Weekday::Friday.shift(31), Weekday::Monday);
    }

    #[test]
    fn shift_agrees_with_stepping_one_day_at_a_time() {
        let mut stepped = Weekday::Thursday;
        for days in 0..400 {
            assert_eq!(Weekday::Thursday.shift(days), stepped, "after {days} days");
            assert_eq!(
                Weekday::Thursday.shift(-days).shift(days),
                Weekday::Thursday
            );
            stepped = stepped.shift(1);
        }
    }

    #[test]
    fn days_in_month_handles_leap_years() {
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2024, 4), 30);
        assert_eq!(days_in_month(2024, 12), 31);
    }

    #[test]
    #[should_panic]
    fn days_in_month_rejects_a_month_above_the_range() {
        days_in_month(2023, 13);
    }

    #[test]
    #[should_panic]
    fn days_in_month_rejects_a_month_below_the_range() {
        days_in_month(2023, 0);
    }
}
