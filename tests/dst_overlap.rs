//! Searching across a duplicated hour, in time zones that shape it differently.
//!
//! When a time zone puts its clock back, a range of wall clock times happens
//! twice. Croner searches on the wall clock, which passes that range once, so
//! the search has to cover the second pass on its own. These tests check that
//! it does, and that it never doubles back.

#![cfg(any(feature = "chrono", feature = "jiff"))]

use croner::Cron;

fn every_minute() -> Cron {
    "* * * * *".parse().expect("the pattern must parse")
}

// ---------------------------------------------------------------------------
// Chrono backend
// ---------------------------------------------------------------------------

#[cfg(feature = "chrono")]
mod chrono_tests {
    use chrono::{DateTime, LocalResult, TimeZone};
    use chrono_tz::Australia::Lord_Howe;
    use chrono_tz::Europe::Paris;
    use chrono_tz::Pacific::Chatham;
    use chrono_tz::Tz;
    use croner::{Cron, Direction};

    use super::every_minute;

    struct Overlap {
        name: &'static str,
        zone: Tz,
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
    }

    impl Overlap {
        fn at(&self, pass: Pass) -> DateTime<Tz> {
            let repeated = self
                .zone
                .with_ymd_and_hms(self.year, self.month, self.day, self.hour, self.minute, 0);
            let LocalResult::Ambiguous(first, second) = repeated else {
                panic!("{}: the test time must happen twice", self.name)
            };
            match pass {
                Pass::First => first,
                Pass::Second => second,
            }
        }
    }

    enum Pass {
        First,
        Second,
    }

    fn overlaps() -> Vec<Overlap> {
        vec![
            Overlap {
                name: "Europe/Paris",
                zone: Paris,
                year: 2024,
                month: 10,
                day: 27,
                hour: 2,
                minute: 30,
            },
            Overlap {
                name: "Australia/Lord_Howe",
                zone: Lord_Howe,
                year: 2024,
                month: 4,
                day: 7,
                hour: 1,
                minute: 40,
            },
            Overlap {
                name: "Pacific/Chatham",
                zone: Chatham,
                year: 2024,
                month: 4,
                day: 7,
                hour: 3,
                minute: 20,
            },
        ]
    }

    #[track_caller]
    fn assert_moves_one_way(times: &[DateTime<Tz>], direction: Direction, zone: &str) {
        for step in times.windows(2) {
            let moved = match direction {
                Direction::Forward => step[0] < step[1],
                Direction::Backward => step[0] > step[1],
            };
            assert!(
                moved,
                "{zone}: the run went the wrong way, from {} to {}",
                step[0], step[1]
            );
        }
    }

    #[test]
    fn chrono_search_never_answers_with_a_time_it_has_passed() {
        for overlap in overlaps() {
            let cron = every_minute();
            let second = overlap.at(Pass::Second);
            let next = cron
                .find_next_occurrence(&second, false)
                .expect("the search must succeed");
            assert!(next > second, "{}: forward search from {second} answered {next}", overlap.name);

            let first = overlap.at(Pass::First);
            let previous = cron
                .find_previous_occurrence(&first, false)
                .expect("the search must succeed");
            assert!(previous < first, "{}: backward search from {first} answered {previous}", overlap.name);
        }
    }

    #[test]
    fn chrono_iterators_run_one_way_and_cover_every_instant_once() {
        for overlap in overlaps() {
            let cron = every_minute();
            let forward: Vec<_> = cron.iter_after(overlap.at(Pass::First)).take(200).collect();
            assert_moves_one_way(&forward, Direction::Forward, overlap.name);

            let last = *forward.last().expect("the run must not be empty");
            let mut backward: Vec<_> = cron.iter_before(last).take(199).collect();
            assert_moves_one_way(&backward, Direction::Backward, overlap.name);
            backward.reverse();
            assert_eq!(backward, forward[..199], "{}: the two directions disagree", overlap.name);
        }
    }

    #[test]
    fn chrono_a_duplicated_minute_comes_round_exactly_twice() {
        for overlap in overlaps() {
            let cron = every_minute();
            let first = overlap.at(Pass::First);
            let wanted = first.naive_local();

            let hits = cron
                .iter_from(first, Direction::Forward)
                .take(201)
                .filter(|time| time.naive_local() == wanted)
                .count();
            assert_eq!(hits, 2, "{}: {wanted} came round {hits} times, not twice", overlap.name);
        }
    }

    #[test]
    fn chrono_a_match_in_a_later_overlap_takes_its_own_real_order() {
        let cron: Cron = "*/30 2 26-27 10 *".parse().expect("the pattern must parse");
        let start = Paris
            .with_ymd_and_hms(2024, 10, 27, 2, 45, 0)
            .latest()
            .expect("the test time must exist");
        let next = cron.find_next_occurrence(&start, false).expect("search must succeed");
        assert_eq!(
            next,
            Paris.with_ymd_and_hms(2025, 10, 26, 2, 0, 0).earliest().expect("test time must exist")
        );
    }
}

