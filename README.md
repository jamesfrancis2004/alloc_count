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
