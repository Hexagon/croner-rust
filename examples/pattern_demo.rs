use croner::pattern::CronPattern;

fn main() {
    let pattern = "5-10 5,10 5-10/2 1,L 5-6 1,7"; // Run every 5 seconds

    let cron_pattern = CronPattern::new(pattern).unwrap();

    println!(
        "\nCron pattern: \t {}\n\nResult: {:#?}",
        pattern, cron_pattern
    );
}
