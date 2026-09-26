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


