use chrono::Local;
use croner::Cron;

fn main() {
    // Example: Parse cron expression
    let cron = Cron::new("0 18 * * * FRI")
        .with_seconds_required()
        .parse()
        .expect("Couldn't parse cron string");

    // Example: Compare cron pattern with current local time
    let time = Local::now();
    let matches = cron.is_time_matching(&time).unwrap();

    // Example: Get next match
    let next = cron.find_next_occurrence(&time, false).unwrap();

    // Example: Output results
    println!("Current time is: {}", time);
    println!(
        "Pattern \"{}\" does {} time {}",
        cron.pattern.to_string(),
        if matches { "match" } else { "not match" },
        time
    );
    println!(
        "Pattern \"{}\" will match next time at {}",
        cron.pattern.to_string(),
        next
    );

    // Example: Iterator
    println!("Next 5 matches:");
    for time in cron.clone().iter_from(Local::now()).take(5) {
        println!("{}", time);
    }
}
