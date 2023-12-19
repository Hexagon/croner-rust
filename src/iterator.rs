use crate::Cron;
use chrono::{DateTime, Duration, TimeZone};

pub struct CronIterator<Tz>
where
    Tz: TimeZone,
{
    cron: Cron,
    current_time: DateTime<Tz>,
}

impl<Tz> CronIterator<Tz>
where
    Tz: TimeZone,
{
    pub fn new(cron: Cron, start_time: DateTime<Tz>) -> Self {
        CronIterator {
            cron,
            current_time: start_time,
        }
    }
}

impl<Tz> Iterator for CronIterator<Tz>
where
    Tz: TimeZone,
{
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.cron.find_next_occurrence(&self.current_time, true) {
            Ok(next_time) => {
                // Check if we can add one second without overflow
                let next_time_clone = next_time.clone();
                if let Some(updated_time) = next_time.checked_add_signed(Duration::seconds(1)) {
                    self.current_time = updated_time;
                    Some(next_time_clone) // Return the next time
                } else {
                    // If we hit an overflow, stop the iteration
                    None
                }
            }
            Err(_) => None, // Stop the iteration if we cannot find the next occurrence
        }
    }
}
