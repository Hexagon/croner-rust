use std::str::FromStr as _;

use chrono::Utc;
use chrono_tz::Tz;
use croner::Cron;

fn main() {
    // Parse cron expression
    let cron = Cron::from_str("18 * * * 5").expect("Couldn't parse cron string");

    // Find the next occurrence in Europe/Stockholm
    let now_stockholm = Utc::now().with_timezone(&Tz::Europe__Stockholm);
    let next_stockholm = cron.find_next_occurrence(&now_stockholm, false).unwrap();

    // Output results for Europe/Stockholm
    println!("UTC time is: {}", &Utc::now());
    println!("Time in Europe/Stockholm time is: {}", &now_stockholm);
    println!(
        "Pattern \"{}\" will match next time at (Europe/Stockholm): {}",
        cron.pattern, next_stockholm
    );
}
