#![cfg(any(feature = "chrono", feature = "jiff"))]

use croner::parser::{CronParser, Seconds, Year};
use croner::time::CronDateTime;
use croner::Direction;

fn check_weekly_alias<T: CronDateTime + std::fmt::Debug + PartialEq>(
    alternative_weekdays: bool,
    start: T,
    sunday: T,
    following_sunday: T,
) {
    for (seconds, year) in [
        (Seconds::Disallowed, Year::Disallowed),
        (Seconds::Optional, Year::Optional),
        (Seconds::Required, Year::Disallowed),
        (Seconds::Required, Year::Required),
    ] {
        let parser = CronParser::builder()
            .alternative_weekdays(alternative_weekdays)
            .seconds(seconds)
            .year(year)
            .build();

        for alias in ["@weekly", "@WEEKLY", "@WeEkLy"] {
            let cron = parser.parse(alias).unwrap();
            assert!(!cron.is_time_matching(&start).unwrap());
            assert!(cron.is_time_matching(&sunday).unwrap());
            assert_eq!(
                cron.iter_from(start.clone(), Direction::Forward)
                    .take(2)
                    .collect::<Vec<_>>(),
                vec![sunday.clone(), following_sunday.clone()]
            );
        }
    }
}

#[cfg(feature = "chrono")]
#[rstest::rstest]
#[case(false)]
#[case(true)]
fn weekly_alias_chrono(#[case] alternative_weekdays: bool) {
    use chrono::{TimeZone, Utc};

    check_weekly_alias(
        alternative_weekdays,
        Utc.with_ymd_and_hms(2026, 9, 26, 0, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2026, 9, 27, 0, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2026, 10, 4, 0, 0, 0).unwrap(),
    );
}

#[cfg(feature = "jiff")]
#[rstest::rstest]
#[case(false)]
#[case(true)]
fn weekly_alias_jiff(#[case] alternative_weekdays: bool) {
    check_weekly_alias(
        alternative_weekdays,
        "2026-09-26T00:00:00Z[UTC]".parse::<jiff::Zoned>().unwrap(),
        "2026-09-27T00:00:00Z[UTC]".parse::<jiff::Zoned>().unwrap(),
        "2026-10-04T00:00:00Z[UTC]".parse::<jiff::Zoned>().unwrap(),
    );
}
