use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use nuclear_router::Router;

fn router_find(c: &mut Criterion) {
    let mut group = c.benchmark_group("router-find");

    group.bench_function("single-route", |b| {
        let mut router: Router<usize> = Router::new();
        router.insert("/hello/:name", 1);
        b.iter_with_large_drop(|| router.find("/hello/world"))
    });

    group.bench_function("small-routes", |b| {
        let mut router: Router<usize> = Router::new();
        router.insert("/posts/:post_id/comments/:id", 1);
        router.insert("/posts/:post_id/comments", 2);
        router.insert("/posts/:post_id", 3);
        router.insert("/posts", 4);
        router.insert("/comments", 5);
        router.insert("/comments/:id", 6);
        b.iter_with_large_drop(|| router.find("/posts/100/comments/200"))
    });

    group.bench_function("large-routes", |b| {
        let mut router: Router<usize> = Router::new();
        let mut pattern = String::new();
        for i in 0..26 {
            pattern.push('/');
            let c = std::char::from_u32('a' as u32 + i).unwrap();
            pattern.push(c);
        }
        for i in 0..512 {
            let pattern = format!("{}/{}", pattern, i);
            router.insert(&pattern, i);
        }
        pattern.push_str("/128");
        b.iter_with_large_drop(|| router.find(&pattern))
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

    group.bench_function("large-route", |b| {
        let mut pattern = String::new();
        for i in 0..26 {
            pattern.push('/');
            let c = std::char::from_u32('a' as u32 + i).unwrap();
            pattern.push(c);
        }
        pattern.push_str("/128");

        b.iter_batched_ref(
            Router::new,
            |router: &mut Router<usize>| {
                router.insert(&pattern, 1);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, router_find, router_insert);
criterion_main!(benches);
