use chrono::Local;
use croner::Cron;

fn main() {
    let time = Local::now();

    let cron_all: Cron = "* * * * * *".parse().unwrap();
    let cron_second: Cron = "0 18 * * * *".parse().unwrap();
    let cron_29th_feb: Cron = "0 18 0 29 2 *".parse().unwrap();
    let cron_29th_feb_mon: Cron = "0 18 0 29 2 MON".parse().unwrap();

    println!("Time is: {}", time);
    println!(
        "Pattern \"{}\" does {}",
        cron_all.pattern.to_string(),
        if cron_all.is_time_matching(&time).unwrap() {
            "match"
        } else {
            "not match"
        }
    );
    println!(
        "Pattern \"{}\" does {}",
        cron_second.pattern.to_string(),
        if cron_second.is_time_matching(&time).unwrap() {
            "match"
        } else {
            "not match"
        }
    );
    println!(
        "Pattern \"{}\" does {}",
        cron_29th_feb.pattern.to_string(),
        if cron_29th_feb.is_time_matching(&time).unwrap() {
            "match"
        } else {
            "not match"
        }
    );
    println!(
        "Pattern \"{}\" does occur the next time at {}",
        cron_29th_feb.pattern.to_string(),
        cron_29th_feb.find_next_occurrence(&time).unwrap()
    );
    println!(
        "Pattern \"{}\" does occur the next time at {}",
        cron_29th_feb_mon.pattern.to_string(),
        cron_29th_feb_mon.find_next_occurrence(&time).unwrap()
    );
}
