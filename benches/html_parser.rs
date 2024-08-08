use monzo_crawler::parse_links;

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let html = std::fs::read_to_string("./tests/test_data/monzo/home.html").unwrap();

    c.bench_function("parse html", |b| {
        b.iter(|| parse_links(black_box(&html), &"https://monzo.com".parse().unwrap()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
