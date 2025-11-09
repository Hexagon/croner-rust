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
    fn every_x_minutes(&self, step: u16) -> String; // Changed to u16
    fn every_x_seconds(&self, step: u16) -> String; // Changed to u16
    fn every_x_hours(&self, step: u16) -> String;   // Changed to u16
    fn every_minute_of_every_x_hours(&self, step: u16) -> String; // Changed to u16

    fn at_time(&self, time: &str) -> String;
    fn at_time_and_every_x_seconds(&self, time: &str, step: u16) -> String; // Changed to u16
    fn at_time_at_second(&self, time: &str, second: &str) -> String;

    fn at_phrase(&self, phrase: &str) -> String;
    fn on_phrase(&self, phrase: &str) -> String;
    fn in_phrase(&self, phrase: &str) -> String;

    fn second_phrase(&self, s: &str) -> String;
    fn minute_phrase(&self, s: &str) -> String;
    fn minute_past_every_hour_phrase(&self, s: &str) -> String;
    fn hour_phrase(&self, s: &str) -> String;
    fn year_phrase(&self, s: &str) -> String; // New for year

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
    let year_desc = describe_year(pattern, lang); // Add year description

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
    if !year_desc.is_empty() {
        parts.push(year_desc);
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

/// Helper function to determine if a component is fully set (like a wildcard `*`).
fn is_all_set(component: &CronComponent) -> bool {
    // We can't just check for a wildcard flag anymore.
    // A component is "all set" if every possible value is included.
    if component.step != 1 {
        return false;
    }
    let total_values = (component.max - component.min + 1) as usize;
    // Handle large ranges efficiently
    if total_values > 10000 { // Heuristic for very large ranges like year
        return component.from_wildcard;
    }
    let set_values = (component.min..=component.max)
        .filter(|i| component.is_bit_set(*i, ALL_BIT).unwrap_or(false)) // Corrected
        .count();
    total_values == set_values
}

fn describe_time<L: Language>(pattern: &CronPattern, lang: &L) -> String {

    let sec_vals = pattern.seconds.get_set_values(ALL_BIT);
    let min_vals = pattern.minutes.get_set_values(ALL_BIT);
    let hour_vals = pattern.hours.get_set_values(ALL_BIT);

    let is_default_seconds = pattern.seconds.step == 1 && sec_vals.len() == 1 && sec_vals[0] == 0;
    let is_every_second = is_all_set(&pattern.seconds);

    // Heuristic to detect `*/step` patterns, replacing `from_wildcard`.
    let is_stepped_from_start =
        |step: u16, vals: &[u16], min: u16| step > 1 && !vals.is_empty() && vals[0] == min;

    // Handle simplest cases first
    if is_every_second && is_all_set(&pattern.minutes) && is_all_set(&pattern.hours) {
        return lang.every_second_phrase().to_string();
    }
    if is_default_seconds && is_all_set(&pattern.minutes) && is_all_set(&pattern.hours) {
        return lang.every_minute().to_string();
    }
    if is_default_seconds
        && is_stepped_from_start(pattern.minutes.step, &min_vals, pattern.minutes.min)
        && is_all_set(&pattern.hours)
    {
        return lang.at_phrase(&lang.every_x_minutes(pattern.minutes.step));
    }

    // Handle specific HH:MM time
    if !is_every_second
        && pattern.hours.step == 1 && hour_vals.len() == 1
        && pattern.minutes.step == 1 && min_vals.len() == 1
    {
        let time_str = format!("{:02}:{:02}", hour_vals[0], min_vals[0]);

        if !is_default_seconds {
            if is_stepped_from_start(pattern.seconds.step, &sec_vals, pattern.seconds.min) {
                return lang.at_time_and_every_x_seconds(&time_str, pattern.seconds.step);
            }
            if sec_vals.len() == 1 {
                return lang.at_time(&format!("{}:{:02}", time_str, sec_vals[0]));
            }
            return lang.at_time_at_second(&time_str, &format_number_list(&sec_vals, lang));
        }
        return lang.at_time(&time_str);
    }

    // Special case: "* 0 * * *" -> "Every minute past hour 0"
    // When minutes are all set (wildcard) and hours are specific (not all set)
    if is_default_seconds && is_all_set(&pattern.minutes) && !is_all_set(&pattern.hours) {
        let hour_desc = if is_stepped_from_start(pattern.hours.step, &hour_vals, pattern.hours.min) {
            lang.every_x_hours(pattern.hours.step)
        } else {
            format!("hour {}", format_number_list(&hour_vals, lang))
        };
        return format!("{} past {}", lang.every_minute(), hour_desc);
    }

    // Special case: "* * 0 * * *" -> "Every second past hour 0"
    // When seconds and minutes are all set (wildcard) and hours are specific (not all set)
    if is_every_second && is_all_set(&pattern.minutes) && !is_all_set(&pattern.hours) {
        let hour_desc = if is_stepped_from_start(pattern.hours.step, &hour_vals, pattern.hours.min) {
            lang.every_x_hours(pattern.hours.step)
        } else {
            format!("hour {}", format_number_list(&hour_vals, lang))
        };
        return format!("{} past {}", lang.every_second_phrase(), hour_desc);
    }

    // Handle all other complex combinations
    let mut parts = vec![];

    if is_every_second {
        parts.push(lang.every_second_phrase().to_string());
    } else if !is_default_seconds {
        if is_stepped_from_start(pattern.seconds.step, &sec_vals, pattern.seconds.min) {
            parts.push(lang.every_x_seconds(pattern.seconds.step));
        } else {
            parts.push(lang.second_phrase(&format_number_list(&sec_vals, lang)));
        }
    }

    if is_stepped_from_start(pattern.minutes.step, &min_vals, pattern.minutes.min) {
        parts.push(lang.every_x_minutes(pattern.minutes.step));
    } else if !is_all_set(&pattern.minutes) {
        let min_desc = lang.minute_phrase(&format_number_list(&min_vals, lang));
        if is_all_set(&pattern.hours) && pattern.hours.step == 1 {
            parts.push(lang.minute_past_every_hour_phrase(&min_desc));
        } else {
            parts.push(min_desc);
        }
    }

    if !is_all_set(&pattern.hours) {
        if is_stepped_from_start(pattern.hours.step, &hour_vals, pattern.hours.min) {
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

fn format_text_list<L: Language>(items: Vec<String>, lang: &L) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} {} {}", items[0], lang.list_conjunction_and(), items[1]),
        _ => {
            if let Some(last) = items.last() {
                let front = &items[..items.len() - 1];
                format!("{}, {} {}", front.join(", "), lang.list_conjunction_and(), last)
            } else {
                String::new()
            }
        }
    }
}

fn format_number_list<L: Language>(values: &[u16], lang: &L) -> String {
    if values.is_empty() {
        return String::new();
    }
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
        if j > i + 1 { // Only create a range for 3 or more consecutive numbers
            items.push(format!("{}-{}", start, sorted_values[j]));
        } else {
            for k in sorted_values.iter().take(j + 1).skip(i) {
                items.push(k.to_string());
            }
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
        format!("{} {} {}", lang.on_phrase(&dom_desc), lang.list_conjunction_or(), dow_desc)
    }
}

fn describe_dom<L: Language>(pattern: &CronPattern, lang: &L) -> String {
    let mut parts = vec![];

    let regular_days = pattern.days.get_set_values(ALL_BIT);
    if !regular_days.is_empty() {
        parts.push(lang.day_phrase(&format_number_list(&regular_days, lang)));
    }

    if pattern.days.is_feature_enabled(LAST_BIT) {
        parts.push(lang.the_last_day_of_the_month().to_string());
    }
    let weekday_values = pattern.days.get_set_values(CLOSEST_WEEKDAY_BIT);
    if !weekday_values.is_empty() {
        parts.push(lang.the_weekday_nearest_day(&format_number_list(&weekday_values, lang)));
    }

    format_text_list(parts, lang)
}

fn describe_dow_parts<L: Language>(pattern: &CronPattern, lang: &L) -> Vec<String> {
    let mut parts = vec![];
    let dow_names_map = lang.day_of_week_names();

    // The `with_alternative_weekdays` flag is gone. Parser normalizes DOW.
    // This mapping handles the 0-7 range where 7 is Sunday.
    let dow_names = &[
        dow_names_map[0], dow_names_map[1], dow_names_map[2], dow_names_map[3],
        dow_names_map[4], dow_names_map[5], dow_names_map[6], dow_names_map[0],
    ];

    let last_values = pattern.days_of_week.get_set_values(LAST_BIT);
    if !last_values.is_empty() {
        let days = last_values.iter().map(|v| dow_names[*v as usize].to_string()).collect::<Vec<_>>(); // Corrected
        parts.push(lang.the_last_weekday_of_the_month(&format_text_list(days, lang)));
    }

    for (i, nth_bit) in [NTH_1ST_BIT, NTH_2ND_BIT, NTH_3RD_BIT, NTH_4TH_BIT, NTH_5TH_BIT].iter().enumerate() {
        let values = pattern.days_of_week.get_set_values(*nth_bit);
        if !values.is_empty() {
            let days = values.iter().map(|v| dow_names[*v as usize].to_string()).collect::<Vec<_>>(); // Corrected
            parts.push(lang.the_nth_weekday_of_the_month((i + 1) as u8, &format_text_list(days, lang)));
        }
    }

    let regular_values = pattern.days_of_week.get_set_values(ALL_BIT);
    if !regular_values.is_empty() {
        let list = regular_values.iter().map(|v| dow_names[*v as usize].to_string()).collect::<Vec<_>>(); // Corrected
        parts.push(format_text_list(list, lang));
    }
    parts
}

fn describe_month<L: Language>(pattern: &CronPattern, lang: &L) -> String {
    if is_all_set(&pattern.months) {
        return "".to_string();
    }
    let month_names = lang.month_names();

    if pattern.months.step > 1 {
        return lang.in_phrase(&format!("every {} months", pattern.months.step));
    }

    let values = pattern.months.get_set_values(ALL_BIT);
    let list = values
        .iter()
        .map(|v| month_names[*v as usize - 1].to_string()) // Corrected
        .collect::<Vec<_>>();
    lang.in_phrase(&format_text_list(list, lang))
}

fn describe_year<L: Language>(pattern: &CronPattern, lang: &L) -> String {
    if is_all_set(&pattern.years) {
        return "".to_string();
    }

    if pattern.years.step > 1 {
        return lang.in_phrase(&lang.year_phrase(&format!("every {}", pattern.years.step)));
    }
    
    let values = pattern.years.get_set_values(ALL_BIT);
    lang.in_phrase(&lang.year_phrase(&format_number_list(&values, lang)))
}


#[cfg(test)]
mod tests {
    use super::lang::english::English;
    use crate::parser::{CronParser, Seconds, Year};
    use super::Language;

    // Updated helper to use the new parser API
    fn get_description_lang_config<L: Language + Default>(
        pattern_str: &str,
        lang: L,
        seconds: Seconds,
        year: Year,
        dom_and_dow: bool,
    ) -> String {
        let cron = CronParser::builder()
            .seconds(seconds)
            .year(year)
            .dom_and_dow(dom_and_dow)
            .build()
            .parse(pattern_str)
            .expect("Failed to parse pattern for test");

        // The user wants to test the describe function in this module
        super::describe(&cron.pattern, &lang)
    }

    // Simplified helper for common cases.
    // It uses a permissive parser config that can handle any pattern
    // since the parser normalizes the pattern before describe is called.
    fn get_description(pattern_str: &str) -> String {
        get_description_lang_config(
            pattern_str,
            English,
            Seconds::Optional, // Be permissive
            Year::Optional,      // Be permissive
            false,
        )
    }

    #[test]
    fn test_time_descriptions() {
        assert_eq!(get_description("* * * * *"), "Every minute.");
        assert_eq!(get_description("*/15 * * * *"), "At every 15 minutes.");
        assert_eq!(get_description("0 * * * *"), "At minute 0 past every hour.");
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
        // Test for issue #35: "* 0 * * *" should describe properly
        assert_eq!(
            get_description("* 0 * * *"),
            "Every minute past hour 0."
        );
        assert_eq!(
            get_description("* 0,12 * * *"),
            "Every minute past hour 0 and 12."
        );
    }

    #[test]
    fn test_seconds_descriptions() {
        assert_eq!(get_description("*/10 * * * * *"), "At every 10 seconds.");
        assert_eq!(get_description("30 0 14 * * *"), "At 14:00:30.");
        assert_eq!(
            get_description("10-20 0 14 * * *"),
            "At 14:00, at second 10-20."
        );
        // Test for similar issue as #35 with seconds
        assert_eq!(
            get_description("* * 0 * * *"),
            "Every second past hour 0."
        );
        assert_eq!(
            get_description("* * 5 * * *"),
            "Every second past hour 5."
        );
    }
    
    #[test]
    fn test_year_descriptions() {
        assert_eq!(
            get_description("0 0 0 1 1 * 2025"),
            "At 00:00, on day 1, in January, in year 2025."
        );
         assert_eq!(
            get_description("0 0 0 1 1 * 2025-2030"),
            "At 00:00, on day 1, in January, in year 2025-2030."
        );
    }

    #[test]
    fn test_day_descriptions() {
        assert_eq!(get_description("0 12 * * MON"), "At 12:00, on Monday.");
        assert_eq!(
            get_description("0 12 * * 1-5"),
            "At 12:00, on Monday, Tuesday, Wednesday, Thursday, and Friday."
        );
        assert_eq!(get_description("0 12 15 * *"), "At 12:00, on day 15.");
        assert_eq!(
            get_description("0 12 L * *"),
            "At 12:00, on the last day of the month."
        );
        assert_eq!(
            get_description("0 12 1,15 * *"),
            "At 12:00, on day 1 and 15."
        );
    }

    #[test]
    fn test_month_descriptions() {
        assert_eq!(get_description("* * * JAN *"), "Every minute, in January.");
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
        let or_desc = get_description("0 0 15 * FRI");
        assert_eq!(or_desc, "At 00:00, on day 15 or Friday.");

        // AND behavior
        let and_desc =
            get_description_lang_config("0 0 15 * FRI", English, Seconds::Optional, Year::Optional, true);
        assert_eq!(
            and_desc,
            "At 00:00, on day 15 (if it is also Friday)."
        );
    }

    #[test]
    fn test_complex_combinations() {
        assert_eq!(
            get_description("30 18 15,L MAR *"),
            "At 18:30, on day 15 and the last day of the month, in March."
        );
        
        let and_desc = get_description_lang_config("30 18 15,L MAR FRI", English, Seconds::Optional, Year::Optional, true);
        assert_eq!(
            and_desc,
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
         let desc = get_description_lang_config("0 0 1 * FRI#L,MON#1", English, Seconds::Optional, Year::Optional, true);
         assert_eq!(
            desc,
            "At 00:00, on day 1 (if it is also one of: the last Friday of the month and the 1st Monday of the month)."
        );
    }

    // Issue #35: Incorrect descriptor
    // https://github.com/Hexagon/croner-rust/issues/35
    // Pattern "* 0 * * *" was producing "At of hour 0." instead of "Every minute past hour 0."
    
    #[test]
    fn test_issue_35_wildcard_minutes_specific_hours() {
        // Original bug: "* 0 * * *" produced "At of hour 0."
        assert_eq!(
            get_description("* 0 * * *"),
            "Every minute past hour 0."
        );
        assert_eq!(
            get_description("* 5 * * *"),
            "Every minute past hour 5."
        );
        assert_eq!(
            get_description("* 0-5 * * *"),
            "Every minute past hour 0-5."
        );
    }

    #[test]
    fn test_issue_35_seconds_variant() {
        // Similar issue with seconds: "* * 0 * * *" was producing "Every second, of hour 0."
        assert_eq!(
            get_description("* * 0 * * *"),
            "Every second past hour 0."
        );
        assert_eq!(
            get_description("* * 5 * * *"),
            "Every second past hour 5."
        );
        assert_eq!(
            get_description("* * 0,12 * * *"),
            "Every second past hour 0 and 12."
        );
    }

    #[test]
    fn test_issue_35_with_other_fields() {
        // Test combinations with days, months, weekdays
        assert_eq!(
            get_description("* 0 * 1 *"),
            "Every minute past hour 0, in January."
        );
        assert_eq!(
            get_description("* 0 * * MON"),
            "Every minute past hour 0, on Monday."
        );
    }

    #[test]
    fn test_no_grammatical_errors() {
        // Ensure no grammatical errors like "At of", "At ,", etc.
        let patterns = vec![
            "* 0 * * *",
            "* * 0 * * *",
            "0 * 0 * * *",
            "* 0 * 1 *",
            "* 0 * * MON",
        ];

        for pattern in patterns {
            let desc = get_description(pattern);
            assert!(
                !desc.contains("At of") && !desc.contains("At ,") && !desc.starts_with("At ."),
                "Pattern '{}' produced grammatically incorrect description: '{}'",
                pattern,
                desc
            );
        }
    }
}