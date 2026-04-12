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
