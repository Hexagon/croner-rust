# Changelog

## [Unreleased]

### Changed
- Parsing a pattern without letters is about 2x faster, because the weekday and month name replacement is skipped for it.

### Fixed
- A day-of-week range of `SUN-SUN` now matches Sundays only. `-SUN` is read as 7 so that ranges like `SAT-SUN` work, which turned `SUN-SUN` into `0-7` and matched every day (e.g. `0 0 * * SUN-SUN` ran daily). Ports Hexagon/croner#469 ([#87](https://github.com/Hexagon/croner-rust/issues/87), [#88](https://github.com/Hexagon/croner-rust/pull/88)).
- With `alternative_weekdays` (Quartz-style, `SUN` = 1), day-of-week ranges ending in `SUN` such as `SAT-SUN` and `MON-SUN` failed with "Range out of bounds.", because they became `7-1` and `2-1`. As in Quartz, a day-of-week range whose end is before its start now wraps around the week in this mode, so `SAT-SUN` is Saturday and Sunday and `FRI-MON` is Friday to Monday. Numeric and stepped ranges wrap too (`7-1`, `FRI-TUE/2`). Standard mode and other fields are unchanged ([#89](https://github.com/Hexagon/croner-rust/pull/89)).

## [5.0.0] - 2026-11-22

This release has breaking changes. See [MIGRATION.md](MIGRATION.md) for how to upgrade from 4.x.

### Added
- Added `no_std` support through a new `std` feature, enabled by default. Without it croner only needs `alloc` ([#69](https://github.com/Hexagon/croner-rust/pull/69)).
- Added `CronIterator::last_error()`, which tells an iterator that stopped on an error (such as `TimeSearchLimitExceeded`) apart from one that ran out of occurrences.
- `CronError` now implements `Clone`, `PartialEq` and `Eq`.
- `CronError` implements `core::error::Error`, also without `std`. `std::error::Error` is the same trait, so nothing changes for `std` users.
- `CronError` is re-exported at the crate root as `croner::CronError`. `croner::errors::CronError` still works.
- `CronParserBuilder` now implements `Debug`, `Clone`, `Copy`, `PartialEq` and `Eq`.
- Added the `no-std` crates.io category.

### Changed
- **Breaking:** the `jiff` feature now uses jiff 1.0 instead of jiff 0.2. jiff 1.0 renamed `jiff::Zoned` to `jiff::ZonedDateTime`, so croner now takes and returns `jiff::ZonedDateTime`, and code using the `jiff` feature must upgrade to jiff 1.0 as well. jiff 1.0 requires Rust 1.85. See [MIGRATION.md](MIGRATION.md) ([#76](https://github.com/Hexagon/croner-rust/issues/76)).
- **Breaking:** croner now requires Rust 1.85 or newer (`rust-version` is set in `Cargo.toml`), with or without the `jiff` feature. This matches jiff 1.0's minimum.
- **Breaking:** with `default-features = false`, croner is now `no_std`. `CronError` keeps its `Error` impl, but `serde` is built without its `std` feature. Add the `std` feature to keep the previous behaviour, e.g. `features = ["std", "jiff"]`.
- **Breaking:** removed the unused `parser::CronParserBuilderError` type. `CronParserBuilder` is now written by hand instead of with `derive_builder`, which is no longer a dependency; its methods are unchanged.
- **Breaking:** weekday and month names are only accepted in their own fields, and a bare `L` in the day-of-week field is rejected. Patterns that used to parse (and run on an unintended schedule) now return an error; see **Fixed** below.
- Removed the `strum` dependency. The `is_*` methods on `Seconds` and `Year` are now hand-written, `const`, and otherwise unchanged ([#61](https://github.com/Hexagon/croner-rust/pull/61), [#80](https://github.com/Hexagon/croner-rust/pull/80)). Together with the `derive_builder` removal, a build with only the `jiff` feature now compiles 3 crates (croner, jiff, jiff-core) instead of 19.
- `CronIterator` no longer prints errors to stderr, with or without `std`. Use `CronIterator::last_error()` to see why an iterator stopped.
- `CronIterator`'s `PartialEq`, `PartialOrd` and `Hash` ignore `last_error`.
- A parsed `Cron` keeps much less heap: about 160 bytes instead of 5.2 KB for a pattern like `0 0 * * MON-FRI`, and parsing peaks at about 0.5 KB instead of 5.5 KB. A year field that is `*` (or left out) no longer stores one byte per year up to `YEAR_UPPER_LIMIT`. A pattern with an explicit year list or range still does. Equality, ordering and hashing of `Cron`, `CronPattern` and `CronComponent` are unchanged.
- Parsing is about 8x faster, because a `*` field now sets its bits in one pass, and finding the next or previous occurrence scans the bits of a field directly ([#77](https://github.com/Hexagon/croner-rust/pull/77)).

### Fixed
- Weekday names (`MON`…`SUN`) are now only accepted in the day-of-week field and month names (`JAN`…`DEC`) only in the month field. Names in any other field were silently read as the number they stand for (e.g. `0 0 * * jan` ran on Mondays and `mon * * * *` at minute 1); such patterns now fail with `CronError::IllegalCharacters` ([#72](https://github.com/Hexagon/croner-rust/issues/72), [#73](https://github.com/Hexagon/croner-rust/pull/73)).
- A bare `L` in the day-of-week field is now rejected. It has no weekday to apply to, so it was silently dropped: `0 0 * * L` never matched (`find_next_occurrence` ended in `TimeSearchLimitExceeded`) and `0 0 * * 1,L` ran on Mondays only. Such patterns now fail with `CronError::ComponentError`. `L` after a weekday (`5L`, `5#L`) and in day-of-month is unchanged ([#75](https://github.com/Hexagon/croner-rust/pull/75)).
- Fixed fixed-time jobs failing with `CronError::TimeSearchLimitExceeded` when their wall clock time fell in a gap longer than two hours, such as the whole day `Pacific/Apia` skipped on 2011-12-30. The end of a gap was searched one second at a time with a two-hour cap; it is now found by stepping out of the gap and bisecting back, so a gap of any length is crossed in a few dozen time zone lookups, and the documented gap rule applies (e.g. `0 12 * * *` runs at 2011-12-31 00:00 +14:00, and `0 12 30 * *` skips to 2012-01-30) ([#84](https://github.com/Hexagon/croner-rust/issues/84)).

### Documentation
- Corrected the README's description of fixed-time jobs in a spring-forward DST gap. They run at the first valid time after the gap only if that time still matches the day, month and year fields. When the gap ends at midnight, that time is on the next calendar day: a daily job runs there, and a job restricted to the gap's day skips that occurrence (e.g. `30 23 30 * *` in `America/Nuuk` has no run in March 2024) ([#74](https://github.com/Hexagon/croner-rust/issues/74)).
- The README's DST gap section now notes that the gap rule holds for gaps of any length, with `Pacific/Apia` on 2011-12-30 as an example ([#84](https://github.com/Hexagon/croner-rust/issues/84)).

## [4.0.3]

### Fixed
- Fixed fixed-time jobs scheduled in the hour before a spring-forward DST gap also running at the first instant after the gap (e.g. a daily `0 0 * * *` job in `Europe/London` running at both 00:00 and 02:00 on the transition day). Only a scheduled wall clock time that falls inside the gap is moved past it, as documented. Regression since 3.0.0 ([#67](https://github.com/Hexagon/croner-rust/issues/67)).
- Fixed `*/n` (and `/n` with `sloppy_ranges`) being rejected in the day-of-week field when `alternative_weekdays` is enabled.
- `describe_lang` now routes every phrase through `Language`: "hour N", "X past Y", "every N months" and "every N years" were hard-coded English. Added `Language` methods with English defaults (`every_past_phrase`, `past_hour_phrase`, `every_x_months`, `every_x_years`), so existing implementations still compile, and translated them for Swedish.
- Fixed `7L` and `7#L` (last Sunday using `7`) never matching in the day-of-week field.

### Added
- Added `Language::time_of_day()` so a language can format times of day, e.g. with a 12-hour clock.

## [4.0.2]

### Added
- Exported `CronPattern` and `CronComponent`, so the parsed fields of `Cron::pattern` can be named outside the crate.
- Added `CronComponent::values()` and `CronComponent::has_modifiers()` to read a field's plain values and whether it uses `L`, `W` or `#`.
- Added `CronPattern::is_dom_wildcard()`, `is_dow_wildcard()` and `matches_dom_and_dow()` to read how day-of-month and day-of-week combine.

## [4.0.1]

### Fixed
- Fixed the `@weekly` nickname in Quartz weekday mode.

## [4.0.0]

### Added
- Added backend-agnostic searching through the `CronDateTime` trait and shared civil-time abstractions.
- Added an optional `jiff` backend, `simple_demo_jiff`, and backend parity coverage.
- Added OCPS-focused coverage for `LW`, `+`, and strict step-syntax validation.
- Added a dedicated migration guide for 3.x to 4.0 upgrades.
- Added regression tests for backward searches across DST gaps.

### Changed
- Made the `chrono` backend optional while keeping it enabled by default.
- Made occurrence-finding APIs generic over the input datetime type.
- Tightened default step parsing to the OCPS-compliant forms, with `sloppy_ranges(true)` available for backward compatibility.

### Fixed
- Fixed DST overlap iteration and search ordering for both backends.
- Fixed backward fixed-time searches in DST gaps so they return the post-gap scheduled instant instead of a pre-gap placeholder.
- Refined README and example guidance for the 4.0 backend split.

## [3.0.1]

### Fixed
- Fixed natural-language descriptions for patterns such as `* 0 * * *` and `* * 0 * * *`.

### Changed
- Updated dependency versions and refreshed versioned README examples.
