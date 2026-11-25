//! `FixedTimeRule::NoWildcard` classifies lists and ranges as fixed-time
//! jobs in DST handling, as croner for JavaScript does with
//! `fixedTimeRule: "no-wildcard"` (#91).

#![cfg(feature = "chrono")]

use chrono::{DateTime, TimeZone};
use chrono_tz::America::New_York;
use chrono_tz::Tz;
use croner::parser::{CronParser, FixedTimeRule, Seconds};
use croner::{Cron, JobType};

fn parse(pattern: &str, rule: FixedTimeRule) -> Cron {
    CronParser::builder()
        .seconds(Seconds::Optional)
        .fixed_time_rule(rule)
        .build()
        .parse(pattern)
        .expect("the pattern must parse")
}

fn runs(cron: &Cron, start: DateTime<Tz>, n: usize) -> Vec<String> {
    cron.iter_after(start)
        .take(n)
        .map(|t| t.format("%Y-%m-%d %H:%M:%S %Z").to_string())
        .collect()
}

#[test]
fn default_rule_is_single_value() {
    let cron: Cron = "0 1,13 * * *".parse().unwrap();
    assert_eq!(cron.pattern.fixed_time_rule(), FixedTimeRule::SingleValue);
    assert_eq!(cron.determine_job_type(), JobType::IntervalWildcard);
}

#[test]
fn job_types_under_no_wildcard() {
    let fixed = [
        "0 1,13 * * *",
        "0,30 1 * * *",
        "15 1,2 * * *",
        "0 0-23 * * *",
        "0 30 2 * * *",
        "0,30 0 2 * * *",
    ];
    for pattern in fixed {
        assert_eq!(
            parse(pattern, FixedTimeRule::NoWildcard).determine_job_type(),
            JobType::FixedTime,
            "{pattern}"
        );
    }
    let interval = [
        "* 1 * * *",
        "*/5 * * * *",
        "0 */2 * * *",
        "0 0-23/2 * * *",
        "0-10/2,15 1 * * *",
        "* 0 1 * * *",
        "*/10 0 1 * * *",
    ];
    for pattern in interval {
        assert_eq!(
            parse(pattern, FixedTimeRule::NoWildcard).determine_job_type(),
            JobType::IntervalWildcard,
            "{pattern}"
        );
    }
}

#[test]
fn single_value_rule_is_unchanged() {
    assert_eq!(
        parse("0 1 * * *", FixedTimeRule::SingleValue).determine_job_type(),
        JobType::FixedTime
    );
    assert_eq!(
        parse("0 1,13 * * *", FixedTimeRule::SingleValue).determine_job_type(),
        JobType::IntervalWildcard
    );
}

// Clocks in America/New_York go back from 02:00 EDT to 01:00 EST on 2025-11-02.
fn before_fall_back() -> DateTime<Tz> {
    New_York.with_ymd_and_hms(2025, 11, 2, 0, 0, 0).unwrap()
}

#[test]
fn issue_91_example_runs_once_in_repeated_hour() {
    let cron = parse("0 1,13 * * *", FixedTimeRule::NoWildcard);
    assert_eq!(
        runs(&cron, before_fall_back(), 3),
        [
            "2025-11-02 01:00:00 EDT",
            "2025-11-02 13:00:00 EST",
            "2025-11-03 01:00:00 EST",
        ]
    );

    // The default rule keeps running on both passes.
    let cron = parse("0 1,13 * * *", FixedTimeRule::SingleValue);
    assert_eq!(
        runs(&cron, before_fall_back(), 3),
        [
            "2025-11-02 01:00:00 EDT",
            "2025-11-02 01:00:00 EST",
            "2025-11-02 13:00:00 EST",
        ]
    );
}

#[test]
fn minute_list_runs_once_in_repeated_hour() {
    let cron = parse("0,30 1 * * *", FixedTimeRule::NoWildcard);
    assert_eq!(
        runs(&cron, before_fall_back(), 3),
        [
            "2025-11-02 01:00:00 EDT",
            "2025-11-02 01:30:00 EDT",
            "2025-11-03 01:00:00 EST",
        ]
    );
}

#[test]
fn hour_list_across_repeated_hour() {
    let cron = parse("15 1,2 * * *", FixedTimeRule::NoWildcard);
    assert_eq!(
        runs(&cron, before_fall_back(), 3),
        [
            "2025-11-02 01:15:00 EDT",
            "2025-11-02 02:15:00 EST",
            "2025-11-03 01:15:00 EST",
        ]
    );
}

