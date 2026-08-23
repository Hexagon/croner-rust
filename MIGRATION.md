# Migration from 3.x to 4.0

Version 4.0 introduces a generic `CronDateTime` trait and an optional `jiff` backend.
Most code needs **no changes** if you use the default features. The changes below
cover every case that may need adjustment.

---

#### 1. Chrono is now an optional feature (still on by default)

In 3.x, `chrono` was always included. In 4.0 it is behind the optional `chrono`
feature flag, which remains in the default features. If your `Cargo.toml` uses
`default-features = false`, you must add `chrono` or `jiff` explicitly:

```toml
# Before (3.x)
[dependencies]
croner = { version = "3", default-features = false, features = ["serde"] }

# After (4.0) — add the backend feature
[dependencies]
croner = { version = "4", default-features = false, features = ["chrono", "serde"] }
```

If you used named time zones with chrono, keep `chrono-tz` in your own
dependencies:

```toml
[dependencies]
croner = "4.0"
chrono-tz = "0.10"
```

---

#### 2. Step syntax validation is now stricter (breaking)

In 3.x, the parser accepted shortcut step syntax like `5/5 * * * *` (meaning
"every five minutes starting at minute 5"). In 4.0 this is **rejected** by
default — you must write the explicit range `5-59/5 * * * *`.

```rust
// Before (3.x) — accepted
let cron = Cron::from_str("5/5 * * * *").unwrap();

// After (4.0) — rejected
let cron = Cron::from_str("5/5 * * * *").unwrap(); // Error!

// After (4.0) — write the range explicitly
let cron = Cron::from_str("5-59/5 * * * *").unwrap();
```

To restore the old lenient behaviour, build a parser with
`sloppy_ranges(true)`:

```rust
use croner::parser::CronParser;

let cron = CronParser::builder()
    .sloppy_ranges(true)
    .build()
    .parse("5/5 * * * *")
    .unwrap();
```

---

#### 3. Method signatures are now generic over `CronDateTime`

All datetime methods previously bound on `chrono::TimeZone` now bind on
croner's own `CronDateTime` trait. At the call site the code looks the same
— the generic is inferred from the argument type:

```rust
// Works in both 3.x and 4.0:
let cron = Cron::from_str("0 0 * * FRI").unwrap();
let next = cron.find_next_occurrence(&Utc::now(), false).unwrap();
```

The return type follows the input type, so you can also pass a
`chrono::NaiveDateTime`, or a `jiff::Zoned`, or a `jiff::civil::DateTime`:

```rust
// 4.0 — works with any backend
use jiff::Zoned;

let cron = Cron::from_str("0 0 * * FRI").unwrap();
let next: Zoned = cron.find_next_occurrence(&Zoned::now(), false).unwrap();
```

---
#### 4. Returning two values from `find_occurrence` (removed)

In 3.x, `find_occurrence` returned `(DateTime<Tz>, Option<DateTime<Tz>>)`, where the
second element carried the later half of a DST-repeated hour. In 4.0 the
method is **no longer public** (it was never meant to be), and the return is
a single `T`.

```rust
// Before (3.x) — only if you called find_occurrence directly
let (dt, _second) = cron.find_occurrence(&start, false, Direction::Forward)?;

// After (4.0)
let dt = cron.find_next_occurrence(&start, false)?;
```

---

#### 5. Turbofish: when inference needs help

Most of the time Rust infers the return type from the argument. When both
`chrono` and `jiff` features are enabled, or when the value's concrete type
isn't fully determined by context, the compiler may need a turbofish:

```rust
// If you see "type annotation needed", add the turbofish:
let next = cron
    .find_next_occurrence::<chrono::DateTime<chrono::Utc>>(&chrono::Utc::now(), false)
    .unwrap();
```

---

#### 6. `iter_after`, `iter_before`, `iter_from` take `T` by value

These methods already took `DateTime<Tz>` by value in 3.x, so most call sites
do not change. The iterator is now `CronIterator<T>` instead of
`CronIterator<Tz>`:

```rust
// Both 3.x and 4.0 — identical call site
for dt in cron.iter_after(Utc::now()).take(5) { ... }
```

If you stored a `CronIterator<Tz>` in a struct, replace `Tz` with your
concrete type or annotate the generic parameter:

```rust
// Before
struct MyScheduler<Tz: TimeZone> {
    iter: CronIterator<Tz>,
}

// After
struct MyScheduler<T: CronDateTime> {
    iter: CronIterator<T>,
}
```

---

#### 7. Deprecated: `from_naive`

The crate-level function `croner::from_naive` is deprecated. Call
`chrono::TimeZone::from_local_datetime` directly instead:

```rust
// Before
use croner::from_naive;
let dt = from_naive(naive, &tz).single().unwrap();

// After
use chrono::TimeZone;
let dt = tz.from_local_datetime(&naive).unwrap();
```

---

#### 8. New public types

Croner now exports backend-agnostic date and time types that
implementations of `CronDateTime` bridge to. These are re-exported from the
crate root:

```rust
use croner::{CivilDate, CivilDateTime, CivilTime, CronDateTime, Resolution, Weekday};
```

Implement `CronDateTime` for your own type to use a different date and time
library.

---

#### 9. New `jiff` backend

Add `jiff` support without `chrono`:

```toml
[dependencies]
croner = { version = "4.0", default-features = false, features = ["jiff"] }
jiff = "0.2"
```

```rust
use jiff::Zoned;
use croner::Cron;
use std::str::FromStr as _;

let cron = Cron::from_str("0 0 * * FRI").unwrap();
let next: Zoned = cron.find_next_occurrence(&Zoned::now(), false).unwrap();
```

All search methods, iterators, and DST handling work identically. The two
backends agree on the same behaviour (verified by shared test patterns).

---

#### Quick reference

| Concern | 3.x | 4.0 |
|---------|-----|-----|
| Default backend | `chrono` (always available) | `chrono` (optional, default) |
| Alternate backend | — | `jiff` (optional) |
| Method bounds | `Tz: TimeZone` | `T: CronDateTime` |
| `find_occurrence` | `pub`, returned tuple | removed from public API |
| `from_naive` | public fn | deprecated |
| Step syntax `5/5` | accepted by default | rejected; use `sloppy_ranges(true)` |
| `find_next_occurrence` return | `DateTime<Tz>` | `T` (same as input) |
| `CronIterator` type | `CronIterator<Tz>` | `CronIterator<T>` |
| `iter_after` etc. | by value | by value (unchanged) |
