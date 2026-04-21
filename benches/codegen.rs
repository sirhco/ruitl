//! Codegen throughput benchmark. Measures the path from parsed AST to a
//! `TokenStream` for templates of varying nesting depth.

use criterion::{criterion_group, criterion_main, Criterion};
use ruitl_compiler::{parse_str, CodeGenerator};

fn fixture(depth: usize) -> String {
    let mut body = String::from("<div>{title}</div>");
    for _ in 0..depth {
        body = format!("<section>{}</section>", body);
    }
    format!(
        "component C {{ props {{ title: String }} }}\n\
         ruitl C(title: String) {{ {} }}\n",
        body
    )
}

fn bench_codegen(c: &mut Criterion) {
    let shallow = parse_str(&fixture(2)).unwrap();
    let deep = parse_str(&fixture(30)).unwrap();

    let mut group = c.benchmark_group("codegen");
    group.bench_function("shallow", |b| {
        b.iter(|| {
            let mut gen = CodeGenerator::new(shallow.clone());
            gen.generate().unwrap()
        })
    });
    group.bench_function("deep", |b| {
        b.iter(|| {
            let mut gen = CodeGenerator::new(deep.clone());
            gen.generate().unwrap()
        })
    });
    group.finish();
}

criterion_group!(benches, bench_codegen);
criterion_main!(benches);
