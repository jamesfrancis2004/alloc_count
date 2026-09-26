# alloc_count

A lightweight Rust library to manually count allocations per scope using a configurable thread-local `GlobalAlloc`.

## Getting Started
It's as easy as dropping the allocator in your root crate config and calling the macro!

```rust
use std::alloc::System;
use alloc_count::{alloc_count, AllocCounter};

#[global_allocator]
static GLOBAL: AllocCounter<System> = AllocCounter(System);

fn main() {
    let (stats, result) = alloc_count!({
        let mut vec = Vec::new();
        // Vec allocations usually happen when capacity expands.
        for i in 0..100 {
            vec.push(i);
        }
        vec
    });

    println!("Loop push: {} allocations, {} bytes requested", stats.alloc_calls, stats.bytes_allocated);
}
```

Expressions can also be measured directly without braces:
```rust
let (stats, boxed) = alloc_count!(Box::new(42));
```

## Snapshot Tracking (`AllocSnapshot`)

If wrapping code in a macro block is inconvenient or you want to track allocations across function boundaries, use `AllocSnapshot` (similar to `std::time::Instant`):

```rust
use alloc_count::{alloc_ignore, AllocSnapshot};

let snapshot = AllocSnapshot::now();

let _vec = vec![1, 2, 3];

// alloc_ignore! works seamlessly with snapshots as well:
alloc_ignore!(println!("Vector contents: {:?}", _vec));

let stats = snapshot.elapsed();
println!("Allocations during work: {}", stats.alloc_calls); // Only counts _vec!
```

Any allocations wrapped inside `alloc_ignore!` are excluded from `AllocSnapshot::elapsed()` as well.

## Testing Assertion Macros

`alloc_count` provides dedicated assertion macros for CI and regression testing:

```rust
use alloc_count::{assert_no_alloc, assert_max_allocs, assert_max_bytes};

// Assert zero heap allocations occur
let result = assert_no_alloc!({
    let x = 10;
    x + 20
});

// Assert a maximum allocation threshold
assert_max_allocs!(2, {
    let b1 = Box::new(1);
    let b2 = Box::new(2);
});

// Assert a maximum byte limit
assert_max_bytes!(1024, {
    vec![0u8; 512]
});
```

## Ignoring Allocations (e.g. Logging / Setup)

Sometimes you want to format strings, print debug output, or run setup logic inside a profiled section without polluting your allocation counts. You can use the `alloc_ignore!` macro:

```rust
use alloc_count::{alloc_count, alloc_ignore};

let (stats, result) = alloc_count!({
    let v = vec![1, 2, 3];

    // Single expressions or logging:
    alloc_ignore!(println!("Debugging output: {v:?}"));

    // Or multi-line blocks:
    alloc_ignore!({
        let _setup = format!("temporary setup string {}", 42);
    });

    v
});

assert_eq!(stats.alloc_calls, 1);
```

You can also use it namespaced directly as `alloc_count::alloc_ignore!(...)` without importing it.

## Inspecting `AllocStats`

`AllocStats` implements `Display` and provides helper methods for examining memory behavior:

```rust
println!("{stats}");
// "AllocStats { allocs: 2, deallocs: 1, reallocs: 0, allocated: 64 B, deallocated: 32 B, net: 32 B }"

if stats.is_zero() {
    println!("No allocations or deallocations occurred!");
}

println!("Net memory change: {} bytes", stats.net_bytes());
println!("Net calls: {}", stats.net_calls());
```

## Multi-Threaded & Parallel Workloads (Rayon, Thread Pools)

By default, `alloc_count!` and `AllocSnapshot` track allocations on the **current thread only**. This gives exact, noise-free counts in multi-threaded environments.

If you want to track allocations across all threads (for example, parallel pipelines using [Rayon](https://crates.io/crates/rayon), background workers, or `std::thread::spawn`), use `alloc_count_global!` or `GlobalAllocSnapshot`:

```rust
use alloc_count::alloc_count_global;
use rayon::prelude::*;

let (stats, squares) = alloc_count_global!({
    (0..1_000)
        .into_par_iter()
        .map(|i| vec![i * i])
        .collect::<Vec<_>>()
});

println!("Allocations across all Rayon threads: {}", stats.alloc_calls);
```

Or using `GlobalAllocSnapshot`:

```rust
use alloc_count::GlobalAllocSnapshot;

let snapshot = GlobalAllocSnapshot::now();

// Work across threads...
let handle = std::thread::spawn(|| vec![1, 2, 3]);
handle.join().unwrap();

let stats = snapshot.elapsed();
println!("Total global allocations: {}", stats.alloc_calls);
```

> **Testing Tip for Global Allocations:**
> Because global tracking captures allocations across the entire process, concurrent unit tests running in parallel during `cargo test` could pollute each other's global counts.
>
> When writing unit tests that assert on `alloc_count_global!` or `GlobalAllocSnapshot`, use `#[serial]` from the [`serial_test`](https://crates.io/crates/serial_test) crate to ensure they run sequentially without interference:
>
> ```rust
> #[test]
> #[serial_test::serial]
> fn test_parallel_pipeline() {
>     let (stats, _) = alloc_count_global!({
>         // parallel code here
>     });
>     assert!(stats.alloc_calls > 0);
> }
> ```
> Alternatively, execute tests with `cargo test -- --test-threads=1`.

## Async (Tokio) Support

If you need to trace memory allocations across `.await` points and multiple worker threads, you can enable the `tokio` feature flag:

```toml
[dependencies]
alloc_count = { version = "0.4", features = ["tokio"] }
```

Then, use the specialized async tracking macro!

```rust
use std::alloc::System;
use alloc_count::{alloc_count_tokio, AllocCounter};

#[global_allocator]
static GLOBAL: AllocCounter<System> = AllocCounter(System);

#[tokio::main]
async fn main() {
    let (stats, result) = alloc_count_tokio!(async {
        let mut vec = Vec::new();
        
        // Even if tokio puts this task to sleep and wakes it up on a completely different 
        // OS worker thread, `alloc_count` effortlessly maps the allocations!
        tokio::task::yield_now().await; 
        
        for i in 0..10 {
            vec.push(i);
        }
        vec
    }).await;

    println!("Async Task ran! Result vector length: {}", result.len());
    println!("Allocations bound across await points: {}", stats.alloc_calls);
    println!("Total Bytes Requested: {}", stats.bytes_allocated);
}
```
