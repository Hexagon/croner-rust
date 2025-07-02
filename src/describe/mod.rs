// src/describe/mod.rs

pub mod lang;
pub use lang::english::English;

use crate::component::{
    CronComponent, ALL_BIT, CLOSEST_WEEKDAY_BIT, LAST_BIT, NTH_1ST_BIT, NTH_2ND_BIT, NTH_3RD_BIT,
    NTH_4TH_BIT, NTH_5TH_BIT,
};
use crate::pattern::CronPattern;

// This defines the contract for providing localized strings.
pub trait Language {
    fn every_minute(&self) -> &'static str;
    fn every_second_phrase(&self) -> &'static str;
    fn every_x_minutes(&self, step: u8) -> String;
    fn every_x_seconds(&self, step: u8) -> String;
    fn every_x_hours(&self, step: u8) -> String;
    fn every_minute_of_every_x_hours(&self, step: u8) -> String;

    fn at_time(&self, time: &str) -> String;
    fn at_time_and_every_x_seconds(&self, time: &str, step: u8) -> String;
    fn at_time_at_second(&self, time: &str, second: &str) -> String;
    
    fn at_phrase(&self, phrase: &str) -> String;
    fn on_phrase(&self, phrase: &str) -> String;
    fn in_phrase(&self, phrase: &str) -> String;

    fn second_phrase(&self, s: &str) -> String;
    fn minute_phrase(&self, s: &str) -> String;
    fn minute_past_every_hour_phrase(&self, s: &str) -> String;
    fn hour_phrase(&self, s: &str) -> String;

    fn day_phrase(&self, s: &str) -> String;
    fn the_last_day_of_the_month(&self) -> &'static str;
    fn the_weekday_nearest_day(&self, day: &str) -> String;
    fn the_last_weekday_of_the_month(&self, day: &str) -> String;
    fn the_nth_weekday_of_the_month(&self, n: u8, day: &str) -> String;

    fn dom_and_dow_if_also(&self, dow: &str) -> String;
    fn dom_and_dow_if_also_one_of(&self, dow: &str) -> String;

    fn list_conjunction_and(&self) -> &'static str;
    fn list_conjunction_or(&self) -> &'static str;
    fn list_conjunction_and_comma(&self) -> &'static str;
    
