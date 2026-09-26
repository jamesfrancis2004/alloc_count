#![allow(clippy::useless_vec, clippy::vec_init_then_push, clippy::let_unit_value)]

use std::alloc::System;
use alloc_count::{alloc_count, AllocCounter};

#[global_allocator]
static GLOBAL: AllocCounter<System> = AllocCounter(System);

#[test]
fn test_single_allocation() {
    let (stats, res) = alloc_count!({
        let b = Box::new(42i64);
        *b
    });
    // We expect exactly 1 allocation for the Box.
    assert_eq!(stats.alloc_calls, 1);
    assert_eq!(stats.bytes_allocated, 8); // 42i64 is 8 bytes
    assert_eq!(res, 42);
}

#[test]
fn test_multiple_allocations() {
    let (stats, res) = alloc_count!({
        let mut v = Vec::new();
        // A Vec without initial capacity allocates on the first push
        v.push(1i32);
        v
    });
    
    // Pushing one element triggers an initial allocation.
    assert_eq!(stats.alloc_calls, 1);
    assert!(stats.bytes_allocated >= 4); // At least 4 bytes for one i32
    assert_eq!(res, vec![1]);
}

#[test]
fn test_zero_allocations() {
    let (stats, res) = alloc_count!({
        let x = 10;
        let y = 20;
        x + y
    });
    
    // Stack and arithmetic operations shouldn't allocate.
    assert_eq!(stats.alloc_calls, 0);
    assert_eq!(stats.bytes_allocated, 0);
    assert_eq!(res, 30);
}

#[test]
fn test_nested_alloc_counts() {
    let (outer_stats, (inner_stats, inner_res)) = alloc_count!({
        let _b = Box::new(1i64);
        
        let inner = alloc_count!({
            let mut v = Vec::new();
            v.push(2i64);
            v
        });
        
        let _b2 = Box::new(3i64);
        inner
    });
    
    // Total allocations in the outer scope:
    // Box 1 (1) + Vec inner (1) + Box 2 (1) = 3
    assert_eq!(outer_stats.alloc_calls, 3);
    assert_eq!(inner_stats.alloc_calls, 1);
    assert_eq!(inner_res, vec![2]);
}

#[test]
fn test_alloc_count_expression() {
    // Tests that alloc_count! works directly with expressions (not just blocks)
    let (stats, res) = alloc_count!(Box::new(99i64));
    assert_eq!(stats.alloc_calls, 1);
    assert_eq!(*res, 99);
}

#[test]
fn test_alloc_ignore_macro() {
    use alloc_count::alloc_ignore;

    let (stats, (res1, res2)) = alloc_count!({
        let val1 = Box::new(10);
        let val2 = alloc_ignore!({
            // These allocations should be ignored completely
            let mut v = Vec::new();
            for i in 0..100 {
                v.push(i);
            }
            let _s = format!("ignored allocation string {}", 123);
            Box::new(20)
        });
        (*val1, *val2)
    });

    // Only val1 allocation was tracked
    assert_eq!(stats.alloc_calls, 1);
    assert_eq!(res1, 10);
    assert_eq!(res2, 20);
}

#[test]
fn test_nested_alloc_ignore() {
    use alloc_count::alloc_ignore;

    let (stats, _) = alloc_count!({
        let _b1 = Box::new(1);
        alloc_ignore!({
            let _ignored1 = Box::new(2);
            alloc_ignore!({
                let _ignored2 = Box::new(3);
            });
            let _ignored3 = Box::new(4);
        });
        let _b2 = Box::new(5);
    });

    // Only b1 and b2 should be counted
    assert_eq!(stats.alloc_calls, 2);
}

#[test]
fn test_alloc_ignore_panic_safety() {
    use alloc_count::alloc_ignore;

    let _ = std::panic::catch_unwind(|| {
        alloc_ignore!({
            let _b = Box::new(1);
            panic!("intentional panic inside alloc_ignore");
        });
    });

    // After an unwound panic, tracking must be fully restored
    let (stats, _) = alloc_count!(Box::new(42));
    assert_eq!(stats.alloc_calls, 1);
}

#[test]
fn test_alloc_ignore_single_expression() {
    use alloc_count::alloc_ignore;

    let (stats, (res1, res2)) = alloc_count!({
        let val1 = Box::new(10);
        let val2 = alloc_ignore!(Box::new(20)); // Single expression macro!
        let _ = alloc_ignore!({                 // Block macro!
            let mut v = Vec::new();
            v.push(99);
        });
        (*val1, *val2)
    });

    assert_eq!(stats.alloc_calls, 1);
    assert_eq!(res1, 10);
    assert_eq!(res2, 20);
}

