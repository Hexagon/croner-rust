use crate::describe::Language;

#[derive(Default, Clone, Copy)]
pub struct English;

impl Language for English {
    fn every_minute(&self) -> &'static str { "Every minute" }
    fn every_second_phrase(&self) -> &'static str { "Every second" }
    fn every_x_minutes(&self, s: u16) -> String { format!("every {s} minutes") }
    fn every_x_seconds(&self, s: u16) -> String { format!("every {s} seconds") }
    fn every_x_hours(&self, s: u16) -> String { format!("of every {s} hours") }
    fn every_minute_of_every_x_hours(&self, s: u16) -> String { format!("Every minute, of every {s} hours") }
    
    fn at_time(&self, time: &str) -> String { format!("At {time}") }
    fn at_time_and_every_x_seconds(&self, time: &str, step: u16) -> String { format!("At {time}, every {step} seconds") }
    fn at_time_at_second(&self, time: &str, second: &str) -> String { format!("At {time}, at second {second}") }
    
    fn at_phrase(&self, phrase: &str) -> String { format!("At {phrase}") }
    fn on_phrase(&self, phrase: &str) -> String { format!("on {phrase}") }
    fn in_phrase(&self, phrase: &str) -> String { format!("in {phrase}") }
    
    fn second_phrase(&self, s: &str) -> String { format!("second {s}") }
    fn minute_phrase(&self, s: &str) -> String { format!("minute {s}") }
    fn minute_past_every_hour_phrase(&self, s: &str) -> String { format!("{s} past every hour") }
    fn hour_phrase(&self, s: &str) -> String { format!("of hour {s}") }
    fn year_phrase(&self, s: &str) -> String { format!("year {s}") }

    fn day_phrase(&self, s: &str) -> String { format!("day {s}") }
    fn the_last_day_of_the_month(&self) -> &'static str { "the last day of the month" }
    fn the_weekday_nearest_day(&self, day: &str) -> String { format!("the weekday nearest day {day}") }
    fn the_last_weekday_of_the_month(&self, day: &str) -> String { format!("the last {day} of the month") }
    
    fn the_nth_weekday_of_the_month(&self, n: u8, day: &str) -> String {
        let suffix = match n {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        };
        let num_str = format!("{n}{suffix}");
        format!("the {num_str} {day} of the month")
    }

    fn dom_and_dow_if_also(&self, dow: &str) -> String { format!("(if it is also {dow})") }
    fn dom_and_dow_if_also_one_of(&self, dow: &str) -> String { format!("(if it is also one of: {dow})") }

    fn list_conjunction_and(&self) -> &'static str { "and" }
    fn list_conjunction_or(&self) -> &'static str { "or" }
    fn list_conjunction_and_comma(&self) -> &'static str { ", and" }
    
    fn day_of_week_names(&self) -> [&'static str; 7] { ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"] }
    fn month_names(&self) -> [&'static str; 12] { ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"] }
}