    fn day_of_week_names(&self) -> [&'static str; 7];
    fn month_names(&self) -> [&'static str; 12];
}

/// Generates a human-readable description for a `CronPattern`.
pub fn describe<L: Language>(pattern: &CronPattern, lang: &L) -> String {
    let time_desc = describe_time(pattern, lang);
    let day_desc = describe_day(pattern, lang);
    let month_desc = describe_month(pattern, lang);

    let mut parts = vec![];
    if !time_desc.is_empty() {
        parts.push(time_desc);
    }
    if !day_desc.is_empty() {
        parts.push(day_desc);
    }
    if !month_desc.is_empty() {
        parts.push(month_desc);
    }

    let mut description = parts.join(", ");
    if !description.is_empty() {
        let mut chars = description.chars();
        description = match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        };
        description.push('.');
    }
    description
}

fn describe_time<L: Language>(pattern: &CronPattern, lang: &L) -> String {
    let has_seconds = pattern.with_seconds_optional || pattern.with_seconds_required;
    let sec_vals = get_set_values(&pattern.seconds, ALL_BIT);
    let min_vals = get_set_values(&pattern.minutes, ALL_BIT);
    let hour_vals = get_set_values(&pattern.hours, ALL_BIT);

    let is_default_seconds = !has_seconds || (pattern.seconds.step == 1 && sec_vals.len() == 1 && sec_vals[0] == 0);
    let is_every_second = has_seconds && pattern.seconds.is_all_set();

    // Handle simplest cases first
    if is_every_second && pattern.minutes.is_all_set() && pattern.hours.is_all_set() {
        return lang.every_second_phrase().to_string();
    }
    if is_default_seconds && pattern.minutes.is_all_set() && pattern.hours.is_all_set() {
        return lang.every_minute().to_string();
    }
    if is_default_seconds && pattern.minutes.from_wildcard && pattern.minutes.step > 1 && pattern.hours.is_all_set() {
        return lang.at_phrase(&lang.every_x_minutes(pattern.minutes.step));
    }

    // Handle specific HH:MM time
    if !is_every_second && pattern.hours.step == 1 && hour_vals.len() == 1 && pattern.minutes.step == 1 && min_vals.len() == 1 {
        let hour = hour_vals[0];
        let minute = min_vals[0];
        let time_str = format!("{:02}:{:02}", hour, minute);
        
        if has_seconds && !is_default_seconds {
            if pattern.seconds.step > 1 && pattern.seconds.from_wildcard {
                return lang.at_time_and_every_x_seconds(&time_str, pattern.seconds.step);
            }
            if sec_vals.len() == 1 {
                return lang.at_time(&format!("{}:{:02}", time_str, sec_vals[0]));
            }
            return lang.at_time_at_second(&time_str, &format_number_list(&sec_vals, lang));
        }
        return lang.at_time(&time_str);
    }

    // Handle all other complex combinations
    let mut parts = vec![];
    
    if is_every_second {
        parts.push(lang.every_second_phrase().to_string());
    } else if has_seconds && !is_default_seconds {
        // Correctly handle ranged steps vs. wildcard steps
        if pattern.seconds.step > 1 && pattern.seconds.from_wildcard {
            parts.push(lang.every_x_seconds(pattern.seconds.step));
        } else {
            parts.push(lang.second_phrase(&format_number_list(&sec_vals, lang)));
        }
    }
    
    if pattern.minutes.from_wildcard && pattern.minutes.step > 1 {
        parts.push(lang.every_x_minutes(pattern.minutes.step));
    } else if !pattern.minutes.is_all_set() {
        let min_desc = lang.minute_phrase(&format_number_list(&min_vals, lang));
        if pattern.hours.is_all_set() && pattern.hours.step == 1 {
            parts.push(lang.minute_past_every_hour_phrase(&min_desc));
        } else {
            parts.push(min_desc);
        }
    }

    if !pattern.hours.is_all_set() {
        if pattern.hours.step > 1 {
            parts.push(lang.every_x_hours(pattern.hours.step));
        } else {
             parts.push(lang.hour_phrase(&format_number_list(&hour_vals, lang)));
        }
    }

    if parts.is_empty() {
        return lang.every_minute().to_string();
    }
    
    if parts.len() > 1 && parts[0] == lang.every_second_phrase() {
        return parts.join(", ");
    }
    
    lang.at_phrase(&parts.join(", "))
}


fn get_set_values(component: &CronComponent, bit: u8) -> Vec<u8> {
    (component.min..=component.max)
        .filter(|&i| component.is_bit_set(i, bit).unwrap_or(false))
        .collect()
}

fn format_text_list<L: Language>(items: Vec<String>, lang: &L) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{}{}{}", items[0], lang.list_conjunction_and(), items[1]),
        _ => {
            if let Some(last) = items.last() {
                let front = &items[..items.len() - 1];
                format!("{}{}{}", front.join(", "), lang.list_conjunction_and_comma(), last)
            } else {
                String::new()
            }
        }
    }
}

fn format_number_list<L: Language>(values: &[u8], lang: &L) -> String {
    if values.is_empty() { return String::new(); }
    let mut sorted_values = values.to_vec();
    sorted_values.sort_unstable();

    let mut items = vec![];
    let mut i = 0;
    while i < sorted_values.len() {
        let start = sorted_values[i];
        let mut j = i;
        while j + 1 < sorted_values.len() && sorted_values[j + 1] == sorted_values[j] + 1 {
            j += 1;
        }
        if j > i {
            items.push(format!("{}-{}", start, sorted_values[j]));
        } else {
            items.push(start.to_string());
        }
        i = j + 1;
    }
    format_text_list(items, lang)
}

fn describe_day<L: Language>(pattern: &CronPattern, lang: &L) -> String {
    let dom_desc = describe_dom(pattern, lang);
    let dow_parts = describe_dow_parts(pattern, lang);

    if pattern.star_dom && pattern.star_dow {
        return "".to_string();
    }
    
    if !pattern.star_dom && pattern.star_dow {
        return lang.on_phrase(&dom_desc);
    }
    
    let dow_desc = format_text_list(dow_parts.clone(), lang);
    if pattern.star_dom && !pattern.star_dow {
        return lang.on_phrase(&dow_desc);
    }

    if pattern.dom_and_dow {
        let final_phrase = if dow_parts.len() > 1 {
            lang.dom_and_dow_if_also_one_of(&dow_desc)
        } else {
            lang.dom_and_dow_if_also(&dow_desc)
        };
        format!("{} {}", lang.on_phrase(&dom_desc), final_phrase)
    } else {
        format!("{}{}{}", lang.on_phrase(&dom_desc), lang.list_conjunction_or(), dow_desc)
    }
}

