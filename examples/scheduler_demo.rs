use chrono::Utc;
use croner::scheduler::CronScheduler;
use croner::Cron;
use std::thread;

// Define the Params structure
#[derive(Clone)]
struct Params {
    // Define the fields you need
    test: &'static str,
}

fn main() {
    // Schedule one task at even seconds
    let cron_1: Cron = "0/2 * 23 * * *".parse().expect("Invalid cron expression");
    let mut scheduler_1 = CronScheduler::new(cron_1, Utc).with_threadpool_size(5);

    // Define the context for the first scheduler
    let context_1 = Params {
        test: "Hello Context!",
    };
    scheduler_1.with_context(context_1);
    scheduler_1.start(|opt_context: Option<&Params>| {
        if let Some(_context) = opt_context {
            println!("Context message:{}", _context.test);
        }
        println!("Task 1 started at {:?}, sleeping for 5 seconds", Utc::now());
        //thread::sleep(std::time::Duration::from_secs(5));
        println!("Task 1 done at {:?}", Utc::now());
    });

    // Schedule another task at odd seconds
    let cron_2: Cron = "1/2 * * * * *".parse().expect("Invalid cron expression");
    let mut scheduler_2 = CronScheduler::new(cron_2, Utc);

    // Define the context for the second scheduler
    let context_2 = Params { test: "Test" };
    scheduler_2.with_context(context_2);
    scheduler_2.start(|opt_context: Option<&Params>| {
        if let Some(_context) = opt_context {
            // Use the context here if needed
        }
        println!("Task 2 started at {:?}, sleeping for 5 seconds", Utc::now());
        thread::sleep(std::time::Duration::from_secs(5));
        println!("Task 2 done at {:?}", Utc::now());
    });

    // The tasks can be paused, resumed, or stopped as needed
    // scheduler_1.pause();
    // scheduler_1.resume();
    // scheduler_1.stop();
    // scheduler_2.pause();
    // scheduler_2.resume();
    // scheduler_2.stop();

    // Loop to keep the main process alive
    while scheduler_1.tick() | scheduler_2.tick() {
        // Sleep for a short duration to prevent busy waiting
        thread::sleep(std::time::Duration::from_millis(300));
    }
}
