use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use crate::time::CronDateTime;
use crate::{Cron, CronError, Direction};

/// Iterator over the occurrences of a [`Cron`] pattern.
///
/// Equality, ordering and hashing consider the schedule and the iteration
/// position only, not [`last_error`](CronIterator::last_error).
#[derive(Debug, Clone)]
pub struct CronIterator<T>
where
    T: CronDateTime,
{
    cron: Cron,
    current_time: T,
    is_first: bool,
    inclusive: bool,
    direction: Direction,
    last_error: Option<CronError>,
}

impl<T> CronIterator<T>
where
    T: CronDateTime,
{
    /// The fields that make up the iterator's identity.
    fn key(&self) -> (&Cron, &T, bool, bool, Direction) {
        (
            &self.cron,
            &self.current_time,
            self.is_first,
            self.inclusive,
            self.direction,
        )
    }
}

impl<T> PartialEq for CronIterator<T>
where
    T: CronDateTime + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl<T> PartialOrd for CronIterator<T>
where
    T: CronDateTime + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.key().partial_cmp(&other.key())
    }
}

impl<T> Hash for CronIterator<T>
where
    T: CronDateTime + Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl<T> CronIterator<T>
where
    T: CronDateTime,
{
    /// Creates a new `CronIterator`.
    ///
    /// # Arguments
    ///
    /// * `cron` - The `Cron` schedule instance.
    /// * `start_time` - The date and time to start iterating from.
    /// * `inclusive` - Whether the `start_time` should be included in the results if it matches.
    /// * `direction` - The direction to iterate in (Forward or Backward).
    pub fn new(cron: Cron, start_time: T, inclusive: bool, direction: Direction) -> Self {
        CronIterator {
            cron,
            current_time: start_time,
            is_first: true,
            inclusive,
            direction,
            last_error: None,
        }
    }

    /// Returns the error that ended the iteration, if any.
    ///
    /// When [`next`](Iterator::next) returns `None`, this tells a search that
    /// failed (for example [`CronError::TimeSearchLimitExceeded`]) apart from
    /// one that simply ran out of occurrences.
    pub fn last_error(&self) -> Option<&CronError> {
        self.last_error.as_ref()
    }
}

impl<T> Iterator for CronIterator<T>
where
    T: CronDateTime,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        // Determine if the search should be inclusive based on whether it's the first run.
        let inclusive_search = if self.is_first {
            self.is_first = false;
            self.inclusive
        } else {
            false // Subsequent searches are always exclusive of the last actual point in time.
        };

        let result =
            self.cron
                .find_occurrence(&self.current_time, inclusive_search, self.direction);

        match result {
            Ok(found_time) => {
                // Resume from this exact instant, exclusively: a step past it
                // could skip a match, and the instant keeps the half of a
                // repeated hour that the next search must continue from.
                self.current_time = found_time.clone();
                Some(found_time)
            }
            Err(e) => {
                #[cfg(feature = "std")]
                if !matches!(e, CronError::TimeSearchLimitExceeded) {
                    std::eprintln!("CronIterator encountered an error: {e:?}");
                }
                self.last_error = Some(e);
                None
            }
        }
    }
}

#[cfg(all(test, feature = "chrono"))]
mod tests {
    use core::str::FromStr as _;

    use chrono::{TimeZone as _, Utc};

    use crate::{Cron, CronError};

    #[test]
    fn last_error_is_none_while_iterating() {
        let cron = Cron::from_str("0 0 * * *").unwrap();
        let mut iter = cron.iter_after(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        assert!(iter.next().is_some());
        assert_eq!(iter.last_error(), None);
    }

    #[test]
    fn last_error_reports_why_iteration_stopped() {
        // February 30th never occurs, so the search gives up.
        let cron = Cron::from_str("0 0 30 2 *").unwrap();
        let mut iter = cron.iter_after(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        assert_eq!(iter.next(), None);
        assert_eq!(iter.last_error(), Some(&CronError::TimeSearchLimitExceeded));
    }

    #[test]
    fn equality_ignores_last_error() {
        let cron = Cron::from_str("0 0 30 2 *").unwrap();
        let start = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let mut failed = cron.iter_after(start);
        let mut fresh = cron.iter_after(start);
        failed.next();
        fresh.next();
        fresh.last_error = None;
        assert_eq!(failed, fresh);
    }
}