fn describe_dom<L: Language>(pattern: &CronPattern, lang: &L) -> String {
    let mut parts = vec![];

    let regular_days = get_set_values(&pattern.days, ALL_BIT);
    if !regular_days.is_empty() {
        parts.push(lang.day_phrase(&format_number_list(&regular_days, lang)));
    }

    if pattern.days.is_feature_enabled(LAST_BIT) {
        parts.push(lang.the_last_day_of_the_month().to_string());
    }
    let weekday_values = get_set_values(&pattern.days, CLOSEST_WEEKDAY_BIT);
    if !weekday_values.is_empty() {
        parts.push(lang.the_weekday_nearest_day(&format_number_list(&weekday_values, lang)));
    }
    
    format_text_list(parts, lang)
}

fn describe_dow_parts<L: Language>(pattern: &CronPattern, lang: &L) -> Vec<String> {
    let mut parts = vec![];
    let dow_names_map = lang.day_of_week_names();
    let dow_names = if pattern.with_alternative_weekdays {
        &["", dow_names_map[0], dow_names_map[1], dow_names_map[2], dow_names_map[3], dow_names_map[4], dow_names_map[5], dow_names_map[6]]
    } else {
        &[dow_names_map[0], dow_names_map[1], dow_names_map[2], dow_names_map[3], dow_names_map[4], dow_names_map[5], dow_names_map[6], dow_names_map[0]]
    };

    let last_values = get_set_values(&pattern.days_of_week, LAST_BIT);
    if !last_values.is_empty() {
        let days = last_values.iter().map(|&v| dow_names[v as usize].to_string()).collect::<Vec<_>>();
        parts.push(lang.the_last_weekday_of_the_month(&format_text_list(days, lang)));
    }

    for (i, nth_bit) in [NTH_1ST_BIT, NTH_2ND_BIT, NTH_3RD_BIT, NTH_4TH_BIT, NTH_5TH_BIT].iter().enumerate() {
        let values = get_set_values(&pattern.days_of_week, *nth_bit);
        if !values.is_empty() {
            let days = values.iter().map(|&v| dow_names[v as usize].to_string()).collect::<Vec<_>>();
            parts.push(lang.the_nth_weekday_of_the_month((i + 1) as u8, &format_text_list(days, lang)));
        }
    }

    let regular_values = get_set_values(&pattern.days_of_week, ALL_BIT);
    if !regular_values.is_empty() {
        let is_continuous_range = if regular_values.len() > 2 {
            regular_values.windows(2).all(|w| w[1] == w[0] + 1)
        } else { false };

        let list = regular_values.iter().map(|&v| dow_names[v as usize].to_string()).collect::<Vec<_>>();
        if is_continuous_range {
            parts.push(list.join(","))
        } else {
            parts.push(format_text_list(list, lang));
        }
    }
    parts
}

fn describe_month<L: Language>(pattern: &CronPattern, lang: &L) -> String {
    if pattern.months.is_all_set() { return "".to_string(); }
    let month_names = lang.month_names();
    
    if pattern.months.step > 1 {
        return lang.in_phrase(&format!("every {}rd month", pattern.months.step));
    }
    
    let values = get_set_values(&pattern.months, ALL_BIT);
    let list = values.iter().map(|&v| month_names[v as usize -1].to_string()).collect::<Vec<_>>();
    lang.in_phrase(&format_text_list(list, lang))
}

#[cfg(test)]
mod tests {
    use super::lang::english::English;
    use super::Language;
    use crate::pattern::CronPattern;

    fn get_description_lang<L: Language + Default>(pattern_str: &str, lang: L) -> String {
        let mut p = CronPattern::new(pattern_str);
        if pattern_str.split_whitespace().count() == 6 {
            p.with_seconds_optional();
        }
        p.parse().unwrap();
        super::describe(&p, &lang)
    }

    fn get_description(pattern_str: &str) -> String {
        get_description_lang(pattern_str, English::default())
    }

