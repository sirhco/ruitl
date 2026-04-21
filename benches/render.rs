//! Render throughput benchmark. Builds a non-trivial `Html` tree at setup
//! time and measures the cost of `render()` — i.e. the runtime path users
//! actually hit when serving each request.

use criterion::{criterion_group, criterion_main, Criterion};
use ruitl::html::*;

fn tree(list_size: usize) -> Html {
    let mut items = Vec::with_capacity(list_size);
    for i in 0..list_size {
        items.push(Html::Element(
            HtmlElement::new("li")
                .class("item")
                .child(Html::text(format!("item #{}", i))),
        ));
    }
    Html::Element(
        HtmlElement::new("section")
            .attr("data-id", "bench")
            .child(Html::Element(HtmlElement::new("h1").text("Benchmark")))
            .child(Html::Element(HtmlElement::new("ul").children(items))),
    )
}

fn bench_render(c: &mut Criterion) {
    let small = tree(10);
    let big = tree(1000);

    let mut group = c.benchmark_group("Html::render");
    group.bench_function("small_10", |b| b.iter(|| small.render()));
    group.bench_function("big_1000", |b| b.iter(|| big.render()));
    group.finish();
}

criterion_group!(benches, bench_render);
criterion_main!(benches);
