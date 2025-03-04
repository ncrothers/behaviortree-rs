mod common;

use std::time::Instant;

use behaviortree_rs::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

#[derive(Clone)]
struct ExpensiveValue {
    data: Vec<u32>,
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("get vs get_ref");

    group.bench_with_input(BenchmarkId::new("get", "i32"), &10i32, |b, input| {
        b.iter_custom(|iters| {
            let mut bb = Blackboard::new();

            bb.set("value", *input);

            let start = Instant::now();

            for _ in 0..iters {
                black_box(bb.get_exact::<i32>("value"));
            }

            start.elapsed()
        });
    });

    group.bench_with_input(BenchmarkId::new("get_ref", "i32"), &10i32, |b, input| {
        b.iter_custom(|iters| {
            let mut bb = Blackboard::new();

            bb.set("value", *input);

            let start = Instant::now();

            for _ in 0..iters {
                black_box(bb.get_exact_ref::<i32>("value"));
            }

            start.elapsed()
        });
    });

    let expensive_value = ExpensiveValue {
        data: black_box((0..1024).collect()),
    };

    group.bench_with_input(
        BenchmarkId::new("get", "ExpensiveValue"),
        &expensive_value,
        |b, input| {
            b.iter_custom(|iters| {
                let mut bb = Blackboard::new();

                bb.set("value", input.clone());

                let start = Instant::now();

                for _ in 0..iters {
                    black_box(bb.get_exact::<ExpensiveValue>("value"));
                }

                start.elapsed()
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("get_ref", "ExpensiveValue"),
        &expensive_value,
        |b, input| {
            b.iter_custom(|iters| {
                let mut bb = Blackboard::new();

                bb.set("value", input.clone());

                let start = Instant::now();

                for _ in 0..iters {
                    black_box(bb.get_exact_ref::<ExpensiveValue>("value"));
                }

                start.elapsed()
            });
        },
    );

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
