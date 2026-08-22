use crate::time::CronDateTime;
use crate::{Cron, CronError, Direction};

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct CronIterator<T>
where
    T: CronDateTime,
{
    cron: Cron,
    current_time: T,
    is_first: bool,
    inclusive: bool,
    direction: Direction,
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
        }
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
            Err(CronError::TimeSearchLimitExceeded) => None,
            Err(e) => {
                eprintln!("CronIterator encountered an error: {e:?}");
                None
            }
        }
    }
}
