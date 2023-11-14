use threadpool::ThreadPool;

use chrono::{DateTime, TimeZone, Timelike, Utc};

use crate::Cron;
use std::sync::{Arc, Mutex};

pub struct CronScheduler<Tz, C, F>
where
    Tz: TimeZone + Send + Sync + 'static,
    Tz::Offset: std::fmt::Display,
    C: Clone + Send + Sync + 'static,
    F: FnMut(Option<&C>) + Send + 'static,
{
    cron: Cron,
    timezone: Tz,
    task: ScheduledTask,
    context: Option<Arc<Mutex<ScheduledTaskContext<C>>>>,
    thread_pool: ThreadPool,
    callback: Option<Arc<Mutex<F>>>,
}

#[derive(PartialEq)]
enum SchedulerState {
    Running,
    Paused,
    Stopped,
}

#[derive(PartialEq)]
enum TaskState {
    Busy,
    Idle,
}

struct ScheduledTask {
    scheduler_state: SchedulerState,
    last_start: Option<DateTime<Utc>>,
    max_executions: Option<usize>,
    _executions: usize,
    shared_state: Arc<Mutex<SharedTaskState>>,
}

struct SharedTaskState {
    last_finish: Option<DateTime<Utc>>,
    task_state: TaskState,
}

pub struct ScheduledTaskContext<C>
where
    C: Send + Sync + 'static,
{
    context: Option<C>,
}

impl<Tz, C, F> CronScheduler<Tz, C, F>
where
    Tz: TimeZone + Send + Sync + 'static,
    Tz::Offset: std::fmt::Display,
    C: Clone + Send + Sync + 'static, // Added Clone here
    F: FnMut(Option<&C>) + Send + 'static,
{
    pub fn new(cron: Cron, timezone: Tz) -> Self {
        CronScheduler {
            cron,
            timezone,
            task: ScheduledTask {
                scheduler_state: SchedulerState::Stopped,
                last_start: None,
                max_executions: None,
                _executions: 0,
                shared_state: Arc::new(Mutex::new(SharedTaskState {
                    task_state: TaskState::Idle,
                    last_finish: None,
                })),
            },
            context: None,
            thread_pool: ThreadPool::new(1),
            callback: None,
        }
    }

    // Will return false if there is no further work to be done
    pub fn tick(&mut self) -> bool {
        // Check if the scheduler is stopped
        if self.task.scheduler_state == SchedulerState::Stopped {
            return false;
        }

        // Check if the scheduler is busy
        {
            // Temporarily unlock the mutex to set task state to busy
            let state = self.task.shared_state.lock().unwrap();
            if TaskState::Busy == state.task_state {
                // Skip this run
                return true;
            }
        }

        let now: DateTime<Utc> = Utc::now();

        // Check if we are past the expected run time
        if let Some(last_run) = self.task.last_start {
            if let Some(next_run_time) = self.next_run_after(last_run) {
                if let Some(next_run_time_no_nano) = next_run_time.with_nanosecond(0) {
                    if now <= next_run_time_no_nano {
                        return true; // Not time yet
                    }
                }
            }
        }

        // Check if it is time to run
        if let Some(next_run_time) = self.next_run_from(None) {
            if now < next_run_time {
                return true; // Not time to trigger yet
            }
        } else {
            return false; // If there's no next run time, don't proceed
        }

        let thread_pool = self.thread_pool.clone();
        let shared_state_clone = self.task.shared_state.clone();
        let callback_clone = self.callback.as_ref().map(Arc::clone);
        let context_clone = self.context.clone(); // Clone the optional context

        self.task.last_start = Some(Utc::now());
        thread_pool.execute(move || {
            if let Some(callback) = callback_clone {
                // Temporarily unlock the mutex to set task state to busy
                {
                    let mut state = shared_state_clone.lock().unwrap();
                    state.task_state = TaskState::Busy;
                }
                let mut callback = callback.lock().unwrap();

                // Clone the context data if it exists, and pass it as an owned value
                let context_clone =
                    context_clone.and_then(|context| context.lock().unwrap().context.clone());

                // Pass the cloned context (now owned) to the callback
                callback(context_clone.as_ref());

                let mut state = shared_state_clone.lock().unwrap();
                state.last_finish = Some(Utc::now());
                state.task_state = TaskState::Idle;
            }
        });

        true
    }

    pub fn start(&mut self, callback: F) {
        self.task.scheduler_state = SchedulerState::Running;
        self.callback = Some(Arc::new(Mutex::new(callback))); // Wrap in Arc and Mutex
    }

    pub fn pause(&mut self) {
        self.task.scheduler_state = SchedulerState::Paused;
    }

    pub fn with_context(&mut self, context: C) {
        self.context = Some(Arc::new(Mutex::new(ScheduledTaskContext {
            context: Some(context),
        })));
    }

    pub fn resume(&mut self) {
        self.task.scheduler_state = SchedulerState::Running;
    }

    pub fn with_max_executions(mut self, max: usize) -> Self {
        self.task.max_executions = Some(max);
        self
    }

    pub fn with_threadpool_size(mut self, size: usize) -> Self {
        self.thread_pool = ThreadPool::new(size);
        self
    }

    pub fn next_run_from(&self, now: Option<DateTime<Utc>>) -> Option<DateTime<Tz>> {
        if let Some(int_now) = now {
            let tz_now = self.timezone.from_utc_datetime(&int_now.naive_utc());
            self.cron.find_next_occurrence(&tz_now, true).ok()
        } else {
            let now = self.timezone.from_utc_datetime(&Utc::now().naive_utc());
            self.cron.find_next_occurrence(&now, true).ok()
        }
    }

    pub fn next_run_after(&self, last_run: DateTime<Utc>) -> Option<DateTime<Tz>> {
        let last_run_utc = self.timezone.from_utc_datetime(&last_run.naive_utc());
        self.cron.find_next_occurrence(&last_run_utc, false).ok()
    }

    pub fn next_runs(&self, count: usize) -> Vec<DateTime<Tz>> {
        let mut runs = Vec::new();
        let current_time = self.timezone.from_utc_datetime(&Utc::now().naive_utc());
        for next_time in self.cron.iter_after(current_time).take(count) {
            runs.push(next_time.clone());
        }
        runs
    }

    // Returns previous runtime, or current run time if a task is running
    pub fn previous_or_current_run(&self) -> Option<DateTime<Tz>> {
        let shared_state_clone = self.task.shared_state.clone();
        let state = shared_state_clone.lock().unwrap();
        if let TaskState::Busy = state.task_state {
            self.task
                .last_start
                .map(|dt| dt.with_timezone(&self.timezone))
        } else {
            state.last_finish.map(|dt| dt.with_timezone(&self.timezone))
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.task.scheduler_state, SchedulerState::Running)
    }

    pub fn is_stopped(&self) -> bool {
        matches!(self.task.scheduler_state, SchedulerState::Stopped)
    }

    pub fn is_busy(&self) -> bool {
        let shared_state_clone = self.task.shared_state.clone();
        let state = shared_state_clone.lock().unwrap();
        matches!(state.task_state, TaskState::Busy)
    }
}
