use chrono::Local;
use croner::pattern::CronPattern;
use croner::scheduler::CronScheduler;

fn main() {

    let pattern_all = "* * * * * *";
    let pattern_full_second = "0 18 * * * *";
    
    let cron_pattern_all = CronPattern::new(pattern_all).unwrap();
    let cron_pattern_full_second = CronPattern::new(pattern_full_second).unwrap();

    let time = Local::now();

    let matches_all = CronScheduler::is_time_matching(&cron_pattern_all, &time).unwrap();
    let matches_full_second = CronScheduler::is_time_matching(&cron_pattern_full_second, &time).unwrap();

    println!("Time is: {}", time);
    println!("Pattern \"{}\" does {}", pattern_all, if matches_all { "match" } else { "not match" });
    println!("Pattern \"{}\" does {}", pattern_full_second, if matches_full_second { "match" } else { "not match" });

}
