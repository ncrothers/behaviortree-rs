mod nodes;
mod trees;

use std::time::Instant;

use behaviortree_rs::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nodes::StatusNode;
use trees::{deep, shallow};

fn registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);

    registry
}

fn deep_tree_n(depth: u32) -> Tree {
    let xml = deep(depth);

    let registry = registry();

    let config = TreeConfig::builder()
        .registry(&registry)
        .xml(&xml)
        .build();

    Tree::from_config(&config).unwrap()
}

fn shallow_tree() -> Tree {
    let xml = shallow();

    let registry = registry();

    let config = TreeConfig::builder()
        .registry(&registry)
        .xml(&xml)
        .build();

    Tree::from_config(&config).unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("shallow tree - manual", |b| {
        b.iter_custom(|iters| {
            let mut tree = shallow_tree();

            let start = Instant::now();
            
            for _ in 0..iters {
                black_box(tree.tick_once().unwrap());
            }
            
            start.elapsed()
        });
    });
    
    c.bench_function("deep tree - 100 - manual", |b| {
        b.iter_custom(|iters| {
            let mut tree = deep_tree_n(100);

            let start = Instant::now();
            
            for _ in 0..iters {
                black_box(tree.tick_once().unwrap());
            }
            
            start.elapsed()
        });
    });

    c.bench_function("deep tree - 1000 - manual", |b| {
        b.iter_custom(|iters| {
            let mut tree = deep_tree_n(1000);

            let start = Instant::now();
            
            for _ in 0..iters {
                black_box(tree.tick_once().unwrap());
            }
            
            start.elapsed()
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);