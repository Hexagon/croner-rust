use chrono::Local;
use croner::Cron;

fn main() {
    // Parse cron expression
    let cron: Cron = "* * * * * *".parse().expect("Couldn't parse cron string");

    // Compare to time now
    let time = Local::now();

    // Get next 5 matches using iter_from
    // There is also iter_after, which does not match starting time
    println!("Finding matches of pattern '{}' starting from {}:", cron.pattern.to_string(), time);

    for time in cron.clone().iter_from(time).take(5) {
        println!("{}", time);
    }
}
