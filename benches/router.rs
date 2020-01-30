use criterion::{criterion_group, criterion_main, Criterion};
use nuclear_router::Router;

fn router_find(c: &mut Criterion) {
    let mut group = c.benchmark_group("router-find");

    group.bench_function("single", |b| {
        let mut router: Router<()> = Router::new();
        router.insert("/hello/:name", ());
        b.iter(|| {
            let ret = router.find("/hello/world");
            assert!(ret.is_some())
        })
    });
}

criterion_group!(benches, router_find);
criterion_main!(benches);
