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
    pending_ambiguous_dt: Option<T>,
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
            pending_ambiguous_dt: None,
        }
    }
}

impl<T> Iterator for CronIterator<T>
where
    T: CronDateTime,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        // Step 1: Check for and yield a pending ambiguous datetime first.
        // This handles the second occurrence of a time during DST fallback.
        if let Some(pending_dt_to_yield) = self.pending_ambiguous_dt.take() {
            // After yielding the second ambiguous time, advance current_time past it.
            self.current_time = pending_dt_to_yield.checked_add_seconds(self.direction.step())?;
            return Some(pending_dt_to_yield);
        }

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
            Ok((found_time, optional_second_ambiguous_dt)) => {
                // This `found_time` is the one we will return in this iteration.

                // If there's a second ambiguous datetime (for interval jobs),
                // store it to be yielded on the *next* call to next().
                // And importantly, set `self.current_time` to advance *past* this second ambiguous time
                // so the *next* search for a *new* naive time is correct.
                if let Some(second_ambiguous_dt) = optional_second_ambiguous_dt {
                    // Advance `self.current_time` past the latest of the ambiguous pair.
                    // This ensures the next `find_occurrence` call searches for the next unique naive time.
                    self.current_time =
                        second_ambiguous_dt.checked_add_seconds(self.direction.step())?;
                    self.pending_ambiguous_dt = Some(second_ambiguous_dt);
                } else {
                    // Case: No second ambiguous time (either not an overlap, or fixed-time job).
                    // Advance `self.current_time` simply past the `found_time`.
                    self.current_time = found_time.checked_add_seconds(self.direction.step())?;
                }

                // Finally, return the found_time for the current iteration.
                // This `found_time` is the original value received from `find_occurrence`.
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
