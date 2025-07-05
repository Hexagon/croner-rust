use crate::describe::Language;

#[derive(Default, Clone, Copy)]
pub struct Swedish;

impl Language for Swedish {
    fn every_minute(&self) -> &'static str { "Varje minut" }
    fn every_second_phrase(&self) -> &'static str { "Varje sekund" }
    fn every_x_minutes(&self, s: u8) -> String { format!("var {}:e minut", s) }
    fn every_x_seconds(&self, s: u8) -> String { format!("var {}:e sekund", s) }
    fn every_x_hours(&self, s: u8) -> String { format!("var {}:e timme", s) }
    fn every_minute_of_every_x_hours(&self, s: u8) -> String { format!("Varje minut, var {}:e timme", s) }
    
    fn at_time(&self, time: &str) -> String { format!("Klockan {}", time) }
    fn at_time_and_every_x_seconds(&self, time: &str, step: u8) -> String { format!("Klockan {}, var {}:e sekund", time, step) }
    fn at_time_at_second(&self, time: &str, second: &str) -> String { format!("Klockan {}, på sekund {}", time, second) }
    
    fn at_phrase(&self, phrase: &str) -> String { format!("Vid {}", phrase) }
    fn on_phrase(&self, phrase: &str) -> String { format!("på {}", phrase) }
    fn in_phrase(&self, phrase: &str) -> String { format!("i {}", phrase) }
    
    fn second_phrase(&self, s: &str) -> String { format!("sekund {}", s) }
    fn minute_phrase(&self, s: &str) -> String { format!("minut {}", s) }
    fn minute_past_every_hour_phrase(&self, s: &str) -> String { format!("{} över varje heltimme", s) }
    fn hour_phrase(&self, s: &str) -> String { format!("timme {}", s) }

    fn day_phrase(&self, s: &str) -> String { format!("dag {}", s) }
    fn the_last_day_of_the_month(&self) -> &'static str { "sista dagen i månaden" }
    fn the_weekday_nearest_day(&self, day: &str) -> String { format!("veckodagen närmast dag {}", day) }
    fn the_last_weekday_of_the_month(&self, day: &str) -> String { format!("sista {} i månaden", day) }
    
    fn the_nth_weekday_of_the_month(&self, n: u8, day: &str) -> String {
        let ordinal = match n {
            1 => "första",
            2 => "andra",
            3 => "tredje",
            4 => "fjärde",
            5 => "femte",
            _ => "", // Should not happen with cron's # specifier
        };
        format!("den {} {} i månaden", ordinal, day)
    }

    fn dom_and_dow_if_also(&self, dow: &str) -> String { format!("(om det också är {})", dow) }
    fn dom_and_dow_if_also_one_of(&self, dow: &str) -> String { format!("(om det också är en av: {})", dow) }

    fn list_conjunction_and(&self) -> &'static str { " och " }
    fn list_conjunction_or(&self) -> &'static str { " eller " }
    fn list_conjunction_and_comma(&self) -> &'static str { " och " } // Oxford comma is not used in Swedish
    
    fn day_of_week_names(&self) -> [&'static str; 7] { ["söndag", "måndag", "tisdag", "onsdag", "torsdag", "fredag", "lördag"] }
    fn month_names(&self) -> [&'static str; 12] { ["januari", "februari", "mars", "april", "maj", "juni", "juli", "augusti", "september", "oktober", "november", "december"] }
}