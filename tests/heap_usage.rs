//! Counts the heap a parsed `Cron` keeps, which matters on small `no_std` targets.
//!
//! This file is its own test binary with a counting global allocator, and holds a
//! single test so that no other test allocates while it measures.
#![cfg(feature = "chrono")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{TimeZone as _, Utc};
use croner::parser::{CronParser, Seconds, Year};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let live = LIVE.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
        PEAK.fetch_max(live, Ordering::SeqCst);
        COUNT.fetch_add(1, Ordering::SeqCst);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Heap bytes `f`'s result keeps, the peak while it ran, and the number of allocations.
fn measure<T>(f: impl FnOnce() -> T) -> (T, usize, usize, usize) {
    let before = LIVE.load(Ordering::SeqCst);
    PEAK.store(before, Ordering::SeqCst);
    let count = COUNT.load(Ordering::SeqCst);
    let value = f();
    let kept = LIVE.load(Ordering::SeqCst) - before;
    let peak = PEAK.load(Ordering::SeqCst) - before;
    (value, kept, peak, COUNT.load(Ordering::SeqCst) - count)
}

#[test]
fn parsed_cron_heap_usage() {
    let parser = CronParser::new();
    let with_years = CronParser::builder()
        .seconds(Seconds::Optional)
        .year(Year::Optional)
        .build();

    // Patterns without a year field, or with `*`, store no per-year flags.
    for pattern in ["0 0 * * *", "0 0 * * MON-FRI", "*/5 * * * * *"] {
        let (cron, kept, peak, _) = measure(|| with_years.parse(pattern).unwrap());
        assert!(kept < 1024, "{pattern} keeps {kept} B");
        assert!(peak < 2048, "{pattern} peaks at {peak} B");
        drop(cron);
    }
    for pattern in ["0 0 0 * * * *", "0 0 0 * * * */1"] {
        let (cron, kept, _, _) = measure(|| with_years.parse(pattern).unwrap());
        assert!(kept < 1024, "{pattern} keeps {kept} B");
        drop(cron);
    }

    // A year list still stores one byte per year up to `YEAR_UPPER_LIMIT`.
    let (cron, kept, _, _) = measure(|| with_years.parse("0 0 0 1 1 * 2030").unwrap());
    assert!(kept > croner::YEAR_UPPER_LIMIT as usize, "kept {kept} B");
    drop(cron);

    // Searching and matching allocate nothing.
    let cron = parser.parse("0 0 * * MON-FRI").unwrap();
    let start = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let (next, _, peak, count) = measure(|| cron.find_next_occurrence(&start, false).unwrap());
    assert_eq!((peak, count), (0, 0));
    let (_, _, peak, count) = measure(|| cron.is_time_matching(&next).unwrap());
    assert_eq!((peak, count), (0, 0));
    let (_, _, peak, count) = measure(|| cron.find_previous_occurrence(&start, false).unwrap());
    assert_eq!((peak, count), (0, 0));
}
