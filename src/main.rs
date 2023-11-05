mod pattern;

fn main() {
    
    let pattern = "5-10 * * * * *"; // Run every 5 seconds

    let cron_pattern = pattern::CronPattern::new(pattern).unwrap();

    println!("{:?}", cron_pattern);

}