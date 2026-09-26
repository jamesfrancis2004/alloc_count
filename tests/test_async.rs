#![cfg(feature = "tokio")]
#![allow(clippy::useless_vec, clippy::vec_init_then_push)]

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_async_ignore() {
    use alloc_count::alloc_ignore;

    let (stats, res) = alloc_count_tokio!(async {
        let v = vec![1, 2, 3];
        tokio::task::yield_now().await;
        let ignored_val = alloc_ignore!({
            let mut s = Vec::new();
            for i in 0..100 {
                s.push(i);
            }
            42
        });
        (v, ignored_val)
    }).await;

    // Only vec![1, 2, 3] was tracked
    assert_eq!(stats.alloc_calls, 1);
    assert_eq!(res.0, vec![1, 2, 3]);
    assert_eq!(res.1, 42);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_async_nested_scopes() {
    let (outer_stats, (inner_stats, res)) = alloc_count_tokio!(async {
        let _b1 = Box::new(10);
        tokio::task::yield_now().await;

        let inner = alloc_count_tokio!(async {
            let _b2 = Box::new(20);
            tokio::task::yield_now().await;
            let _b3 = Box::new(30);
            42
        }).await;

        let _b4 = Box::new(40);
        inner
    }).await;

    // Inner made 2 allocations (b2 and b3)
    assert_eq!(inner_stats.alloc_calls, 2);
    assert_eq!(res, 42);

    // Outer should see all 4 allocations (b1, b2, b3, b4)
    assert_eq!(outer_stats.alloc_calls, 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_async_assert_no_alloc() {
    use alloc_count::assert_no_alloc_tokio;

    let res = assert_no_alloc_tokio!(async {
        tokio::task::yield_now().await;
        10 + 20
    }).await;

    assert_eq!(res, 30);
}

