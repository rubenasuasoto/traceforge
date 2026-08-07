# Benchmark methodology

Benchmarks answer one narrow question: how does `outcome:failure AND user:ana` behave with the built index versus the reference linear evaluator on deterministic synthetic data?

## Method

- Release build with LTO, one codegen unit and Rust 1.97.1 stable.
- Seed `42`, mixed scenario, event counts 1k, 10k, 100k and 1M.
- 100, 50, 20 and 3 measured query iterations respectively.
- Index construction is timed separately.
- `black_box` retains query work; indexed and linear match counts must be equal.
- Raw JSON includes UTC date, OS, architecture, processor string and logical processor count.

## Results from 2026-08-07

| Events | Build ns | Indexed mean ns | Linear mean ns | Matches | Observed ratio |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 10,930,500 | 24,911 | 322,222 | 26 | 12.93× |
| 10,000 | 99,940,700 | 241,730 | 3,813,424 | 221 | 15.78× |
| 100,000 | 1,169,467,900 | 3,386,855 | 35,372,590 | 2,149 | 10.44× |
| 1,000,000 | 9,760,197,600 | 41,847,866 | 10,217,521,300 | 20,861 | 244.16× |

Machine: Windows x86-64, Intel64 Family 6 Model 94 Stepping 3, 4 logical processors. These numbers include result materialization and reflect one machine, data distribution and query. They are evidence for this run only. OS caching, allocator state, power policy and small iteration count at 1M can materially change results.

Criterion benches in `crates/traceforge-core/benches` provide statistically sampled 1k–100k comparisons. The CLI command records larger manual runs without making shared-runner CI unstable.

