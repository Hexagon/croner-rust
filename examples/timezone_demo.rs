use chrono::Local;
use chrono_tz::Tz;
use croner::Cron;

fn main() {
    // Parse cron expression
    let cron = Cron::parse("0 18 * * * 5").expect("Couldn't parse cron string");

    // Choose a different time zone, for example Eastern Standard Time (EST)
    let est_timezone: Tz = "America/New_York".parse().expect("Invalid timezone");

    // Find the next occurrence in EST
    let time_est = Local::now().with_timezone(&est_timezone);
    let next_est = cron.find_next_occurrence(&time_est, false).unwrap();

    // Output results for EST
    println!("EST time is: {}", time_est);
    println!(
        "Pattern \"{}\" will match next time at (EST): {}",
        cron.pattern.to_string(),
        next_est
    );
}
