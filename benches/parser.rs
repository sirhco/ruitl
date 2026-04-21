//! Parser throughput benchmark. Exercises `ruitl_compiler::parse_str` on
//! templates of three sizes so regressions in lexer/parser cost surface as
//! distinct curves.

use criterion::{criterion_group, criterion_main, Criterion};
use ruitl_compiler::parse_str;

fn fixture(list_items: usize) -> String {
    let mut s = String::from(
        "component Bench {\n\
            props {\n\
                title: String,\n\
                items: Vec<String>,\n\
            }\n\
        }\n\n\
        ruitl Bench(title: String, items: Vec<String>) {\n\
            <div>\n\
                <h1>{title}</h1>\n\
                <ul>\n",
    );
    for _ in 0..list_items {
        s.push_str("                    for item in items { <li class=\"x\">{item}</li> }\n");
    }
    s.push_str("                </ul>\n            </div>\n        }\n");
    s
}

fn bench_parse(c: &mut Criterion) {
    let small = fixture(4);
    let medium = fixture(50);
    let large = fixture(500);

    let mut group = c.benchmark_group("parse_str");
    group.bench_function("small", |b| b.iter(|| parse_str(&small).unwrap()));
    group.bench_function("medium", |b| b.iter(|| parse_str(&medium).unwrap()));
    group.bench_function("large", |b| b.iter(|| parse_str(&large).unwrap()));
    group.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
