mod common;

use std::time::Instant;

use behaviortree_rs::prelude::*;
use common::{nodes::StatusNode, trees::deep};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let deep_tree = deep(100);
    let mut registry = NodeRegistry::new();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);

    c.bench_with_input(
        BenchmarkId::new("parse", "deep-100"),
        &(deep_tree, registry),
        |b, (xml, registry)| {
            b.iter_custom(|iters| {
                let config = Tree::builder(xml, registry);

                let start = Instant::now();

                for _ in 0..iters {
                    black_box(config.build().unwrap());
                }

                start.elapsed()
            });
        },
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
