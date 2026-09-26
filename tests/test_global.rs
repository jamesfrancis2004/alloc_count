#![allow(clippy::useless_vec, clippy::vec_init_then_push, clippy::let_unit_value)]

use std::alloc::System;
use alloc_count::AllocCounter;

#[global_allocator]
static GLOBAL: AllocCounter<System> = AllocCounter(System);

#[test]
#[serial_test::serial]
fn test_alloc_count_global_exact_accuracy() {
    use alloc_count::alloc_count_global;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let start_flag = Arc::new(AtomicBool::new(false));
    let ready_flag = Arc::new(AtomicBool::new(false));
    let done_flag = Arc::new(AtomicBool::new(false));

    let s = Arc::clone(&start_flag);
    let r = Arc::clone(&ready_flag);
    let d = Arc::clone(&done_flag);

    let t = std::thread::spawn(move || {
        // Signal that thread startup and TLS setup are finished
        r.store(true, Ordering::Release);
        while !s.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }

        // Worker allocates exactly 2 Box<u32> = 8 bytes
        let x = Box::new(10u32);
        let y = Box::new(20u32);

        // Signal that allocations are done
        d.store(true, Ordering::Release);

        // Keep them alive until main thread finishes measuring
        while s.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
        drop(x);
        drop(y);
    });

    // Wait until worker thread is ready and spinning
    while !ready_flag.load(Ordering::Acquire) {
        std::hint::spin_loop();
    }

    let m_holder;
    let (stats, ()) = alloc_count_global!({
        // Release worker thread NOW, strictly inside alloc_count_global!
        start_flag.store(true, Ordering::Release);

        // Main thread allocates exactly 1 Box<u32> = 4 bytes
        m_holder = Some(Box::new(30u32));

        // Wait for worker thread to perform its allocations
        while !done_flag.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
    });

    // Tell worker thread it can drop its allocations now
    start_flag.store(false, Ordering::Release);
    t.join().unwrap();
    drop(m_holder);

    // Exactly 2 on worker + 1 on main = 3 alloc calls, exactly 12 bytes!
    assert_eq!(stats.alloc_calls, 3);
    assert_eq!(stats.bytes_allocated, 3 * std::mem::size_of::<u32>());
}

#[test]
#[serial_test::serial]
fn test_global_alloc_snapshot_exact_multi_thread_accuracy() {
    use alloc_count::GlobalAllocSnapshot;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let start_flag = Arc::new(AtomicBool::new(false));
    let t1_ready = Arc::new(AtomicBool::new(false));
    let t2_ready = Arc::new(AtomicBool::new(false));
    let t1_done = Arc::new(AtomicBool::new(false));
    let t2_done = Arc::new(AtomicBool::new(false));

    let s1 = Arc::clone(&start_flag);
    let r1 = Arc::clone(&t1_ready);
    let d1 = Arc::clone(&t1_done);
    let t1 = std::thread::spawn(move || {
        r1.store(true, Ordering::Release);
        while !s1.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
        // Worker 1 allocates exactly 1 Box<u64> = 8 bytes
        let b = Box::new(100u64);
        d1.store(true, Ordering::Release);
        while s1.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
        drop(b);
    });

    let s2 = Arc::clone(&start_flag);
    let r2 = Arc::clone(&t2_ready);
    let d2 = Arc::clone(&t2_done);
    let t2 = std::thread::spawn(move || {
        r2.store(true, Ordering::Release);
        while !s2.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
        // Worker 2 allocates exactly 2 Box<u64> = 16 bytes
        let b_a = Box::new(200u64);
        let b_b = Box::new(300u64);
        d2.store(true, Ordering::Release);
        while s2.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
        drop(b_a);
        drop(b_b);
    });

    // Wait until both worker threads are fully spawned and spinning
    while !t1_ready.load(Ordering::Acquire) || !t2_ready.load(Ordering::Acquire) {
        std::hint::spin_loop();
    }

    // Take snapshot BEFORE releasing worker threads
    let snap = GlobalAllocSnapshot::now();
    start_flag.store(true, Ordering::Release);

    // Main thread allocates exactly 3 Box<u64> = 24 bytes
    let m1 = Box::new(1u64);
    let m2 = Box::new(2u64);
    let m3 = Box::new(3u64);

    // Wait for worker threads to complete their allocations
    while !t1_done.load(Ordering::Acquire) || !t2_done.load(Ordering::Acquire) {
        std::hint::spin_loop();
    }

    // Snapshot before any boxes are dropped
    let stats_before_drop = snap.elapsed();

    // Thread 1 (1 alloc, 8 B) + Thread 2 (2 allocs, 16 B) + Main (3 allocs, 24 B) = exactly 6 allocs, 48 B!
    assert_eq!(stats_before_drop.alloc_calls, 6);
    assert_eq!(stats_before_drop.bytes_allocated, 6 * std::mem::size_of::<u64>());

    // Release workers to drop and join
    start_flag.store(false, Ordering::Release);
    t1.join().unwrap();
    t2.join().unwrap();
    drop(m1);
    drop(m2);
    drop(m3);
}

#[test]
#[serial_test::serial]
fn test_alloc_count_global_rayon() {
    use alloc_count::alloc_count_global;
    use rayon::prelude::*;

    let (stats, results) = alloc_count_global!({
        (0..100)
            .into_par_iter()
            .map(|i| vec![i; 10])
            .collect::<Vec<_>>()
    });

    assert_eq!(results.len(), 100);
    // Rayon parallel iterator spawned allocations across worker threads
    assert!(stats.alloc_calls >= 100);
    assert!(stats.bytes_allocated >= 100 * 10 * std::mem::size_of::<i32>());
}

#[test]
#[serial_test::serial]
fn test_global_alloc_snapshot_rayon() {
    use alloc_count::GlobalAllocSnapshot;
    use rayon::prelude::*;

    let snapshot = GlobalAllocSnapshot::now();

    let items: Vec<Box<i32>> = (0..50)
        .into_par_iter()
        .map(Box::new)
        .collect();

    let stats = snapshot.elapsed();
    assert_eq!(items.len(), 50);
    assert!(stats.alloc_calls >= 50);
    assert!(stats.bytes_allocated >= 50 * std::mem::size_of::<i32>());
}

#[test]
#[serial_test::serial]
fn test_nested_local_and_global_alloc_count() {
    use alloc_count::{alloc_count, alloc_count_global};
    use rayon::prelude::*;

    let (global_stats, (local_stats, ())) = alloc_count_global!({
        // Thread-local allocation on caller thread
        let local_res = alloc_count!({
            let _caller_vec = vec![1, 2, 3];
        });

        // Parallel allocation on Rayon threads
        (0..20).into_par_iter().for_each(|_| {
            let _par_vec = vec![4, 5, 6];
        });

        local_res
    });

    // Local stats should only see caller thread allocations (1 vector)
    assert_eq!(local_stats.alloc_calls, 1);
    // Global stats should see both caller thread and worker threads
    assert!(global_stats.alloc_calls >= 21);
}

#[test]
#[serial_test::serial]
fn test_reset_global_stats() {
    use alloc_count::{global_stats, reset_global};

    let _v = vec![1, 2, 3];
    assert!(global_stats().alloc_calls > 0);

    reset_global();
    assert_eq!(global_stats().alloc_calls, 0);
    assert_eq!(global_stats().bytes_allocated, 0);
}
