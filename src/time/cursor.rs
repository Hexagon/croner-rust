//! The moving position of croner's search through the calendar.

use crate::time::{days_in_month, days_in_year, CivilDateTime, CivilTime, CronDateTime, Weekday};

/// A wall clock date and time together with its day of the week.
///
/// Croner asks a backend for the weekday once, when a search starts, and this
/// type keeps it in step from then on: every move below is a known number of
/// days, so the new weekday is that many days along. Nothing in the crate
/// works out a weekday from a year, a month and a day.
///
/// Every move returns `None` on overflow rather than wrapping, which ends the
/// search with an error instead of a wrong answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cursor {
    civil: CivilDateTime,
    weekday: Weekday,
}

impl Cursor {
    /// Starts a search at the wall clock time of `start`.
    pub(crate) fn new<T: CronDateTime>(start: &T) -> Cursor {
        Cursor {
            civil: start.to_civil(),
            weekday: start.civil_weekday(),
        }
    }

    /// Returns the wall clock date and time.
    pub(crate) const fn civil(self) -> CivilDateTime {
        self.civil
    }

    /// Returns the day of the week.
    pub(crate) const fn weekday(self) -> Weekday {
        self.weekday
    }

    /// Returns the time of day.
    pub(crate) const fn time(self) -> CivilTime {
        self.civil.time()
    }

    pub(crate) const fn year(self) -> i32 {
        self.civil.year()
    }

    pub(crate) const fn month(self) -> u32 {
        self.civil.month()
    }

    pub(crate) const fn day(self) -> u32 {
        self.civil.day()
    }

    pub(crate) const fn hour(self) -> u32 {
        self.civil.hour()
    }

    pub(crate) const fn minute(self) -> u32 {
        self.civil.minute()
    }

    pub(crate) const fn second(self) -> u32 {
        self.civil.second()
    }

    /// Replaces the time of day. The date, and so the weekday, does not move.
    pub(crate) const fn with_time(self, time: CivilTime) -> Cursor {
        Cursor {
            civil: self.civil.with_time(time),
            weekday: self.weekday,
        }
    }

    /// Adds a signed number of seconds of wall clock time.
    pub(crate) fn checked_add_seconds(self, seconds: i64) -> Option<Cursor> {
        let (civil, days) = self.civil.checked_add_seconds(seconds)?;
        Some(Cursor {
            civil,
            weekday: self.weekday.shift(i32::try_from(days).ok()?),
        })
    }

    /// Adds a signed number of days, keeping the time of day.
    pub(crate) fn checked_add_days(self, days: i64) -> Option<Cursor> {
        Some(Cursor {
            civil: CivilDateTime::new(self.civil.date().checked_add_days(days)?, self.civil.time()),
            weekday: self.weekday.shift(i32::try_from(days).ok()?),
        })
    }

    /// Moves to the first moment of the next year.
    pub(crate) fn start_of_next_year(self) -> Option<Cursor> {
        // From this day to the last day of the year, then one more.
        let to_year_end = days_in_year(self.year()) - self.civil.date().day_of_year();
        self.checked_add_days(i64::from(to_year_end) + 1)
            .map(|cursor| cursor.with_time(CivilTime::MIDNIGHT))
    }

    /// Moves to the last moment of the previous year.
    pub(crate) fn end_of_previous_year(self) -> Option<Cursor> {
        // Stepping back by the day of the year lands on the last day of the
        // previous year.
        self.checked_add_days(-i64::from(self.civil.date().day_of_year()))
            .map(|cursor| cursor.with_time(CivilTime::END_OF_DAY))
    }

    /// Moves to the first moment of the next month.
    pub(crate) fn start_of_next_month(self) -> Option<Cursor> {
        // From this day to the last day of the month, then one more.
        let to_month_end = days_in_month(self.year(), self.month()) - self.day();
        self.checked_add_days(i64::from(to_month_end) + 1)
            .map(|cursor| cursor.with_time(CivilTime::MIDNIGHT))
    }

    /// Moves to the last moment of the previous month.
    pub(crate) fn end_of_previous_month(self) -> Option<Cursor> {
        // Stepping back by the day of the month lands on the last day of the
        // previous month.
        self.checked_add_days(-i64::from(self.day()))
            .map(|cursor| cursor.with_time(CivilTime::END_OF_DAY))
    }

    /// Resolves the wall clock time in the same time zone as `origin`.
    pub(crate) fn resolve_in<T: CronDateTime>(
        self,
        origin: &T,
    ) -> Result<super::Resolution<T>, crate::errors::CronError> {
        origin.resolve_civil(self.civil)
    }
}

