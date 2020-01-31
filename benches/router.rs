use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use nuclear_router::Router;

fn router_find(c: &mut Criterion) {
    let mut group = c.benchmark_group("router-find");

    group.bench_function("single-route", |b| {
        let mut router: Router<usize> = Router::new();
        router.insert("/hello/:name", 1);
        b.iter_with_large_drop(|| router.find("/hello/world"))
    });
}

fn router_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("router-insert");

    group.bench_function("single-route", |b| {
        b.iter_batched_ref(
            Router::new,
            |router: &mut Router<usize>| {
                router.insert("/hello/:name", 1);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, router_find, router_insert);
criterion_main!(benches);
