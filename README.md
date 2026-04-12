# alloc_count

A lightweight Rust library to manually count allocations per scope using a configurable thread-local `GlobalAlloc`.

## Getting Started
It's as easy as dropping the allocator in your root crate config and calling the macro!

```rust
use alloc_count::{alloc_count, AllocCounter};

#[global_allocator]
static GLOBAL: AllocCounter = AllocCounter;

fn main() {
    let (count, result) = alloc_count!({
        let mut vec = Vec::new();
        // Vec allocations usually happen when capacity expands.
        for i in 0..100 {
            vec.push(i);
        }
        vec
    });

    println!("Loop push: {} allocations, vector size: {}", count, result.len());
}
```
