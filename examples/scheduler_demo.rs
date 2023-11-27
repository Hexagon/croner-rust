use chrono::Local;
use croner::scheduler::{CronScheduler, SchedulerResult};
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
    let cron_1: Cron = "0/2 * 21 * * *".parse().expect("Invalid cron expression");
    let mut scheduler_1 = CronScheduler::new(cron_1).with_threadpool_size(5);

    // Define the context for the first scheduler
    let context_1 = Params {
        test: "Hello Context!",
    };
    scheduler_1.with_context(context_1);
    scheduler_1.start(|opt_context: Option<&Params>| {
        if let Some(_context) = opt_context {
            println!("Context message:{}", _context.test);
        }
        println!(
            "Task 1 started at {:?}, sleeping for 5 seconds",
            Local::now()
        );
        //thread::sleep(std::time::Duration::from_secs(5));
        println!("Task 1 done at {:?}", Local::now());
    });

    // Schedule another task at odd seconds
    let cron_2: Cron = "1/2 * * * * *".parse().expect("Invalid cron expression");
    let mut scheduler_2 = CronScheduler::new(cron_2);
    scheduler_2 = scheduler_2.with_threadpool_size(5);

    // Define the context for the second scheduler
    let context_2 = Params { test: "Test" };
    scheduler_2.with_context(context_2);
    scheduler_2.start(|opt_context: Option<&Params>| {
        if let Some(_context) = opt_context {
            // Use the context here if needed
        }
        println!(
            "Task 2 started at {:?}, sleeping for 5 seconds",
            Local::now()
        );
        thread::sleep(std::time::Duration::from_secs(5));
        println!("Task 2 done at {:?}", Local::now());
    });

    // The tasks can be paused, resumed, or stopped as needed
    // scheduler_1.pause();
    // scheduler_1.resume();
    // scheduler_1.stop();
    // scheduler_2.pause();
    // scheduler_2.resume();
    // scheduler_2.stop();

    // Loop to keep the main process alive
    // - You need to supply a time zoned "now" to tick, so that
    //   croner knows which timezone to match the pattern against.
    //   Using Local in this example.
    loop {
        // Exit when both schedulers are dead
        let res1 = scheduler_1.tick(Local::now());
        let res2 = scheduler_2.tick(Local::now());
        if res1 == SchedulerResult::Dead && res2 == SchedulerResult::Dead {
            break;
        }

        // Warn on pool exhaustion
        if res1 == SchedulerResult::ThreadPoolExhausted {
            println!("Scheduler 1 exhausted");
        }
        if res2 == SchedulerResult::ThreadPoolExhausted {
            println!("Scheduler 2 exhausted");
        }

        // Sleep for a short duration to prevent busy waiting
        thread::sleep(std::time::Duration::from_millis(300));
    }
}
