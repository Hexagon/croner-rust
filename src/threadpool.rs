use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

type Task = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Arc<Mutex<VecDeque<Task>>>,
    cvar: Arc<Condvar>,
    shutdown: Arc<AtomicBool>,
    max_threads: usize,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let sender = Arc::new(Mutex::new(VecDeque::new()));
        let cvar = Arc::new(Condvar::new());
        let shutdown = Arc::new(AtomicBool::new(false));
        let mut workers = Vec::with_capacity(size);

        for _ in 0..size {
            workers.push(Worker::new(
                Arc::clone(&sender),
                Arc::clone(&cvar),
                Arc::clone(&shutdown),
            ));
        }

        ThreadPool {
            workers,
            sender,
            cvar,
            shutdown,
            max_threads: size,
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let task = Box::new(f);
        let mut tasks = self.sender.lock().unwrap();
        tasks.push_back(task);
        self.cvar.notify_one();
    }

    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.cvar.notify_all();

        // We need to use indices to borrow each worker mutably
        for i in 0..self.workers.len() {
            if let Some(thread) = self.workers[i].thread.take() {
                thread.join().unwrap();
            }
        }
    }

    pub fn max_count(&self) -> usize {
        self.max_threads
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(
        sender: Arc<Mutex<VecDeque<Task>>>,
        cvar: Arc<Condvar>,
        shutdown: Arc<AtomicBool>,
    ) -> Worker {
        let thread = thread::spawn(move || loop {
            let task = {
                let mut tasks = sender.lock().unwrap();
                while !shutdown.load(Ordering::SeqCst) && tasks.is_empty() {
                    tasks = cvar.wait(tasks).unwrap();
                }
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                tasks.pop_front()
            };

            if let Some(task) = task {
                task();
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}
