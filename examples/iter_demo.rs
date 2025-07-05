use chrono::Utc;
use croner::parser::CronParser;

fn main() {
    // Parse cron expression
    let cron = CronParser::builder()
        .seconds(croner::parser::Seconds::Optional)
        .build()
        .parse("* * * * * *")
        .expect("Couldn't parse cron string");

    // Compare to UTC time now
    let time = Utc::now();

    // (Or Local)
    // let time = Local::now();

    // Get next 5 matches using iter_after
    // There is also iter_after, which does not match starting time
    println!(
        "Finding matches of pattern '{}' starting from {}:",
        cron.pattern, time
    );

    for time in cron.iter_after(time).take(5) {
        println!("{time}");
    }
}
