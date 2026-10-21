# Changelog

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
