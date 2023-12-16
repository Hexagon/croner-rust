use chrono::Utc;
use croner::Cron;

fn main() {
    // Parse cron expression
    let cron = Cron::new("* * * * * *").with_seconds_optional().parse().expect("Couldn't parse cron string");

    // Compare to UTC time now
    let time = Utc::now();

    // (Or Local)
    // let time = Local::now();

    // Get next 5 matches using iter_from
    // There is also iter_after, which does not match starting time
    println!(
        "Finding matches of pattern '{}' starting from {}:",
        cron.pattern.to_string(),
        time
    );

    for time in cron.iter_from(time).take(5) {
        println!("{}", time);
    }
}