// ---------------------------------------------------------------------------
// Jiff backend
// ---------------------------------------------------------------------------

#[cfg(feature = "jiff")]
mod jiff_tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;
    use jiff::Zoned;
    use croner::{Cron, Direction};

    use super::every_minute;

    struct Overlap {
        name: &'static str,
        zone: TimeZone,
        year: i16,
        month: i8,
        day: i8,
        hour: i8,
        minute: i8,
    }

    impl Overlap {
        fn at(&self, pass: Pass) -> Zoned {
            let ambiguous = self
                .zone
                .to_ambiguous_zoned(date(self.year, self.month, self.day).at(self.hour, self.minute, 0, 0));
            match pass {
                Pass::First => ambiguous.earlier().expect("the time must be ambiguous"),
                Pass::Second => ambiguous.later().expect("the time must be ambiguous"),
            }
        }
    }

    enum Pass {
        First,
        Second,
    }

    fn overlaps() -> Vec<Overlap> {
        vec![
            Overlap {
                name: "Europe/Paris",
                zone: TimeZone::get("Europe/Paris").expect("known zone"),
                year: 2024,
                month: 10,
                day: 27,
                hour: 2,
                minute: 30,
            },
            Overlap {
                name: "Australia/Lord_Howe",
                zone: TimeZone::get("Australia/Lord_Howe").expect("known zone"),
                year: 2024,
                month: 4,
                day: 7,
                hour: 1,
                minute: 40,
            },
            Overlap {
                name: "Pacific/Chatham",
                zone: TimeZone::get("Pacific/Chatham").expect("known zone"),
                year: 2024,
                month: 4,
                day: 7,
                hour: 3,
                minute: 20,
            },
        ]
    }

    #[track_caller]
    fn assert_moves_one_way(times: &[Zoned], direction: Direction, zone: &str) {
        for step in times.windows(2) {
            let moved = match direction {
                Direction::Forward => step[0] < step[1],
                Direction::Backward => step[0] > step[1],
            };
            assert!(
                moved,
                "{zone}: the run went the wrong way, from {} to {}",
                step[0], step[1]
            );
        }
    }

    #[test]
    fn jiff_search_never_answers_with_a_time_it_has_passed() {
        for overlap in overlaps() {
            let cron = every_minute();
            let second = overlap.at(Pass::Second);
            let next = cron
                .find_next_occurrence(&second, false)
                .expect("the search must succeed");
            assert!(next > second, "{}: forward search from {second} answered {next}", overlap.name);

            let first = overlap.at(Pass::First);
            let previous = cron
                .find_previous_occurrence(&first, false)
                .expect("the search must succeed");
            assert!(previous < first, "{}: backward search from {first} answered {previous}", overlap.name);
        }
    }

    #[test]
    fn jiff_iterators_run_one_way_and_cover_every_instant_once() {
        for overlap in overlaps() {
            let cron = every_minute();
            let forward: Vec<_> = cron.iter_after(overlap.at(Pass::First)).take(200).collect();
            assert_moves_one_way(&forward, Direction::Forward, overlap.name);

            let last = forward.last().expect("the run must not be empty").clone();
            let mut backward: Vec<_> = cron.iter_before(last).take(199).collect();
            assert_moves_one_way(&backward, Direction::Backward, overlap.name);
            backward.reverse();
            assert_eq!(backward, forward[..199], "{}: the two directions disagree", overlap.name);
        }
    }

    #[test]
    fn jiff_a_duplicated_minute_comes_round_exactly_twice() {
        for overlap in overlaps() {
            let cron = every_minute();
            let first = overlap.at(Pass::First);
            let wanted = first.datetime();

            let hits = cron
                .iter_from(first, Direction::Forward)
                .take(201)
                .filter(|time| time.datetime() == wanted)
                .count();
            assert_eq!(hits, 2, "{}: {wanted} came round {hits} times, not twice", overlap.name);
        }
    }

    #[test]
    fn jiff_a_match_in_a_later_overlap_takes_its_own_real_order() {
        let paris = TimeZone::get("Europe/Paris").expect("known zone");
        let cron: Cron = "*/30 2 26-27 10 *".parse().expect("the pattern must parse");

        let start = paris
            .to_ambiguous_zoned(date(2024, 10, 27).at(2, 45, 0, 0))
            .later()
            .expect("the time must be ambiguous");
        let next = cron.find_next_occurrence(&start, false).expect("search must succeed");

        let expected = paris
            .to_ambiguous_zoned(date(2025, 10, 26).at(2, 0, 0, 0))
            .earlier()
            .expect("the time must be ambiguous");
        assert_eq!(next, expected);
    }
}