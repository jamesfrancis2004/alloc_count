#![cfg(feature = "tokio")]

use std::alloc::System;
use alloc_count::{alloc_count_tokio, AllocCounter};

#[global_allocator]
static GLOBAL: AllocCounter<System> = AllocCounter(System);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_async_await_boundary() {
    let (stats, res) = alloc_count_tokio!(async {
        let mut v = Vec::new();
        // Force the task to sleep and likely wake up on a different worker thread
        tokio::task::yield_now().await;
        v.push(1);
        v
    }).await;
    
    // Memory profiling smoothly bridged the worker thread jump!
    assert_eq!(stats.alloc_calls, 1);
    assert_eq!(res, vec![1]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_async_spawning_detached() {
    let (stats, res) = alloc_count_tokio!(async {
        // tokio::spawn runs completely detached from the parent's task-local scope.
        let handle = tokio::spawn(async move {
            let mut v = Vec::new(); // allocation happens inside the raw detached task!
            v.push(2);
            v
        });
        handle.await.unwrap()
    }).await;
    
    // `tokio::spawn` allocates memory internally to build the task structure. 
    // The parent task *should* log the spawn structure allocations, but *not* the Vec allocations 
    // which happened freely outside of its task scope in the background.
    assert!(stats.alloc_calls >= 1);
    assert_eq!(res, vec![2]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_async_spawning_wrapped() {
    let (_, res) = alloc_count_tokio!(async {
        // If a developer wants to profile a spawned task, they can wrap it natively!
        let handle = tokio::spawn(async move {
            let (inner_stats, inner_res) = alloc_count_tokio!(async {
                let mut v = Vec::new();
                v.push(3);
                v
            }).await;
            (inner_stats, inner_res)
        });
        handle.await.unwrap()
    }).await;
    
    // The spawned task was flawlessly tracked dynamically in the background!
    assert_eq!(res.0.alloc_calls, 1);
    assert_eq!(res.1, vec![3]);
}
