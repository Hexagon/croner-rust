use crate::describe::Language;

#[derive(Default, Clone, Copy)]
pub struct English;

impl Language for English {
    fn every_minute(&self) -> &'static str { "Every minute" }
    fn every_second_phrase(&self) -> &'static str { "Every second" }
    fn every_x_minutes(&self, s: u8) -> String { format!("every {} minutes", s) }
    fn every_x_seconds(&self, s: u8) -> String { format!("every {} seconds", s) }
    fn every_x_hours(&self, s: u8) -> String { format!("of every {} hours", s) }
    fn every_minute_of_every_x_hours(&self, s: u8) -> String { format!("Every minute, of every {} hours", s) }
    
    fn at_time(&self, time: &str) -> String { format!("At {}", time) }
    fn at_time_and_every_x_seconds(&self, time: &str, step: u8) -> String { format!("At {}, every {} seconds", time, step) }
    fn at_time_at_second(&self, time: &str, second: &str) -> String { format!("At {}, at second {}", time, second) }
    
    fn at_phrase(&self, phrase: &str) -> String { format!("At {}", phrase) }
    fn on_phrase(&self, phrase: &str) -> String { format!("on {}", phrase) }
    fn in_phrase(&self, phrase: &str) -> String { format!("in {}", phrase) }
    
    fn second_phrase(&self, s: &str) -> String { format!("second {}", s) }
    fn minute_phrase(&self, s: &str) -> String { format!("minute {}", s) }
    fn minute_past_every_hour_phrase(&self, s: &str) -> String { format!("{} past every hour", s) }
    fn hour_phrase(&self, s: &str) -> String { format!("of hour {}", s) }

    fn day_phrase(&self, s: &str) -> String { format!("day {}", s) }
    fn the_last_day_of_the_month(&self) -> &'static str { "the last day of the month" }
    fn the_weekday_nearest_day(&self, day: &str) -> String { format!("the weekday nearest day {}", day) }
    fn the_last_weekday_of_the_month(&self, day: &str) -> String { format!("the last {} of the month", day) }
    
    fn the_nth_weekday_of_the_month(&self, n: u8, day: &str) -> String {
        let suffix = match n {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        };
        let num_str = format!("{}{}", n, suffix);
        format!("the {} {} of the month", num_str, day)
    }

    fn dom_and_dow_if_also(&self, dow: &str) -> String { format!("(if it is also {})", dow) }
    fn dom_and_dow_if_also_one_of(&self, dow: &str) -> String { format!("(if it is also one of: {})", dow) }

    fn list_conjunction_and(&self) -> &'static str { " and " }
    fn list_conjunction_or(&self) -> &'static str { " or " }
    fn list_conjunction_and_comma(&self) -> &'static str { ", and " }
    
    fn day_of_week_names(&self) -> [&'static str; 7] { ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"] }
    fn month_names(&self) -> [&'static str; 12] { ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"] }
}
