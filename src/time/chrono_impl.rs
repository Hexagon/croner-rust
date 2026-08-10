//! [`CronDateTime`] implementations for the `chrono` crate.

use chrono::{
    DateTime, Datelike, LocalResult, NaiveDate, NaiveDateTime, TimeDelta, TimeZone, Timelike,
};

use crate::errors::CronError;
use crate::time::{CivilDate, CivilDateTime, CivilTime, CronDateTime, Resolution, Weekday};

/// Converts a `chrono` naive date and time to croner's civil type.
///
/// `chrono` keeps every part in a valid range, and it holds a leap second in
/// the nanosecond part, so this conversion always succeeds.
#[inline]
fn to_civil(naive: NaiveDateTime) -> CivilDateTime {
    // The parts are checked by `chrono`, so they are taken as they are.
    CivilDateTime::new(
        CivilDate::from_parts_unchecked(naive.year(), naive.month(), naive.day()),
        CivilTime::from_parts_unchecked(naive.hour(), naive.minute(), naive.second()),
    )
}

/// Converts croner's civil type to a `chrono` naive date and time.
#[inline]
fn to_naive(civil: CivilDateTime) -> Result<NaiveDateTime, CronError> {
    NaiveDate::from_ymd_opt(civil.year(), civil.month(), civil.day())
        .ok_or(CronError::InvalidDate)?
        .and_hms_opt(civil.hour(), civil.minute(), civil.second())
        .ok_or(CronError::InvalidTime)
}

/// Converts a `chrono` weekday to croner's.
#[inline]
fn to_weekday<D: Datelike>(date: &D) -> Weekday {
    Weekday::from_days_from_sunday(date.weekday().num_days_from_sunday())
}

impl<Tz: TimeZone> CronDateTime for DateTime<Tz> {
    #[inline]
    fn to_civil(&self) -> CivilDateTime {
        to_civil(self.naive_local())
    }

    #[inline]
    fn civil_weekday(&self) -> Weekday {
        to_weekday(&self.naive_local())
    }

    #[inline]
    fn resolve_civil(&self, civil: CivilDateTime) -> Result<Resolution<Self>, CronError> {
        let naive = to_naive(civil)?;
        Ok(match self.timezone().from_local_datetime(&naive) {
            LocalResult::Single(dt) => Resolution::Single(dt),
            LocalResult::Ambiguous(earlier, later) => Resolution::Ambiguous(earlier, later),
            LocalResult::None => Resolution::Gap,
        })
    }

    #[inline]
    fn checked_add_seconds(&self, seconds: i64) -> Option<Self> {
        self.clone()
            .checked_add_signed(TimeDelta::try_seconds(seconds)?)
    }
}

impl CronDateTime for NaiveDateTime {
    #[inline]
    fn to_civil(&self) -> CivilDateTime {
        to_civil(*self)
    }

    #[inline]
    fn civil_weekday(&self) -> Weekday {
        to_weekday(self)
    }

    #[inline]
    fn resolve_civil(&self, civil: CivilDateTime) -> Result<Resolution<Self>, CronError> {
        Ok(Resolution::Single(to_naive(civil)?))
    }

    #[inline]
    fn checked_add_seconds(&self, seconds: i64) -> Option<Self> {
        self.checked_add_signed(TimeDelta::try_seconds(seconds)?)
    }
}
