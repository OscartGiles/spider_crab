use monzo_crawler::{parse_links, PageContent};

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let html = std::fs::read_to_string("./tests/test_data/monzo/home.html").unwrap();

    let page = PageContent {
        url: "https://monzo.com".parse().unwrap(),
        status_code: reqwest::StatusCode::OK,
        content: html.clone(),
        content_type: Some("text/html".parse().unwrap()),
    };

    c.bench_function("parse html", |b| b.iter(|| parse_links(black_box(&page))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
