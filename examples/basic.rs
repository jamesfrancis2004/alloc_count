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

    let (count, _) = alloc_count!({
        // String allocations
        let _s = String::from("hello world!");
        
        // Box allocations
        let _b = Box::new(42);
    });

    println!("String & Box: {} allocations", count);
}