#[test]
fn test_alloc_ignore_namespaced() {
    // Calling alloc_count::alloc_ignore! directly without importing it
    let (stats, res) = alloc_count!({
        let v1 = Box::new(1);
        let v2 = alloc_count::alloc_ignore!(Box::new(2));
        (*v1, *v2)
    });

    assert_eq!(stats.alloc_calls, 1);
    assert_eq!(res, (1, 2));
}


#[test]
fn test_alloc_ignore_try_operator() {
    use alloc_count::alloc_ignore;

    fn helper() -> Result<i32, &'static str> {
        let (stats, res) = alloc_count!({
            let val = Box::new(10);
            // Verify ? operator works inside alloc_ignore! macro
            let ignored = alloc_ignore!({
                let b = Box::new(20);
                if *b != 20 {
                    return Err("unexpected");
                }
                *b
            });
            *val + ignored
        });
        assert_eq!(stats.alloc_calls, 1);
        Ok(res)
    }

    assert_eq!(helper(), Ok(30));
}

#[test]
fn test_realloc_accounting() {
    let (stats, _) = alloc_count!({
        let mut v = Vec::with_capacity(4); // initial alloc (4 * 8 bytes = 32 bytes)
        for i in 0..100 {
            v.push(i as u64); // triggers multiple reallocations!
        }
        // Vector is dropped at the end of this block!
    });

    // Reallocations occurred:
    assert!(stats.realloc_calls > 0);
    // bytes_allocated and bytes_deallocated must be equal because the vector was dropped!
    assert_eq!(stats.bytes_allocated, stats.bytes_deallocated);
    assert_eq!(stats.net_bytes(), 0);
}

#[test]
fn test_alloc_stats_methods_and_display() {
    let (stats, _) = alloc_count!({
        let b = Box::new(42);
        drop(b);
    });

    assert_eq!(stats.alloc_calls, 1);
    assert_eq!(stats.dealloc_calls, 1);
    assert_eq!(stats.net_calls(), 0);
    assert_eq!(stats.net_bytes(), 0);
    assert!(!stats.is_zero());

    let (empty_stats, _) = alloc_count!(1 + 2);
    assert!(empty_stats.is_zero());

    // Test Display implementation
    let display_str = format!("{stats}");
    assert!(display_str.contains("AllocStats {"));
    assert!(display_str.contains("allocs: 1"));
    assert!(display_str.contains("deallocs: 1"));
}

#[test]
fn test_alloc_snapshot_basic() {
    use alloc_count::AllocSnapshot;

    let snapshot = AllocSnapshot::now();
    let _v = vec![1, 2, 3];
    let stats = snapshot.elapsed();
    assert_eq!(stats.alloc_calls, 1);
    assert!(stats.bytes_allocated >= 3 * std::mem::size_of::<i32>());
}

#[test]
fn test_alloc_snapshot_multiple_elapsed() {
    use alloc_count::AllocSnapshot;

    let snapshot = AllocSnapshot::now();

    let _b1 = Box::new(1);
    let stats1 = snapshot.elapsed();
    assert_eq!(stats1.alloc_calls, 1);

    let _b2 = Box::new(2);
    let stats2 = snapshot.elapsed();
    assert_eq!(stats2.alloc_calls, 2);

    let _b3 = Box::new(3);
    let stats3 = snapshot.elapsed();
    assert_eq!(stats3.alloc_calls, 3);
}

#[test]
fn test_alloc_snapshot_overlapping() {
    use alloc_count::AllocSnapshot;

    let snap1 = AllocSnapshot::now();

    let _b1 = Box::new(10); // inside snap1

    let snap2 = AllocSnapshot::now();

    let _b2 = Box::new(20); // inside snap1 and snap2

    let stats2 = snap2.elapsed();
    let stats1 = snap1.elapsed();

    // snap2 only saw b2
    assert_eq!(stats2.alloc_calls, 1);
    // snap1 saw both b1 and b2
    assert_eq!(stats1.alloc_calls, 2);
}

#[test]
fn test_alloc_snapshot_across_functions() {
    use alloc_count::{AllocSnapshot, AllocStats};

    fn producer() -> (Vec<i32>, AllocSnapshot) {
        let snap = AllocSnapshot::now();
        let v = vec![1, 2, 3];
        (v, snap)
    }

    fn consumer(mut v: Vec<i32>, snap: AllocSnapshot) -> (usize, AllocStats) {
        v.push(4); // may trigger realloc or stay within capacity
        let _b = Box::new(99);
        (v.len(), snap.elapsed())
    }

    let (v, snap) = producer();
    let (len, stats) = consumer(v, snap);

    assert_eq!(len, 4);
    // producer made 1 alloc (vec), consumer made at least 1 alloc (box)
    assert!(stats.alloc_calls >= 2);
}

