//! `compile_dir_sibling` throughput benchmark.
//!
//! Generates N synthetic `.ruitl` files in a tempdir and measures the full
//! parse + codegen + write pipeline. Useful for tracking parallel speedups
//! (compare with/without the `parallel` feature on the `ruitl_compiler`
//! dependency).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ruitl_compiler::compile_dir_sibling;
use std::fs;
use tempfile::TempDir;

fn synth_template(idx: usize) -> String {
    format!(
        r#"// Synthetic fixture #{idx}.
component Card{idx} {{
    props {{
        title: String,
        body: String,
    }}
}}

ruitl Card{idx}(title: String, body: String) {{
    <article class="card-{idx}">
        <h1>{{title}}</h1>
        <p>{{body}}</p>
    </article>
}}
"#
    )
}

fn populate(dir: &std::path::Path, count: usize) {
    for i in 0..count {
        fs::write(dir.join(format!("Card{i}.ruitl")), synth_template(i)).unwrap();
    }
}

fn bench_compile_dir(c: &mut Criterion) {
    let mut group = c.benchmark_group("compile_dir_sibling");
    for size in &[10usize, 100, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &count| {
            b.iter_custom(|iters| {
                let mut total = std::time::Duration::ZERO;
                for _ in 0..iters {
                    // Fresh tempdir per iteration so the hash-skip cache is
                    // cold — we want to measure the full parse+codegen
                    // pipeline, not the no-op fast path.
                    let dir = TempDir::new().unwrap();
                    populate(dir.path(), count);
                    let start = std::time::Instant::now();
                    compile_dir_sibling(dir.path()).unwrap();
                    total += start.elapsed();
                }
                total
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_compile_dir);
criterion_main!(benches);
