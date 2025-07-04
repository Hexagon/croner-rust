use chrono::Local;
use croner::parser::CronParser;

fn main() {
    // Example: Parse cron expression
    let cron = CronParser::builder()
        .seconds(croner::parser::Seconds::Required)
        .build()
        .parse("0 18 * * * FRI")
        .expect("Couldn't parse cron string");

    // Example: Compare cron pattern with current local time
    let time = Local::now();
    let matches = cron.is_time_matching(&time).unwrap();

    // Example: Get next match
    let next = cron.find_next_occurrence(&time, false).unwrap();

    // Example: Output results
    println!("Current time is: {time}");
    println!(
        "Pattern \"{}\" does {} time {}",
        cron.pattern,
        if matches { "match" } else { "not match" },
        time
    );
    println!(
        "Pattern \"{}\" will match next time at {}",
        cron.pattern, next
    );

    // Example: Iterator
    println!("Next 5 matches:");
    for time in cron.clone().iter_after(Local::now()).take(5) {
        println!("{time}");
    }

    // Example: Reverse Iterator
    println!("Previous 5 matches:");
    for time in cron.clone().iter_before(Local::now()).take(5) {
        println!("{time}");
    }
}
