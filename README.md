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

## Async (Tokio) Support

If you need to trace memory allocations across `.await` points and multiple worker threads, you can enable the `tokio` feature flag:

```toml
[dependencies]
alloc_count = { version = "0.3", features = ["tokio"] }
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
