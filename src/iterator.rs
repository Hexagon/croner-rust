use crate::{Cron, CronError, Direction};
use chrono::{DateTime, TimeZone};

/// An iterator over the occurrences of a cron schedule.
/// It can iterate both forwards and backwards in time.
pub struct CronIterator<Tz>
where
    Tz: TimeZone,
{
    cron: Cron,
    current_time: DateTime<Tz>,
    is_first: bool,
    inclusive: bool,
    direction: Direction,
}

impl<Tz> CronIterator<Tz>
where
    Tz: TimeZone,
{
    /// Creates a new `CronIterator`.
    ///
    /// # Arguments
    ///
    /// * `cron` - The `Cron` schedule instance.
    /// * `start_time` - The `DateTime` to start iterating from.
    /// * `inclusive` - Whether the `start_time` should be included in the results if it matches.
    /// * `direction` - The direction to iterate in (Forward or Backward).
    pub fn new(cron: Cron, start_time: DateTime<Tz>, inclusive: bool, direction: Direction) -> Self {
        CronIterator {
            cron,
            current_time: start_time,
            is_first: true,
            inclusive,
            direction,
        }
    }
}

impl<Tz> Iterator for CronIterator<Tz>
where
    Tz: TimeZone,
{
    type Item = DateTime<Tz>;

    /// Finds the next or previous occurrence of the cron schedule based on the iterator's direction.
    fn next(&mut self) -> Option<Self::Item> {
        // Determine if the search should be inclusive based on whether it's the first run.
        let inclusive_search = if self.is_first {
            self.is_first = false;
            self.inclusive
        } else {
            false // Subsequent searches are always exclusive of the last found time.
        };

        let result = match self.direction {
            Direction::Forward => self
                .cron
                .find_next_occurrence(&self.current_time, inclusive_search),
            Direction::Backward => self
                .cron
                .find_previous_occurrence(&self.current_time, inclusive_search),
        };

        match result {
            Ok(found_time) => {
                // Update the current time to the found occurrence for the next iteration.
                self.current_time = found_time.clone();
                Some(found_time)
            }
            Err(CronError::TimeSearchLimitExceeded) => None, // Stop if we hit the year limit.
            Err(_) => None, // Stop the iteration on any other error.
        }
    }
}
