//! Telling the two halves of a repeated wall clock range apart.
//!
//! When a time zone puts its clock back, a range of wall clock times happens
//! twice. Croner searches on the wall clock, which passes that range only
//! once, so a search must know which half it is on. These helpers read an
//! instant's half and find the edge between the halves.

use crate::errors::CronError;
use crate::time::{CronDateTime, Resolution, SECONDS_PER_DAY};

/// Which half of a repeated wall clock range an instant is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fold {
    /// Before the clock went back.
    Earlier,

    /// After the clock went back.
    Later,
}

/// The furthest a half of a repeated range can reach. No time zone repeats
/// more than a few hours, so a day is a wide margin for the bisection below.
const MAX_OVERLAP_SECONDS: i64 = SECONDS_PER_DAY;

/// Returns the half of a repeated range that `instant` is on, or `None` if its
/// wall clock time happens only once.
pub(crate) fn fold_of<T: CronDateTime>(instant: &T) -> Result<Option<Fold>, CronError> {
    match instant.resolve_civil(instant.to_civil())? {
        // Order against the later twin, not equality with the earlier one:
        // `instant` may carry sub-seconds that `CivilDateTime` drops.
        Resolution::Ambiguous(_, later) => Ok(Some(if instant.cmp_instant(&later).is_lt() {
            Fold::Earlier
        } else {
            Fold::Later
        })),
        _ => Ok(None),
    }
}

/// Returns the nearest instant of the half that `origin` is not on: the
/// moment the clock goes back, or the moment just before it. A walk from
/// here covers that whole half.
///
/// The caller must already know that `origin` is on the `on` half.
pub(crate) fn other_fold_edge<T: CronDateTime>(origin: &T, on: Fold) -> Result<T, CronError> {
    // The edge lies toward the change of clock: forward of the earlier half,
    // backward of the later one.
    let step = match on {
        Fold::Earlier => 1,
        Fold::Later => -1,
    };
    let at_offset = |offset: i64| -> Result<T, CronError> {
        origin
            .checked_add_seconds(offset * step)
            .ok_or(CronError::InvalidTime)
    };
    let is_inside =
        |offset: i64| -> Result<bool, CronError> { Ok(fold_of(&at_offset(offset)?)? == Some(on)) };

    // `is_inside` flips exactly once between `origin` and the margin, at the
    // change of clock. Bisect to the flip: about seventeen `resolve_civil`
    // calls, paid only on a day whose clock goes back.
    if is_inside(MAX_OVERLAP_SECONDS)? {
        return Err(CronError::TimeSearchLimitExceeded);
    }
    let mut inside_offset = 0;
    let mut outside_offset = MAX_OVERLAP_SECONDS;
    while outside_offset - inside_offset > 1 {
        let middle = inside_offset + (outside_offset - inside_offset) / 2;
        if is_inside(middle)? {
            inside_offset = middle;
        } else {
            outside_offset = middle;
        }
    }
    at_offset(outside_offset)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeDelta, TimeZone};
    use chrono_tz::Europe::Paris;
    use chrono_tz::Tz;

    use super::*;

    // On 2024-10-27 Paris clocks go 03:00 CEST back to 02:00 CET, so every wall
    // clock time from 02:00 to 02:59:59 happens twice. The change itself is
    // 01:00 UTC.
    fn paris(hour: u32, minute: u32, second: u32, fold: Fold) -> DateTime<Tz> {
        let ambiguous = Paris.with_ymd_and_hms(2024, 10, 27, hour, minute, second);
        match fold {
            Fold::Earlier => ambiguous.earliest(),
            Fold::Later => ambiguous.latest(),
        }
        .expect("the test time must exist")
    }

    #[test]
    fn reads_the_half_that_an_instant_is_on() {
        assert_eq!(
            fold_of(&paris(2, 30, 0, Fold::Earlier)).unwrap(),
            Some(Fold::Earlier)
        );
        assert_eq!(
            fold_of(&paris(2, 30, 0, Fold::Later)).unwrap(),
            Some(Fold::Later)
        );
        // Either side of the repeated range happens only once.
        assert_eq!(fold_of(&paris(1, 30, 0, Fold::Earlier)).unwrap(), None);
        assert_eq!(fold_of(&paris(3, 30, 0, Fold::Earlier)).unwrap(), None);
    }

    #[test]
    fn reads_the_half_of_an_instant_with_sub_second_precision() {
        // The half second is added in absolute time: chrono refuses to set
        // a field on an ambiguous local time.
        for fold in [Fold::Earlier, Fold::Later] {
            let instant = paris(2, 30, 0, fold) + TimeDelta::milliseconds(500);
            assert_eq!(fold_of(&instant).unwrap(), Some(fold));
        }
    }

    #[test]
    fn finds_the_edge_from_anywhere_in_the_range() {
        let change = paris(2, 0, 0, Fold::Later);
        let before_change = change.checked_add_seconds(-1).unwrap();

        // Every point on a half reports the same edge.
        for (hour, minute, second) in [(2, 0, 0), (2, 30, 0), (2, 59, 59)] {
            let earlier = paris(hour, minute, second, Fold::Earlier);
            assert_eq!(other_fold_edge(&earlier, Fold::Earlier).unwrap(), change);

            let later = paris(hour, minute, second, Fold::Later);
            assert_eq!(other_fold_edge(&later, Fold::Later).unwrap(), before_change);
        }
    }
}