#[test]
fn backward_search_skips_second_pass() {
    let cron = parse("0,30 1 * * *", FixedTimeRule::NoWildcard);
    let start = New_York.with_ymd_and_hms(2025, 11, 2, 3, 0, 0).unwrap();
    let back: Vec<String> = cron
        .iter_before(start)
        .take(3)
        .map(|t| t.format("%Y-%m-%d %H:%M:%S %Z").to_string())
        .collect();
    assert_eq!(
        back,
        [
            "2025-11-02 01:30:00 EDT",
            "2025-11-02 01:00:00 EDT",
            "2025-11-01 01:30:00 EDT",
        ]
    );
}

// Clocks in America/New_York jump from 02:00 EST to 03:00 EDT on 2025-03-09.
#[test]
fn times_in_gap_run_once_at_its_end() {
    let start = New_York.with_ymd_and_hms(2025, 3, 9, 0, 0, 0).unwrap();

    let cron = parse("0,30 2 * * *", FixedTimeRule::NoWildcard);
    assert_eq!(
        runs(&cron, start, 2),
        ["2025-03-09 03:00:00 EDT", "2025-03-10 02:00:00 EDT"]
    );

    let cron = parse("30 1,2 * * *", FixedTimeRule::NoWildcard);
    assert_eq!(
        runs(&cron, start, 3),
        [
            "2025-03-09 01:30:00 EST",
            "2025-03-09 03:00:00 EDT",
            "2025-03-10 01:30:00 EDT",
        ]
    );

    // Under the default rule the list is an interval job and skips the gap.
    let cron = parse("0,30 2 * * *", FixedTimeRule::SingleValue);
    assert_eq!(
        runs(&cron, start, 2),
        ["2025-03-10 02:00:00 EDT", "2025-03-10 02:30:00 EDT"]
    );
}

#[test]
fn rule_is_part_of_equality() {
    let a = parse("0 1,13 * * *", FixedTimeRule::SingleValue);
    let b = parse("0 1,13 * * *", FixedTimeRule::NoWildcard);
    assert_ne!(a, b);
    assert_eq!(b, parse("0 1,13 * * *", FixedTimeRule::NoWildcard));
}

// `FixedTimeRule::All` keeps the croner 2.x behaviour (#70).

#[test]
fn all_rule_makes_every_job_fixed_time() {
    for pattern in ["* * * * *", "*/30 * * * *", "0 */2 * * *", "*/10 * * * * *"] {
        assert_eq!(
            parse(pattern, FixedTimeRule::All).determine_job_type(),
            JobType::FixedTime,
            "{pattern}"
        );
    }
}

#[test]
fn all_rule_runs_repeated_hour_once() {
    // Clocks in America/New_York go back from 02:00 EDT to 01:00 EST on 2026-11-01.
    let cron = parse("*/30 * * * *", FixedTimeRule::All);
    let start = New_York.with_ymd_and_hms(2026, 11, 1, 0, 50, 0).unwrap();
    assert_eq!(
        runs(&cron, start, 5),
        [
            "2026-11-01 01:00:00 EDT",
            "2026-11-01 01:30:00 EDT",
            "2026-11-01 02:00:00 EST",
            "2026-11-01 02:30:00 EST",
            "2026-11-01 03:00:00 EST",
        ]
    );
}

#[test]
fn all_rule_never_returns_a_past_time_from_second_pass() {
    // 01:05 EST is on the second pass; croner 2.x returned 01:30 EDT here,
    // which lies before the start.
    let cron = parse("*/30 * * * *", FixedTimeRule::All);
    let start = New_York
        .with_ymd_and_hms(2026, 11, 1, 1, 5, 0)
        .latest()
        .unwrap();
    assert_eq!(
        runs(&cron, start, 2),
        ["2026-11-01 02:00:00 EST", "2026-11-01 02:30:00 EST"]
    );
}

#[test]
fn all_rule_runs_once_after_gap() {
    // Clocks in America/New_York jump from 02:00 EST to 03:00 EDT on 2026-03-08.
    let cron = parse("*/30 2 * * *", FixedTimeRule::All);
    let start = New_York.with_ymd_and_hms(2026, 3, 7, 12, 0, 0).unwrap();
    assert_eq!(
        runs(&cron, start, 3),
        [
            "2026-03-08 03:00:00 EDT",
            "2026-03-09 02:00:00 EDT",
            "2026-03-09 02:30:00 EDT",
        ]
    );

    let cron = parse("*/30 * * * *", FixedTimeRule::All);
    let start = New_York.with_ymd_and_hms(2026, 3, 8, 1, 10, 0).unwrap();
    assert_eq!(
        runs(&cron, start, 3),
        [
            "2026-03-08 01:30:00 EST",
            "2026-03-08 03:00:00 EDT",
            "2026-03-08 03:30:00 EDT",
        ]
    );
}