    #[test]
    fn test_time_descriptions() {
        assert_eq!(get_description("* * * * *"), "Every minute.");
        assert_eq!(get_description("*/15 * * * *"), "At every 15 minutes.");
        assert_eq!(
            get_description("0 * * * *"),
            "At minute 0 past every hour."
        );
        assert_eq!(get_description("0 14 * * *"), "At 14:00.");
        
        assert_eq!(
            get_description("2,4,6 * * * *"),
            "At minute 2, 4, and 6 past every hour." 
        );
    
        assert_eq!(
            get_description("0 0-6 * * *"),
            "At minute 0, of hour 0-6."
        );
        assert_eq!(
            get_description("0 */2 * * *"),
            "At minute 0, of every 2 hours."
        );
    }

    #[test]
    fn test_seconds_descriptions() {
        let mut p1 = CronPattern::new("*/10 * * * * *");
        p1.with_seconds_optional();
        assert_eq!(p1.parse().unwrap().describe(), "At every 10 seconds.");

        let mut p2 = CronPattern::new("30 0 14 * * *");
        p2.with_seconds_optional();
        assert_eq!(p2.parse().unwrap().describe(), "At 14:00:30.");

        let mut p3 = CronPattern::new("10-20 0 14 * * *");
        p3.with_seconds_optional();
        assert_eq!(
            p3.parse().unwrap().describe(),
            "At 14:00, at second 10-20."
        );
    }

    #[test]
    fn test_day_descriptions() {
        assert_eq!(get_description("0 12 * * MON"), "At 12:00, on Monday.");
        assert_eq!(get_description("0 12 * * 1-5"), "At 12:00, on Monday,Tuesday,Wednesday,Thursday,Friday.");
        assert_eq!(get_description("0 12 15 * *"), "At 12:00, on day 15.");
        assert_eq!(get_description("0 12 L * *"), "At 12:00, on the last day of the month.");
        assert_eq!(
            get_description("0 12 1,15 * *"),
            "At 12:00, on day 1 and 15."
        );
    }

    #[test]
    fn test_month_descriptions() {
        assert_eq!(
            get_description("* * * JAN *"),
            "Every minute, in January."
        );
        assert_eq!(
            get_description("* * * 1,3,5 *"),
            "Every minute, in January, March, and May."
        );
    }

    #[test]
    fn test_special_char_descriptions() {
        assert_eq!(
            get_description("* * * * 5L"),
            "Every minute, on the last Friday of the month."
        );
        assert_eq!(
            get_description("* * * * TUE#3"),
            "Every minute, on the 3rd Tuesday of the month."
        );
        assert_eq!(
            get_description("* * 15W * *"),
            "Every minute, on the weekday nearest day 15."
        );
    }
    
    #[test]
    fn test_dom_and_dow_logic() {
        // Default behavior (OR)
        let pattern_or = CronPattern::new("0 0 15 * FRI").parse().unwrap();
        assert_eq!(pattern_or.describe(), "At 00:00, on day 15 or Friday.");

        // AND behavior
        let mut pattern_and = CronPattern::new("0 0 15 * FRI");
        pattern_and.with_dom_and_dow();
        let parsed_and = pattern_and.parse().unwrap();
        assert_eq!(parsed_and.describe(), "At 00:00, on day 15 (if it is also Friday).");
    }

    #[test]
    fn test_complex_combinations() {
         assert_eq!(
            get_description("30 18 15,L MAR *"),
            "At 18:30, on day 15 and the last day of the month, in March."
        );

        let mut p = CronPattern::new("30 18 15,L MAR FRI");
        p.with_dom_and_dow();
        assert_eq!(
            p.parse().unwrap().describe(),
            "At 18:30, on day 15 and the last day of the month (if it is also Friday), in March."
        );
    }

    #[test]
    fn test_second_and_minute_steps() {
        assert_eq!(
            get_description("* */2 * * * *"),
            "Every second, every 2 minutes."
        )
    }

    #[test]
    fn test_ranged_steps() {
        assert_eq!(
            get_description("18-28/2 * * * * *"),
            "At second 18, 20, 22, 24, 26, and 28."
        );
    }

    #[test]
    fn test_complex_dom_and_dow() {
        let mut p = CronPattern::new("0 0 1 * FRI#L,MON#1");
        p.with_dom_and_dow();
        assert_eq!(
            p.parse().unwrap().describe(),
            "At 00:00, on day 1 (if it is also one of: the last Friday of the month and the 1st Monday of the month)."
        );
    }
}
