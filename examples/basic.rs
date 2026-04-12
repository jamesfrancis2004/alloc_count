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

    println!("Loop push: {} allocations, {} bytes. vector size: {}", stats.alloc_calls, stats.bytes_allocated, result.len());

    let (stats, _) = alloc_count!({
        // String allocations
        let _s = String::from("hello world!");
        
        // Box allocations
        let _b = Box::new(42);
    });

    println!("String & Box: {} allocations, {} bytes", stats.alloc_calls, stats.bytes_allocated);
}