impl core::fmt::Display for Cursor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.civil)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate};

    use super::*;

    // Builds a cursor straight from parts, the way `Cursor::new` would from a
    // backend, with the weekday taken from chrono.
    fn cursor(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> Cursor {
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("test date must exist");
        Cursor {
            civil: CivilDateTime::from_ymd_hms(year, month, day, hour, minute, second).unwrap(),
            weekday: Weekday::from_days_from_sunday(date.weekday().num_days_from_sunday()),
        }
    }

    // Fails if the cursor's carried weekday has drifted from the truth.
    #[track_caller]
    fn assert_agrees_with_calendar(actual: Cursor) {
        let expected = cursor(
            actual.year(),
            actual.month(),
            actual.day(),
            actual.hour(),
            actual.minute(),
            actual.second(),
        );
        assert_eq!(actual, expected, "weekday drifted at {actual}");
    }

    #[test]
    fn seconds_carry_into_the_next_and_previous_day() {
        let end_of_year = cursor(2023, 12, 31, 23, 59, 59);
        let new_year = end_of_year.checked_add_seconds(1).unwrap();
        assert_eq!(new_year.civil(), cursor(2024, 1, 1, 0, 0, 0).civil());
        assert_agrees_with_calendar(new_year);
        assert_agrees_with_calendar(new_year.checked_add_seconds(-1).unwrap());
        // An hour step lands on the previous day just as a second step does.
        assert_agrees_with_calendar(new_year.checked_add_seconds(-3600).unwrap());
    }

    #[test]
    fn month_and_year_moves_keep_the_weekday_in_step() {
        // Walk every day of a leap year and a common year, taking each move
        // from every day so that short and long months are all covered.
        for year in [2024, 2025] {
            let mut day = cursor(year, 1, 1, 12, 0, 0);
            while day.year() == year {
                assert_agrees_with_calendar(day);
                assert_agrees_with_calendar(day.start_of_next_month().unwrap());
                assert_agrees_with_calendar(day.end_of_previous_month().unwrap());
                assert_agrees_with_calendar(day.start_of_next_year().unwrap());
                assert_agrees_with_calendar(day.end_of_previous_year().unwrap());
                day = day.checked_add_days(1).unwrap();
            }
        }
    }

    #[test]
    fn moves_land_on_the_expected_dates() {
        let mid_february = cursor(2024, 2, 15, 8, 30, 15);
        assert_eq!(
            mid_february.start_of_next_month().unwrap().civil(),
            cursor(2024, 3, 1, 0, 0, 0).civil()
        );
        assert_eq!(
            mid_february.end_of_previous_month().unwrap().civil(),
            cursor(2024, 1, 31, 23, 59, 59).civil()
        );
        assert_eq!(
            mid_february.start_of_next_year().unwrap().civil(),
            cursor(2025, 1, 1, 0, 0, 0).civil()
        );
        assert_eq!(
            mid_february.end_of_previous_year().unwrap().civil(),
            cursor(2023, 12, 31, 23, 59, 59).civil()
        );
        // The leap day is reached from February, and skipped in a common year.
        assert_eq!(
            cursor(2024, 2, 28, 0, 0, 0)
                .checked_add_days(1)
                .unwrap()
                .civil(),
            cursor(2024, 2, 29, 0, 0, 0).civil()
        );
        assert_eq!(
            cursor(2023, 2, 28, 0, 0, 0)
                .checked_add_days(1)
                .unwrap()
                .civil(),
            cursor(2023, 3, 1, 0, 0, 0).civil()
        );
    }

    #[test]
    fn day_steps_cross_months_and_years() {
        let mut walked = cursor(2023, 11, 1, 0, 0, 0);
        let mut expected = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();
        // Cross a 30 day month, a 31 day month and a year boundary.
        for _ in 0..120 {
            assert_eq!(
                (walked.year(), walked.month(), walked.day()),
                (expected.year(), expected.month(), expected.day())
            );
            assert_agrees_with_calendar(walked);
            walked = walked.checked_add_days(1).unwrap();
            expected = expected.succ_opt().unwrap();
        }
        // And the same walk backwards.
        for _ in 0..120 {
            walked = walked.checked_add_days(-1).unwrap();
            expected = expected.pred_opt().unwrap();
            assert_eq!(
                (walked.year(), walked.month(), walked.day()),
                (expected.year(), expected.month(), expected.day())
            );
            assert_agrees_with_calendar(walked);
        }
    }
}
