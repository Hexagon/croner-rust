use chrono::Local;

use croner::pattern::CronPattern;
use croner::scheduler::CronScheduler;

fn main() {
    let pattern_all = "* * * * * *";
    let pattern_full_second = "0 18 * * * *";
    let pattern_29th_feb = "0 18 0 29 2 *";
    let pattern_29th_feb_mon = "0 18 0 29 2 1";

    let cron_pattern_all = CronPattern::new(pattern_all).unwrap();
    let cron_pattern_full_second = CronPattern::new(pattern_full_second).unwrap();
    let cron_pattern_29th_feb = CronPattern::new(pattern_29th_feb).unwrap();
    let cron_pattern_29th_feb_mon = CronPattern::new(pattern_29th_feb_mon).unwrap();

    let time = Local::now();

    let matches_all = CronScheduler::is_time_matching(&cron_pattern_all, &time).unwrap();
    let matches_full_second =
        CronScheduler::is_time_matching(&cron_pattern_full_second, &time).unwrap();

    let next_match_29th_feb = CronScheduler::find_next_occurrence(&cron_pattern_29th_feb, &time).unwrap();

    let next_match_29th_feb_mon = CronScheduler::find_next_occurrence(&cron_pattern_29th_feb_mon, &time).unwrap();

    println!("Time is: {}", time);
    println!(
        "Pattern \"{}\" does {}",
        pattern_all,
        if matches_all { "match" } else { "not match" }
    );
    println!(
        "Pattern \"{}\" does {}",
        pattern_full_second,
        if matches_full_second {
            "match"
        } else {
            "not match"
        }
    );
    println!(
        "Pattern \"{}\" does {}",
        pattern_full_second,
        if matches_full_second {
            "match"
        } else {
            "not match"
        }
    );
    println!(
        "Pattern \"{}\" does occur the next time at {}",
        pattern_29th_feb,
        next_match_29th_feb
    );
    println!(
        "Pattern \"{}\" does occur the next time at {}",
        pattern_29th_feb_mon,
        next_match_29th_feb_mon
    );
}
