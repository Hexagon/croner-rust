//! [`CronDateTime`] implementations for the `jiff` crate.

use core::cmp::Ordering;

use jiff::civil::DateTime;
use jiff::tz::AmbiguousOffset;
use jiff::{SignedDuration, Zoned};

use crate::errors::CronError;
use crate::time::{CivilDate, CivilDateTime, CivilTime, CronDateTime, Resolution, Weekday};

/// Converts a `jiff` civil date and time to croner's civil type.
///
/// `jiff` keeps every part in a valid range, so this conversion always
/// succeeds.
#[inline]
fn to_civil(datetime: DateTime) -> CivilDateTime {
    // The parts are checked by `jiff`, so they are taken as they are.
    CivilDateTime::new(
        CivilDate::from_parts_unchecked(
            i32::from(datetime.year()),
            datetime.month() as u32,
            datetime.day() as u32,
        ),
        CivilTime::from_parts_unchecked(
            datetime.hour() as u32,
            datetime.minute() as u32,
            datetime.second() as u32,
        ),
    )
}

/// Converts croner's civil type to a `jiff` civil date and time.
///
/// This fails outside the year range that `jiff` supports, -9999 to 9999.
#[inline]
fn to_jiff(civil: CivilDateTime) -> Result<DateTime, CronError> {
    let year = i16::try_from(civil.year()).map_err(|_| CronError::InvalidDate)?;
    DateTime::new(
        year,
        civil.month() as i8,
        civil.day() as i8,
        civil.hour() as i8,
        civil.minute() as i8,
        civil.second() as i8,
        0,
    )
    .map_err(|_| CronError::InvalidDate)
}

/// Converts a `jiff` weekday to croner's.
#[inline]
fn to_weekday(weekday: jiff::civil::Weekday) -> Weekday {
    Weekday::from_days_from_sunday(weekday.to_sunday_zero_offset() as u32)
}

impl CronDateTime for Zoned {
    #[inline]
    fn to_civil(&self) -> CivilDateTime {
        to_civil(self.datetime())
    }

    #[inline]
    fn civil_weekday(&self) -> Weekday {
        to_weekday(self.weekday())
    }

    #[inline]
    fn resolve_civil(&self, civil: CivilDateTime) -> Result<Resolution<Self>, CronError> {
        let datetime = to_jiff(civil)?;
        let ambiguous = self.time_zone().to_ambiguous_zoned(datetime);
        Ok(match ambiguous.offset() {
            AmbiguousOffset::Unambiguous { .. } => Resolution::Single(
                ambiguous
                    .unambiguous()
                    .map_err(|_| CronError::InvalidTime)?,
            ),
            AmbiguousOffset::Fold { .. } => {
                let earlier = ambiguous
                    .clone()
                    .earlier()
                    .map_err(|_| CronError::InvalidTime)?;
                let later = ambiguous.later().map_err(|_| CronError::InvalidTime)?;
                Resolution::Ambiguous(earlier, later)
            }
            AmbiguousOffset::Gap { .. } => Resolution::Gap,
        })
    }

    #[inline]
    fn checked_add_seconds(&self, seconds: i64) -> Option<Self> {
        self.checked_add(SignedDuration::from_secs(seconds)).ok()
    }

    #[inline]
    fn cmp_instant(&self, other: &Self) -> Ordering {
        // The timestamp is the instant a `Zoned` names, so the two halves
        // of a repeated wall clock range compare unequal.
        self.timestamp().cmp(&other.timestamp())
    }
}

impl CronDateTime for DateTime {
    #[inline]
    fn to_civil(&self) -> CivilDateTime {
        to_civil(*self)
    }

    #[inline]
    fn civil_weekday(&self) -> Weekday {
        to_weekday(self.weekday())
    }

    #[inline]
    fn resolve_civil(&self, civil: CivilDateTime) -> Result<Resolution<Self>, CronError> {
        Ok(Resolution::Single(to_jiff(civil)?))
    }

    #[inline]
    fn checked_add_seconds(&self, seconds: i64) -> Option<Self> {
        self.checked_add(SignedDuration::from_secs(seconds)).ok()
    }

    #[inline]
    fn cmp_instant(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }
}
