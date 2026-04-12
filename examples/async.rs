use std::alloc::System;
use alloc_count::{alloc_count_tokio, AllocCounter};

#[global_allocator]
static GLOBAL: AllocCounter<System> = AllocCounter(System);

#[tokio::main]
async fn main() {
    let (stats, result) = alloc_count_tokio!(async {
        let mut vec = Vec::new();
        for i in 0..10 {
            tokio::task::yield_now().await; // Simulate suspension and thread bouncing!
            vec.push(i);
        }
        vec
    }).await;

    println!("Async Task ran! Result vector length: {}", result.len());
    println!("Allocations bound across await points: {}", stats.alloc_calls);
    println!("Total Bytes Requested: {}", stats.bytes_allocated);
}