#[test]
fn test_alloc_snapshot_with_alloc_ignore() {
    use alloc_count::{alloc_ignore, AllocSnapshot};

    let snapshot = AllocSnapshot::now();

    let _b1 = Box::new(10);
    alloc_ignore!({
        let _ignored1 = Box::new(20);
        let _ignored2 = vec![1, 2, 3, 4, 5];
    });
    let _b2 = Box::new(30);

    let stats = snapshot.elapsed();
    // Only b1 and b2 should be recorded
    assert_eq!(stats.alloc_calls, 2);
}

#[test]
fn test_alloc_snapshot_thread_isolation() {
    use alloc_count::AllocSnapshot;
    use std::sync::Arc;
    use std::sync::Barrier;

    let barrier = Arc::new(Barrier::new(2));
    let barrier_clone = Arc::clone(&barrier);

    let handle = std::thread::spawn(move || {
        // Wait until main thread is ready to take snapshot
        barrier_clone.wait();

        // Background thread allocates heavily
        let snap_bg = AllocSnapshot::now();
        let _v: Vec<i32> = (0..10_000).collect();
        let bg_stats = snap_bg.elapsed();
        assert!(bg_stats.alloc_calls > 0);

        // Wait before exiting
        barrier_clone.wait();
    });

    // Synchronize and take snapshot on main thread
    barrier.wait();
    let snap_main = AllocSnapshot::now();

    // Wait for background thread to complete its allocations
    barrier.wait();

    // Main thread did not allocate anything, should see 0
    let main_stats = snap_main.elapsed();
    assert_eq!(main_stats.alloc_calls, 0);
    assert_eq!(main_stats.bytes_allocated, 0);
    assert!(main_stats.is_zero());

    handle.join().unwrap();
}

#[test]
fn test_alloc_snapshot_realloc_and_drop() {
    use alloc_count::AllocSnapshot;

    let snapshot = AllocSnapshot::now();
    {
        let mut v = Vec::with_capacity(2);
        for i in 0..50 {
            v.push(i);
        }
        // Vector drops here
    }
    let stats = snapshot.elapsed();

    assert!(stats.realloc_calls > 0);
    // Everything was dropped inside snapshot, so net bytes must be 0!
    assert_eq!(stats.bytes_allocated, stats.bytes_deallocated);
    assert_eq!(stats.net_bytes(), 0);
}

#[test]
fn test_reset_stats() {
    use alloc_count::{reset, stats};

    let _v = vec![1, 2, 3];
    assert!(stats().alloc_calls > 0);

    reset();
    assert_eq!(stats().alloc_calls, 0);
    assert_eq!(stats().bytes_allocated, 0);
}

#[test]
fn test_assert_no_alloc_macro() {
    use alloc_count::assert_no_alloc;

    // Passing case
    let val = assert_no_alloc!({
        let x = 10;
        let y = 20;
        x + y
    });
    assert_eq!(val, 30);

    // Failing case panics
    let result = std::panic::catch_unwind(|| {
        assert_no_alloc!({
            let _b = Box::new(42);
        });
    });
    assert!(result.is_err());
}

#[test]
fn test_assert_max_allocs_macro() {
    use alloc_count::assert_max_allocs;

    // Passing case
    let (stats, val) = assert_max_allocs!(2, {
        let b1 = Box::new(1);
        let b2 = Box::new(2);
        *b1 + *b2
    });
    assert_eq!(stats.alloc_calls, 2);
    assert_eq!(val, 3);

    // Failing case panics
    let result = std::panic::catch_unwind(|| {
        assert_max_allocs!(1, {
            let _b1 = Box::new(1);
            let _b2 = Box::new(2);
        });
    });
    assert!(result.is_err());
}

#[test]
fn test_assert_max_bytes_macro() {
    use alloc_count::assert_max_bytes;

    // Passing case
    let (stats, _) = assert_max_bytes!(1024, {
        let _b = Box::new(42i64);
    });
    assert!(stats.bytes_allocated <= 1024);

    // Failing case panics
    let result = std::panic::catch_unwind(|| {
        assert_max_bytes!(4, {
            let _b = Box::new([0u8; 100]); // 100 bytes > 4 bytes
        });
    });
    assert!(result.is_err());
}
