# Croner

Croner is a fully-featured, lightweight, and efficient Rust library designed for parsing and evaluating cron patterns.

This is the Rust flavor of the popular JavaScript/TypeScript cron parser
[croner](https://github.com/hexagon/croner).

## Features

- Parse and evaluate [cron](https://en.wikipedia.org/wiki/Cron#CRON_expression)
  expressions to calculate upcoming execution times.
- Supports extended Vixie-cron patterns with additional specifiers such as `L`
  for the last day and weekday of the month, `#` for the nth weekday of the
  month and `W` for closest weekday to a day of month.
- Evaulate cron expressions across different time zones.
- Compatible with `chrono` and (optionally) `chrono-tz`.
- Robust error handling.

## Why croner instead of cron or saffron?

Croner combines the features of cron and saffron, while having fewer dependencies and following the "standards". See this table:

Feature              | Croner      | Cron      | Saffron |
---------------------|-------------|-----------|---------|
Time Zones | X         |    X    |     | 
Ranges (15-25)| X         |    X    |   X   | 
Ranges with stepping (15-25/2)| X         |    X    |   X   |    X   |
`L` - Last day of month | X         |         |   X   |
`L` - Last occurrence of weekday |    X     |       |       |
`#` - Nth occurrence of weekday |    X     |      |   X    |
`W` - Closest weekday |    X     |        |  X     |
"Standards"-compliant weekdays (1 is monday) |   X    |      |       |
Five part patterns (minute granularity) |  X   |         |    X   |
Six part patterns (second granularity)|  X   |    X    |       |
Weekday/Month text representations |  X   |    X    |   X   |
Aliases (`@hourly` etc.) |  X           |     X      |          |
chrono `DateTime` compatibility |    X     |     X   |   X    |

> **Note**
> Tests carried out at 2023-12-02 using `cron@0.12.0` and `saffron@.0.1.0`

## Getting Started

### Prerequisites

Ensure you have Rust installed on your machine. If not, you can get it from
[the official Rust website](https://www.rust-lang.org/).

### Installation

Add `croner` to your `Cargo.toml` dependencies:

**Please note that croner for Rust is work in progress, and not production
ready**

```toml
[dependencies]
croner = "1.0.0" # Adjust the version as necessary
```

### Usage

Here's a quick example to get you started with matching current time, and
finding the next occurrence. `is_time_matching` takes a `chrono` `DateTime`:

```rust
use croner::Cron;
use chrono::Local;

fn main() {

    // Parse cron expression
    let cron_all: Cron = "0 18 * * * 5".parse().expect("Couldn't parse cron string");

    // Compare cron pattern with current local time
    let time = Local::now();
    let matches_all = cron_all.is_time_matching(&time).unwrap();

    // Get next match
    let next = cron_all.find_next_occurrence(&time, false).unwrap();

    // Output results
    println!("Time is: {}", time);
    println!("Pattern \"{}\" does {} time {}", cron_all.pattern.to_string(), if matches_all { "match" } else { "not match" }, time );
    println!("Pattern \"{}\" will match next time at {}", cron_all.pattern.to_string(), next);

}
```

To match against a non local timezone, croner supports zoned chrono DateTime's
`DateTime<Tz>`. To use a named time zone, you can utilize the `chrono-tz` crate.

```rust
use croner::Cron;
use chrono::Local;
use chrono_tz::Tz;

fn main() {
    // Parse cron expression
    let cron = Cron::parse("0 18 * * * 5").expect("Couldn't parse cron string");

    // Choose a different time zone, for example America/New_York
    let est_timezone: Tz = "America/New_York".parse().expect("Invalid timezone");

    // Find the next occurrence in EST
    let time_est = Local::now().with_timezone(&est_timezone);
    let next_est = cron.find_next_occurrence(&time_est, false).unwrap();

    // Output results for EST
    println!("EST time is: {}", time_est);
    println!(
        "Pattern \"{}\" will match next time at (EST): {}",
        cron.pattern.to_string(),
        next_est
    );
}
```

### Pattern

The expressions used by Croner are very similar to those of Vixie Cron, but with
a few additions and changes as outlined below:

```javascript
// ┌──────────────── (optional) second (0 - 59)
// │ ┌────────────── minute (0 - 59)
// │ │ ┌──────────── hour (0 - 23)
// │ │ │ ┌────────── day of month (1 - 31)
// │ │ │ │ ┌──────── month (1 - 12, JAN-DEC)
// │ │ │ │ │ ┌────── day of week (0 - 6, SUN-Mon)
// │ │ │ │ │ │       (0 to 6 are Sunday to Saturday; 7 is Sunday, the same as 0)
// │ │ │ │ │ │
// * * * * * *
```

- Croner expressions have the following additional modifiers:
  - _?_: In the Rust version of croner, a questionmark behaves just as *, to
    allow for legacy cron patterns to be used.
  - _L_: The letter 'L' can be used in the day of the month field to indicate
    the last day of the month. When used in the day of the week field in
    conjunction with the # character, it denotes the last specific weekday of
    the month. For example, `5#L` represents the last Friday of the month.
  - _#_: The # character specifies the "nth" occurrence of a particular day
    within a month. For example, supplying `5#2` in the day of week field
    signifies the second Friday of the month. This can be combined with ranges
    and supports day names. For instance, MON-FRI#2 would match the Monday
    through Friday of the second week of the month.
  - _W_: The character 'W' is used to specify the closest weekday to a given day
    in the day of the month field. For example, 15W will match the closest
    weekday to the 15th of the month. If the specified day falls on a weekend
    (Saturday or Sunday), the pattern will match the closest weekday before or
    after that date. For instance, if the 15th is a Saturday, 15W will match the
    14th (Friday), and if the 15th is a Sunday, it will match the 16th (Monday).

| Field        | Required | Allowed values  | Allowed special characters | Remarks                                                                                                         |
| ------------ | -------- | --------------- | -------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Seconds      | Optional | 0-59            | * , - / ?                  |                                                                                                                 |
| Minutes      | Yes      | 0-59            | * , - / ?                  |                                                                                                                 |
| Hours        | Yes      | 0-23            | * , - / ?                  |                                                                                                                 |
| Day of Month | Yes      | 1-31            | * , - / ? L W              |                                                                                                                 |
| Month        | Yes      | 1-12 or JAN-DEC | * , - / ?                  |                                                                                                                 |
| Day of Week  | Yes      | 0-7 or SUN-MON  | * , - / ? # L              | 0 to 6 are Sunday to Saturday<br>7 is Sunday, the same as 0<br># is used to specify nth occurrence of a weekday |

> **Note** Weekday and month names are case-insensitive. Both `MON` and `mon`
> work. When using `L` in the Day of Week field, it affects all specified
> weekdays. For example, `5-6#L` means the last Friday and Saturday in the
> month." The # character can be used to specify the "nth" weekday of the month.
> For example, 5#2 represents the second Friday of the month.

It is also possible to use the following "nicknames" as pattern.

| Nickname   | Description                        |
| ---------- | ---------------------------------- |
| \@yearly   | Run once a year, ie. "0 0 1 1 *".  |
| \@annually | Run once a year, ie. "0 0 1 1 *".  |
| \@monthly  | Run once a month, ie. "0 0 1 * *". |
| \@weekly   | Run once a week, ie. "0 0 * * 0".  |
| \@daily    | Run once a day, ie. "0 0 * * *".   |
| \@hourly   | Run once an hour, ie. "0 * * * *". |

### Documentation

For detailed usage and API documentation, visit
[Croner on docs.rs](https://docs.rs/croner/).

## Development

To start developing in the Croner project:

1. Clone the repository.
2. Navigate into the project directory.
3. Build the project using `cargo build`.
4. Run tests with `cargo test`.
5. Run demo with `cargo run --example pattern_demo`

## Contributing

We welcome contributions! Please feel free to submit a pull request or open an
issue.

## License

This project is licensed under the MIT License - see the
[LICENSE.md](LICENSE.md) file for details.

## Disclaimer

Please note that Croner is currently in its early stages of development. As
such, the API is subject to change in future releases, adhering to semantic
versioning principles. We recommend keeping this in mind when integrating Croner
into your projects.

## Contact

If you have any questions or feedback, please open an issue in the repository
and we'll get back to you as soon as possible.
