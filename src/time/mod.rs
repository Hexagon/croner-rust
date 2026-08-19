//! Backend-agnostic date and time support.
//!
//! Croner runs its search on the civil (wall clock) types in this module, so a
//! pattern gives the same result with every date and time library. A library is
//! connected through the [`CronDateTime`] trait.
//!
//! Croner includes these implementations:
//!
//! | Type | Crate feature |
//! |------|---------------|
//! | [`chrono::DateTime<Tz>`](https://docs.rs/chrono/0.4/chrono/struct.DateTime.html) | `chrono` (default) |
//! | [`chrono::NaiveDateTime`](https://docs.rs/chrono/0.4/chrono/struct.NaiveDateTime.html) | `chrono` (default) |
//! | [`jiff::Zoned`](https://docs.rs/jiff/0.2/jiff/struct.Zoned.html) | `jiff` |
//! | [`jiff::civil::DateTime`](https://docs.rs/jiff/0.2/jiff/civil/struct.DateTime.html) | `jiff` |
//!
//! Implement [`CronDateTime`] for your own type to use a different library.

#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
mod chrono_impl;
#[cfg(feature = "jiff")]
#[cfg_attr(docsrs, doc(cfg(feature = "jiff")))]
mod jiff_impl;

mod cursor;

pub(crate) use cursor::Cursor;

use crate::errors::CronError;

const SECONDS_PER_DAY: i64 = 86_400;

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

/// Returns the number of days in a year, either 365 or 366.
pub const fn days_in_year(year: i32) -> u32 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

/// Returns the day of the year, in the range 1 to 366.
///
/// The month must be in the range 1 to 12 and the day must exist.
pub const fn day_of_year(year: i32, month: u32, day: u32) -> u32 {
    // Days in the whole months before each month of a common year.
    const BEFORE_MONTH: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

    let leap_day = if month > 2 && is_leap_year(year) {
        1
    } else {
        0
    };
    BEFORE_MONTH[(month - 1) as usize] + day + leap_day
}

/// A civil date: a year, a month and a day, without a time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilDate {
    year: i32,
    month: u8,
    day: u8,
}

impl CivilDate {
    /// Creates a date, or returns `None` if the date does not exist.
    pub fn from_ymd_opt(year: i32, month: u32, day: u32) -> Option<CivilDate> {
        // The month has to be real before its length means anything.
        if month == 0 || month > 12 {
            return None;
        }
        if day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(CivilDate {
            year,
            month: month as u8,
            day: day as u8,
        })
    }

    /// Creates a date, or returns [`CronError::InvalidDate`] if the date does
    /// not exist.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Result<CivilDate, CronError> {
        CivilDate::from_ymd_opt(year, month, day).ok_or(CronError::InvalidDate)
    }

    /// Creates a date without checking that it exists.
    ///
    /// The caller must pass a real date. Backends use this when their own type
    /// has already checked the parts.
    pub const fn from_parts_unchecked(year: i32, month: u32, day: u32) -> CivilDate {
        CivilDate {
            year,
            month: month as u8,
            day: day as u8,
        }
    }

    /// Returns the year.
    pub const fn year(self) -> i32 {
        self.year
    }

    /// Returns the month, in the range 1 to 12.
    pub const fn month(self) -> u32 {
        self.month as u32
    }

    /// Returns the day of the month, in the range 1 to 31.
    pub const fn day(self) -> u32 {
        self.day as u32
    }

    /// Returns the number of days in this date's month.
    pub const fn days_in_month(self) -> u32 {
        days_in_month(self.year, self.month())
    }

    /// Returns the day of the year, in the range 1 to 366.
    pub const fn day_of_year(self) -> u32 {
        day_of_year(self.year, self.month(), self.day())
    }

    /// Adds a signed number of days, or returns `None` if the year would leave
    /// the range of an `i32`.
    ///
    /// The cost grows with the number of month boundaries crossed, so this is
    /// kept inside the crate, where every step is at most one year.
    pub(crate) fn checked_add_days(self, days: i64) -> Option<CivilDate> {
        let mut year = self.year;
        let mut month = self.month();
        let mut day = i64::from(self.day) + days;

        // Croner steps a day at a time, so the result usually stays inside the
        // same month and neither loop runs.
        while day > i64::from(days_in_month(year, month)) {
            day -= i64::from(days_in_month(year, month));
            month += 1;
            if month > 12 {
                month = 1;
                year = year.checked_add(1)?;
            }
        }
        while day < 1 {
            if month == 1 {
                month = 12;
                year = year.checked_sub(1)?;
            } else {
                month -= 1;
            }
            day += i64::from(days_in_month(year, month));
        }

        Some(CivilDate {
            year,
            month: month as u8,
            day: day as u8,
        })
    }
}

