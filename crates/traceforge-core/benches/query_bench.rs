use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use traceforge_core::{Scenario, SearchIndex, generate_events, parse_query};

fn compare_index_and_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_outcome_failure_and_user");
    for size in [1_000_usize, 10_000, 100_000] {
        let index = SearchIndex::build(generate_events(size, 42, Scenario::Mixed));
        let query = "outcome:failure AND user:ana";
        let expr = parse_query(query).unwrap();
        group.bench_with_input(BenchmarkId::new("indexed", size), &size, |b, _| {
            b.iter(|| black_box(index.query(query, usize::MAX).unwrap().matches.len()))
        });
        group.bench_with_input(BenchmarkId::new("linear", size), &size, |b, _| {
            b.iter(|| black_box(index.linear_scan(&expr).len()))
        });
    }
    group.finish();
}

criterion_group!(benches, compare_index_and_scan);
criterion_main!(benches);
