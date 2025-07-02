# Croner

Croner is a fully-featured, lightweight, and efficient Rust library designed for parsing and evaluating cron patterns.

This is the Rust flavor of the popular JavaScript/TypeScript cron parser
[croner](https://github.com/hexagon/croner).

## Features

- Parse and evaluate [cron](https://en.wikipedia.org/wiki/Cron#CRON_expression)
  expressions to calculate upcoming execution times.
- Generates human-readable descriptions of cron patterns.
- Follows POSIX/Vixie-cron standards, while extending it with additional specifiers such as `L`
  for the last day and weekday of the month, `#` for the nth weekday of the
  month, `W` for closest weekday to a day of month.
- Evaluate cron expressions across different time zones.
- Supports optional second granularity `.with_seconds_optional` or `.with_seconds_required`
- Supports optional alternative weekday mode to use Quartz-style weekdays instead of POSIX using `with_alternative_weekdays`
- Allows for flexible combination of DOM and DOW conditions, enabling patterns to match specific days of the week in specific weeks of the month or the closest weekday to a specific day.
- Compatible with `chrono` and (optionally) `chrono-tz`.
- Robust error handling.

## Crate Features

- `serde`: Enables [`serde::Serialize`](https://docs.rs/serde/1/serde/trait.Serialize.html) and [`serde::Deserialize`](https://docs.rs/serde/1/serde/trait.Deserialize.html) implementations for [`Cron`](https://docs.rs/croner/2/croner/struct.Cron.html). This feature is disabled by default.

## Why croner instead of cron or saffron?

Croner combines the features of cron and saffron, while following the POSIX/Vixie "standards" for the relevant parts. See this table:

Feature              | Croner      | Cron      | Saffron |
---------------------|-------------|-----------|---------|
Time Zones | X         |    X    |     | 
Ranges (15-25)| X         |    X    |   X   | 
Ranges with stepping (15-25/2)| X         |    X    |   X   |    X   |
`L` - Last day of month | X         |         |   X   |
`5#L` - Last occurrence of weekday |    X     |   X    |       |
`5L` - Last occurrence of weekday |    X     |    ?   |   X    |
`#` - Nth occurrence of weekday |    X     |      |   X    |
`W` - Closest weekday |    X     |        |  X     |
"Standards"-compliant weekdays (1 is monday) |   X    |      |       |
Five part patterns (minute granularity) |  X   |         |    X   |
Six part patterns (second granularity)|  X   |    X    |       |
Weekday/Month text representations |  X   |    X    |   X   |
Aliases (`@hourly` etc.) |  X           |     X      |          |
chrono `DateTime` compatibility |    X     |     X   |   X    |
DOM-and-DOW option |    X     |           |         |
Generate human readable string |    X     |           |    X    |

> **Note**
> Tests carried out at 2023-12-02 using `cron@0.12.0` and `saffron@.0.1.0`

## Getting Started

### Prerequisites

Ensure you have Rust installed on your machine. If not, you can get it from
[the official Rust website](https://www.rust-lang.org/).

### Installation

Add `croner` to your `Cargo.toml` dependencies:

```toml
[dependencies]
croner = "2.2.0" # Adjust the version as necessary
```

### Usage

Here's a quick example to get you started with matching current time, and
finding the next occurrence. `is_time_matching` takes a `chrono` `DateTime`:

```rust
use croner::Cron;
use chrono::Local;

fn main() {

    // Parse cron expression
    let cron_all = Cron::new("18 * * * 5")
      .parse()
      .expect("Couldn't parse cron string");

    // Compare cron pattern with current local time
    let time = Local::now();
    let matches_all = cron_all.is_time_matching(&time).unwrap();

    // Get next match
    let next = cron_all.find_next_occurrence(&time, false).unwrap();

    // Output results
    println!("Description: {}", cron.describe());
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
    let cron = Cron::new("18 * * * 5")
      .parse()
      .expect("Couldn't parse cron string");

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

This example demonstrates how to calculate the next 5 occurrences of New Year's Eve that fall on a Friday. We'll use a cron expression to match every Friday (`FRI`) in December (`12`) and use the `with_dom_and_dow` method to ensure both day of month and day of week conditions are met.

```rust
use croner::Cron;
use chrono::Local;

fn main() {
    // Parse cron expression for Fridays in December
    let cron = Cron::new("0 0 0 31 12 FRI")
      // Include seconds in pattern
      .with_seconds_optional()
      // Ensure both day of month and day of week conditions are met
      .with_dom_and_dow()
      .parse()
      .expect("Couldn't parse cron string");

    let time = Local::now();

    println!("Finding the next 5 New Year's Eves on a Friday:");
    for time in cron.iter_from(time).take(5) {
        println!("{}", time);
    }
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
  - _?_: In the Rust version of croner, a questionmark in the day-of-month or 
    day-of-week field behaves just as *. This allow for legacy cron patterns 
    to be used.
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
| Seconds      | Optional | 0-59            | * , - /                    |                                                                                                                 |
| Minutes      | Yes      | 0-59            | * , - /                    |                                                                                                                 |
| Hours        | Yes      | 0-23            | * , - /                    |                                                                                                                 |
| Day of Month | Yes      | 1-31            | * , - / ? L W              |                                                                                                                 |
| Month        | Yes      | 1-12 or JAN-DEC | * , - /                    |                                                                                                                 |
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

### Configuration

Croner offers several configuration methods to change how patterns are interpreted:

#### 1. `with_seconds_optional()`

This method enables the inclusion of seconds in the cron pattern, but it's not mandatory. By using this method, you can create cron patterns that either include or omit the seconds field. This offers greater flexibility, allowing for more precise scheduling without imposing the strict requirement of defining seconds in every pattern.

**Example Usage**:
```rust
let cron = Cron::new("*/10 * * * * *") // Every 10 seconds
    .with_seconds_optional()
    .parse()
    .expect("Invalid cron pattern");
```

#### 2. `with_seconds_required()`

In contrast to `with_seconds_optional()`, the `with_seconds_required()` method requires the seconds field in every cron pattern. This enforces a high level of precision in task scheduling, ensuring that every pattern explicitly specifies the second at which the task should run.

**Example Usage**:
```rust
let cron = Cron::new("5 */2 * * * *") // At 5 seconds past every 2 minutes
    .with_seconds_required()
    .parse()
    .expect("Invalid cron pattern");
```

#### 3. `with_dom_and_dow()`

This method enables the combination of Day of Month (DOM) and Day of Week (DOW) conditions in your cron expressions. It's particularly useful for creating schedules that require specificity in terms of both the day of the month and the day of the week, such as running a task when the first of the month is a Monday, or christmas day is on a friday.

**Example Usage**:
```rust
let cron = Cron::new("0 0 25 * FRI") // When christmas day is on a friday
    .with_dom_and_dow()
    .parse()
    .expect("Invalid cron pattern");
```

#### 4. `with_alternative_weekdays()` (Quartz mode)

This configuration method switches the weekday mode from the POSIX standard to the Quartz-style, commonly used in Java-based scheduling systems. It's useful for those who are accustomed to Quartz's way of specifying weekdays or for ensuring compatibility with existing Quartz-based schedules.

**Example Usage**:
```rust
let cron = Cron::new("0 0 12 * * 6") // Every Friday (denoted with 6 in Quartz mode) at noon
    .with_alternative_weekdays()
    .parse()
    .expect("Invalid cron pattern");
```

### Documentation

For detailed usage and API documentation, visit
[Croner on docs.rs](https://docs.rs/croner/).

## Development

To start developing in the Croner project:

1. Clone the repository.
2. Navigate into the project directory.
3. Build the project using `cargo build`.
4. Run tests with `cargo test`.
5. Run demo with `cargo run --example simple_demo`

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