/// A civil time of day to second precision, without a time zone.
///
/// Sub-second parts are not kept, because cron patterns never match on them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilTime {
    hour: u8,
    minute: u8,
    second: u8,
}

impl CivilTime {
    /// The first second of a day, 00:00:00.
    pub const MIDNIGHT: CivilTime = CivilTime::from_parts_unchecked(0, 0, 0);

    /// The last second of a day, 23:59:59.
    pub const END_OF_DAY: CivilTime = CivilTime::from_parts_unchecked(23, 59, 59);

    /// Creates a time of day, or returns `None` if any part is out of range.
    ///
    /// Leap seconds are not supported, so `second` must be in the range 0 to 59.
    pub fn from_hms_opt(hour: u32, minute: u32, second: u32) -> Option<CivilTime> {
        if hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        Some(CivilTime::from_parts_unchecked(hour, minute, second))
    }

    /// Creates a time of day without checking the parts.
    ///
    /// The caller must pass a time of day in range. Backends use this when
    /// their own type has already checked the parts.
    pub const fn from_parts_unchecked(hour: u32, minute: u32, second: u32) -> CivilTime {
        CivilTime {
            hour: hour as u8,
            minute: minute as u8,
            second: second as u8,
        }
    }

    /// Returns the hour, in the range 0 to 23.
    pub const fn hour(self) -> u32 {
        self.hour as u32
    }

    /// Returns the minute, in the range 0 to 59.
    pub const fn minute(self) -> u32 {
        self.minute as u32
    }

    /// Returns the second, in the range 0 to 59.
    pub const fn second(self) -> u32 {
        self.second as u32
    }

    /// Returns the number of seconds since midnight.
    const fn seconds_of_day(self) -> i64 {
        self.hour as i64 * 3600 + self.minute as i64 * 60 + self.second as i64
    }

    /// Creates a time of day from a number of seconds since midnight, which
    /// must be in the range 0 to 86399.
    const fn from_seconds_of_day(seconds: i64) -> CivilTime {
        CivilTime {
            hour: (seconds / 3600) as u8,
            minute: (seconds % 3600 / 60) as u8,
            second: (seconds % 60) as u8,
        }
    }
}

impl core::fmt::Display for CivilTime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

/// A civil date and time to second precision, without a time zone.
///
/// This is the value croner and a date and time backend exchange. Sub-second
/// parts are not kept, because cron patterns never match on them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilDateTime {
    date: CivilDate,
    time: CivilTime,
}

impl CivilDateTime {
    /// Joins a date and a time of day.
    pub const fn new(date: CivilDate, time: CivilTime) -> CivilDateTime {
        CivilDateTime { date, time }
    }

