use alloc_count::{alloc_count, AllocCounter};

#[global_allocator]
static GLOBAL: AllocCounter = AllocCounter;

#[test]
fn test_single_allocation() {
    let (count, res) = alloc_count!({
        let b = Box::new(42);
        *b
    });
    // We expect exactly 1 allocation for the Box.
    assert_eq!(count, 1);
    assert_eq!(res, 42);
}

#[test]
fn test_multiple_allocations() {
    let (count, res) = alloc_count!({
        let mut v = Vec::new();
        // A Vec without initial capacity allocates on the first push
        v.push(1);
        v
    });
    // Pushing one element triggers an initial allocation.
    assert_eq!(count, 1);
    assert_eq!(res, vec![1]);
}

#[test]
fn test_zero_allocations() {
    let (count, res) = alloc_count!({
        let x = 10;
        let y = 20;
        x + y
    });
    // Stack and arithmetic operations shouldn't allocate.
    assert_eq!(count, 0);
    assert_eq!(res, 30);
}

#[test]
fn test_nested_alloc_counts() {
    let (outer_count, (inner_count, inner_res)) = alloc_count!({
        let _b = Box::new(1);
        
        let inner = alloc_count!({
            let mut v = Vec::new();
            v.push(2);
            v
        });
        
        let _b2 = Box::new(3);
        inner
    });
    
    // Total allocations in the outer scope:
    // Box 1 (1) + Vec inner (1) + Box 2 (1) = 3
    assert_eq!(outer_count, 3);
    assert_eq!(inner_count, 1);
    assert_eq!(inner_res, vec![2]);
}
