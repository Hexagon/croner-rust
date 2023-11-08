use chrono::Local;
use croner::Cron;

fn main() {
    // Parse cron expression
    let cron_all: Cron = "0 18 * * * 5".parse().expect("Couldn't parse cron string");

    // Compare to time now
    let time = Local::now();
    let matches_all = cron_all.is_time_matching(&time).unwrap();

    // Get next match
    let next = cron_all.find_next_occurrence(&time, false).unwrap();

    // Output results
    println!("Time is: {}", time);
    println!(
        "Pattern \"{}\" does {} time {}",
        cron_all.pattern.to_string(),
        if matches_all { "match" } else { "not match" },
        time
    );
    println!(
        "Pattern \"{}\" will match next time at {}",
        cron_all.pattern.to_string(),
        next
    );
}