    /// Creates a date and time, or returns `None` if either part is invalid.
    ///
    /// Leap seconds are not supported, so `second` must be in the range 0 to 59.
    pub fn from_ymd_hms_opt(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Option<CivilDateTime> {
        Some(CivilDateTime::new(
            CivilDate::from_ymd_opt(year, month, day)?,
            CivilTime::from_hms_opt(hour, minute, second)?,
        ))
    }

    /// Creates a date and time, or returns an error if either part is invalid.
    pub fn from_ymd_hms(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Result<CivilDateTime, CronError> {
        Ok(CivilDateTime::new(
            CivilDate::from_ymd(year, month, day)?,
            CivilTime::from_hms_opt(hour, minute, second).ok_or(CronError::InvalidTime)?,
        ))
    }

    /// Returns the date part.
    pub const fn date(self) -> CivilDate {
        self.date
    }

    /// Returns the time of day.
    pub const fn time(self) -> CivilTime {
        self.time
    }

    /// Returns the year.
    pub const fn year(self) -> i32 {
        self.date.year()
    }

    /// Returns the month, in the range 1 to 12.
    pub const fn month(self) -> u32 {
        self.date.month()
    }

    /// Returns the day of the month, in the range 1 to 31.
    pub const fn day(self) -> u32 {
        self.date.day()
    }

    /// Returns the hour, in the range 0 to 23.
    pub const fn hour(self) -> u32 {
        self.time.hour()
    }

    /// Returns the minute, in the range 0 to 59.
    pub const fn minute(self) -> u32 {
        self.time.minute()
    }

    /// Returns the second, in the range 0 to 59.
    pub const fn second(self) -> u32 {
        self.time.second()
    }

    /// Replaces the time of day, keeping the date.
    pub(crate) const fn with_time(self, time: CivilTime) -> CivilDateTime {
        CivilDateTime { time, ..self }
    }

    /// Adds a signed number of seconds to the wall clock, returning the result
    /// and the number of days it moved.
    ///
    /// The arithmetic is done on the wall clock, so it never skips or repeats a
    /// time. Daylight saving time is applied later, when the result is resolved
    /// in a time zone.
    pub(crate) fn checked_add_seconds(self, seconds: i64) -> Option<(CivilDateTime, i64)> {
        let total = self.time.seconds_of_day().checked_add(seconds)?;
        // Croner searches in steps of a second, a minute or an hour, so the
        // result almost always stays on the same day.
        let days = total.div_euclid(SECONDS_PER_DAY);
        let date = if days == 0 {
            self.date
        } else {
            self.date.checked_add_days(days)?
        };
        let moved = CivilDateTime {
            date,
            time: CivilTime::from_seconds_of_day(total.rem_euclid(SECONDS_PER_DAY)),
        };
        Some((moved, days))
    }
}

impl core::fmt::Display for CivilDate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl core::fmt::Display for CivilDateTime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}T{}", self.date, self.time)
    }
}

/// The result of resolving a [`CivilDateTime`] in a time zone.
///
/// A wall clock time is not always one instant. When daylight saving time
/// starts, a range of wall clock times never happens, and when it ends, a range
/// happens twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution<T> {
    /// The wall clock time happens exactly once.
    Single(T),

    /// The wall clock time happens twice, because the clock was set back.
    /// The first value is the earlier instant.
    Ambiguous(T, T),

    /// The wall clock time never happens, because the clock was set forward.
    Gap,
}

/// A date and time type that croner can search over.
///
/// Croner is generic over this trait, so [`Cron::find_next_occurrence`],
/// [`Cron::is_time_matching`] and the iterators return the same type that you
/// give them. Implement it to use a date and time library that croner does not
/// include.
///
/// [`Cron::find_next_occurrence`]: crate::Cron::find_next_occurrence
/// [`Cron::is_time_matching`]: crate::Cron::is_time_matching
///
/// # Example
///
/// ```
/// # #[cfg(feature = "chrono")] {
/// use std::str::FromStr as _;
///
/// use chrono::Utc;
/// use croner::Cron;
///
/// let cron = Cron::from_str("0 0 * * FRI").unwrap();
///
/// // The return type follows the argument type.
/// let next: chrono::DateTime<Utc> = cron.find_next_occurrence(&Utc::now(), false).unwrap();
/// # }
/// ```
pub trait CronDateTime: Sized + Clone {
    /// Returns the local wall clock date and time.
    fn to_civil(&self) -> CivilDateTime;

    /// Returns the day of the week of the local wall clock date.
    ///
    /// Croner asks for this once per search and then keeps it in step as it
    /// moves through the calendar, so a backend never has to work it out from a
    /// date that croner made up.
    ///
    /// The name keeps this apart from the `weekday` methods that date and time
    /// libraries define on their own types, which would otherwise be ambiguous
    /// wherever both traits are in scope.
    fn civil_weekday(&self) -> Weekday;

    /// Resolves a wall clock date and time in the same time zone as `self`.
    ///
    /// The returned values must carry the given wall clock time. Croner relies
    /// on this to check a pattern once for both halves of an ambiguous time.
    ///
    /// Types without a time zone always return [`Resolution::Single`].
    fn resolve_civil(&self, civil: CivilDateTime) -> Result<Resolution<Self>, CronError>;

    /// Adds a signed number of seconds of elapsed time.
    ///
    /// This moves along the absolute time line, so a daylight saving time shift
    /// changes the wall clock result. Returns `None` on overflow.
    fn checked_add_seconds(&self, seconds: i64) -> Option<Self>;
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
