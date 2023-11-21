use threadpool::ThreadPool;

use chrono::{DateTime, TimeZone, Timelike, Utc};

use crate::Cron;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

pub struct CronScheduler<C, F>
where
    C: Clone + Send + Sync + 'static,
    F: FnMut(Option<&C>) + Send + 'static,
{
    cron: Cron,
    task: ScheduledTask,
    context: Option<Arc<Mutex<ScheduledTaskContext<C>>>>,
    thread_pool: ThreadPool,
    callback: Option<F>,
}

#[derive(PartialEq)]
enum SchedulerState {
    Running,
    Paused,
    Stopped,
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
    active_task_count: AtomicUsize,
}

pub struct ScheduledTaskContext<C>
where
    C: Send + Sync + 'static,
{
    context: Option<C>,
}

impl<C, F> CronScheduler<C, F>
where
    C: Clone + Send + Sync + 'static, // Added Clone here
    F: FnMut(Option<&C>) + Clone + Send + 'static,
{
    pub fn new(cron: Cron) -> Self {
        CronScheduler {
            cron,
            task: ScheduledTask {
                scheduler_state: SchedulerState::Stopped,
                last_start: None,
                max_executions: None,
                _executions: 0,
                shared_state: Arc::new(Mutex::new(SharedTaskState {
                    active_task_count: AtomicUsize::new(0),
                    last_finish: None,
                })),
            },
            context: None,
            thread_pool: ThreadPool::new(1),
            callback: None,
        }
    }

    // Will return false if there is no further work to be done
    pub fn tick<Tz: TimeZone>(&mut self, now: DateTime<Tz>) -> bool {
        // Check if the scheduler is stopped
        if self.task.scheduler_state == SchedulerState::Stopped {
            return false;
        }

        // Check if the scheduler is busy
        {
            if self.is_busy() && self.thread_pool.max_count() == 1 {
                // Skip this run
                return true;
            }
        }

        // Check if we are past the expected run time
        if let Some(last_run) = self.task.last_start {
            let last_run_tz = last_run.with_timezone(&now.timezone());
            if now.with_nanosecond(0) <= last_run_tz.with_nanosecond(0) {
                return true; // Already run in this second
            }
        }

        // Check if it is time to run
        if let Some(next_run_time) = self.next_run_from(now.clone()) {
            if now < next_run_time {
                return true; // Not time to trigger yet
            }
        } else {
            return false; // If there's no next run time, don't proceed
        }

        let thread_pool = self.thread_pool.clone();
        let shared_state_clone = self.task.shared_state.clone();
        let callback_clone = self.callback.clone();
        let context_clone = self.context.clone(); // Clone the optional context

        self.task.last_start = Some(Utc::now());

        thread_pool.execute(move || {
            if let Some(mut callback) = callback_clone {
                // Temporarily unlock the mutex to increment running tasks
                {
                    shared_state_clone
                        .lock()
                        .unwrap()
                        .active_task_count
                        .fetch_add(1, Ordering::SeqCst);
                }

                // Clone the context data if it exists, and pass it as an owned value
                let context_clone =
                    context_clone.and_then(|context| context.lock().unwrap().context.clone());

                // Pass the cloned context (now owned) to the callback
                callback(context_clone.as_ref());

                // Update task state
                let mut state = shared_state_clone.lock().unwrap();
                state.last_finish = Some(Utc::now());
                state.active_task_count.fetch_sub(1, Ordering::SeqCst);
            }
        });

        true
    }

    pub fn start(&mut self, callback: F) {
        self.task.scheduler_state = SchedulerState::Running;
        self.callback = Some(callback); // Wrap in Arc and Mutex
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

    pub fn next_run_from<Tz: TimeZone>(&self, from: DateTime<Tz>) -> Option<DateTime<Tz>> {
        self.cron.find_next_occurrence(&from, true).ok()
    }

    pub fn next_run_after<Tz: TimeZone>(&self, after: DateTime<Tz>) -> Option<DateTime<Tz>> {
        self.cron.find_next_occurrence(&after, false).ok()
    }

    pub fn next_runs<Tz: TimeZone>(&self, after: DateTime<Tz>, count: usize) -> Vec<DateTime<Tz>> {
        let mut runs = Vec::new();
        for next_time in self.cron.iter_after(after).take(count) {
            runs.push(next_time.clone());
        }
        runs
    }

    // Returns previous runtime, or current run time if a task is running
    pub fn previous_or_current_run<Tz: TimeZone>(&self, timezone: Tz) -> Option<DateTime<Tz>> {
        self.task.last_start.map(|dt| dt.with_timezone(&timezone))
    }

    pub fn is_running(&self) -> bool {
        matches!(self.task.scheduler_state, SchedulerState::Running)
    }

    pub fn is_stopped(&self) -> bool {
        matches!(self.task.scheduler_state, SchedulerState::Stopped)
    }

    pub fn is_busy(&self) -> bool {
        self.task
            .shared_state
            .lock()
            .unwrap()
            .active_task_count
            .load(Ordering::SeqCst)
            > 0
    }
}
