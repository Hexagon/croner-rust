use std::hint::black_box;

use chrono::Local;
use criterion::{criterion_group, criterion_main, Criterion};
use croner::{parser::CronParser, Cron};

fn parse_take_100(_n: u64) {
    let cron: Cron = CronParser::builder()
        .seconds(croner::parser::Seconds::Optional)
        .build()
        .parse("15 15 15 L 3 *")
        .expect("Couldn't parse cron string");
    let time = Local::now();
    for _time in cron.clone().iter_after(time).take(100) {}
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("parse_take_100", |b| {
        b.iter(|| parse_take_100(black_box(20)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
