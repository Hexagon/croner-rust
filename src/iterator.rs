use crate::{Cron, CronError, Direction};
use chrono::{DateTime, TimeZone, Duration};

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct CronIterator<Tz>
where
    Tz: TimeZone,
{
    cron: Cron,
    current_time: DateTime<Tz>,
    is_first: bool,
    inclusive: bool,
    direction: Direction,
    pending_ambiguous_dt: Option<DateTime<Tz>>,
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
    pub fn new(
        cron: Cron,
        start_time: DateTime<Tz>,
        inclusive: bool,
        direction: Direction,
    ) -> Self {
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

impl<Tz> Iterator for CronIterator<Tz>
where
    Tz: TimeZone + Clone + Copy,
{
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        // Step 1: Check for and yield a pending ambiguous datetime first.
        // This handles the second occurrence of a time during DST fallback.
        if let Some(pending_dt_to_yield) = self.pending_ambiguous_dt.take() {
            // After yielding the second ambiguous time, advance current_time past it.
            // Clone pending_dt_to_yield because it's about to be returned,
            // but we need its value to calculate the next `self.current_time`.
            self.current_time = pending_dt_to_yield.clone().checked_add_signed(match self.direction { // Fixed E0382: pending_dt_to_yield
                Direction::Forward => Duration::seconds(1),
                Direction::Backward => Duration::seconds(-1),
            }).ok_or(CronError::InvalidTime).ok()?;
            return Some(pending_dt_to_yield);
        }

        // Determine if the search should be inclusive based on whether it's the first run.
        let inclusive_search = if self.is_first {
            self.is_first = false;
            self.inclusive
        } else {
            false // Subsequent searches are always exclusive of the last actual point in time.
        };

        loop {
            let result = self.cron.find_occurrence(&self.current_time, inclusive_search, self.direction);

            match result {
                Ok((found_time, optional_second_ambiguous_dt)) => {
                    // This `found_time` is the one we will return in this iteration.

                    // If there's a second ambiguous datetime (for interval jobs),
                    // store it to be yielded on the *next* call to next().
                    // And importantly, set `self.current_time` to advance *past* this second ambiguous time
                    // so the *next* search for a *new* naive time is correct.
                    if let Some(second_ambiguous_dt) = optional_second_ambiguous_dt {
                        // Clone second_ambiguous_dt because it's stored in self.pending_ambiguous_dt
                        // AND used to calculate the next self.current_time.
                        self.pending_ambiguous_dt = Some(second_ambiguous_dt.clone()); // Fixed E0382: second_ambiguous_dt

                        // Advance `self.current_time` past the latest of the ambiguous pair.
                        // This ensures the next `find_occurrence` call searches for the next unique naive time.
                        self.current_time = second_ambiguous_dt.checked_add_signed(match self.direction {
                            Direction::Forward => Duration::seconds(1),
                            Direction::Backward => Duration::seconds(-1),
                        }).ok_or(CronError::InvalidTime).ok()?;

                    } else {
                        // Case: No second ambiguous time (either not an overlap, or fixed-time job).
                        // Advance `self.current_time` simply past the `found_time`.
                        // Clone found_time because it's used to calculate the next self.current_time
                        // AND returned at the end of this block.
                        self.current_time = found_time.clone().checked_add_signed(match self.direction { // Fixed E0382: found_time
                            Direction::Forward => Duration::seconds(1),
                            Direction::Backward => Duration::seconds(-1),
                        }).ok_or(CronError::InvalidTime).ok()?;
                    }

                    // Finally, return the found_time for the current iteration.
                    // This `found_time` is the original value received from `find_occurrence`.
                    return Some(found_time);
                }
                Err(CronError::TimeSearchLimitExceeded) => return None,
                Err(e) => {
                    eprintln!("CronIterator encountered an error: {:?}", e);
                    return None;
                }
            }
        }
    }
}