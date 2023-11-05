# Croner (Rust Edition)

**Work in progress**

Croner is a lightweight, efficient Rust library for parsing and handling cron patterns. Designed with simplicity and performance in mind, it provides Rust developers with a tool to schedule tasks efficiently, following the familiar cron syntax.

This is the **Work in progress** Rust flavor of the popular JavaScript/TypeScript cron scheduler [https://github.com/hexagon/croner](croner).

## Features

- Parse standard cron patterns with ease.
- Validate cron expressions against customizable time ranges.
- Check if a `DateTime` matches a given cron pattern.
- Supports last day of month (`L`) field.

## Getting Started

### Prerequisites

Ensure you have Rust installed on your machine. If not, you can get it from [the official Rust website](https://www.rust-lang.org/).

### Installation

Add `croner` to your `Cargo.toml` dependencies:

```toml
[dependencies]
croner = "7.0.5" # Adjust the version as necessary
```

### Usage

Here's a quick example to get you started:

```rust
use croner::CronPattern;
use chrono::Local;

fn main() {
    let pattern_str = "0 30 8 * * *"; // Every day at 8:30:00
    let cron_pattern = CronPattern::new(pattern_str).expect("Pattern should be valid");

    let now = Local::now();
    if cron_pattern.is_time_matching(&now) {
        println!("The cron pattern matches the current time!");
    }
}
```

## Development

To start developing in the Croner project:

1. Clone the repository.
2. Navigate into the project directory.
3. Build the project using `cargo build`.
4. Run tests with `cargo test`.

## Contributing

We welcome contributions! Please feel free to submit a pull request or open an issue.

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE) file for details.

## Acknowledgments

- Thanks to the `chrono` crate for providing robust date and time handling in Rust.
- This project adheres to Semantic Versioning.

## Disclaimer

This is an early version of Croner, and the API is subject to change.

## Contact

If you have any questions or feedback, please open an issue in the repository and we'll get back to you as soon as possible.