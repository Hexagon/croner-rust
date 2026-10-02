use crate::time::CronDateTime;
use crate::{Cron, CronError, Direction};

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

    /// Diagnostic field — excluded from `PartialEq`/`Hash` comparisons.
    last_error: Option<CronError>,
}

impl<T> PartialEq for CronIterator<T>
where
    T: CronDateTime + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.cron == other.cron
            && self.current_time == other.current_time
            && self.is_first == other.is_first
            && self.inclusive == other.inclusive
            && self.direction == other.direction
    }
}

impl<T> core::hash::Hash for CronIterator<T>
where
    T: CronDateTime + core::hash::Hash,
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.cron.hash(state);
        self.current_time.hash(state);
        self.is_first.hash(state);
        self.inclusive.hash(state);
        self.direction.hash(state);
    }
}

impl<T> PartialOrd for CronIterator<T>
where
    T: CronDateTime + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        match self.cron.partial_cmp(&other.cron) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.current_time.partial_cmp(&other.current_time) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.is_first.partial_cmp(&other.is_first) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.inclusive.partial_cmp(&other.inclusive) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.direction.partial_cmp(&other.direction)
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

    /// Returns the last error encountered during iteration, if any.
    ///
    /// When the iterator returns `None`, this method can be used to distinguish
    /// between a completed iteration (no more matches) and an error condition.
    /// This is especially useful in `no_std` environments where `eprintln!` is
    /// not available.
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
            Err(CronError::TimeSearchLimitExceeded) => {
                self.last_error = Some(CronError::TimeSearchLimitExceeded);
                None
            }
            Err(_e) => {
                #[cfg(feature = "std")]
                eprintln!("CronIterator encountered an error: {_e:?}");
                self.last_error = Some(_e);
                None
            }
        }
    }
}
