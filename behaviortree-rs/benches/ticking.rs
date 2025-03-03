mod nodes;
mod trees;

use std::time::Instant;

use behaviortree_rs::{prelude::*, tree::AsyncTree};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nodes::StatusNode;
use trees::{deep, shallow};

fn factory() -> Factory {
    let mut factory = Factory::new();

    register_action_node!(factory, "StatusNode", StatusNode);

    factory
}

fn deep_tree_n(depth: u32) -> AsyncTree {
    let xml = deep(depth);

    let mut factory = factory();

    let bb = Blackboard::create();

    factory.create_async_tree_from_text(xml, &bb).unwrap()
}

fn shallow_tree() -> AsyncTree {
    let xml = shallow();

    let mut factory = factory();

    let bb = Blackboard::create();

    factory.create_async_tree_from_text(xml, &bb).unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();    

    c.bench_function("shallow tree - manual", |b| {
        b.to_async(&rt).iter_custom(|iters| {
            async move {
                let mut tree = shallow_tree();
    
                let start = Instant::now();
                
                for _ in 0..iters {
                    black_box(tree.tick_once().await.unwrap());
                }
                
                start.elapsed()
            }
        });
    });

    c.bench_function("deep tree - 100 - manual", |b| {
        b.to_async(&rt).iter_custom(|iters| {
            async move {
                let mut tree = deep_tree_n(100);
    
                let start = Instant::now();
                
                for _ in 0..iters {
                    black_box(tree.tick_once().await.unwrap());
                }
                
                start.elapsed()
            }
        });
    });

    c.bench_function("deep tree - 1000 - manual", |b| {
        b.to_async(&rt).iter_custom(|iters| {
            async move {
                let mut tree = deep_tree_n(1000);
    
                let start = Instant::now();
                
                for _ in 0..iters {
                    black_box(tree.tick_once().await.unwrap());
                }
                
                start.elapsed()
            }
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);