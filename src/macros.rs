/// Measures the total allocation statistics that occurred exclusively on the calling thread
/// during the execution of the provided block.
///
/// Returns a tuple `(alloc_stats, block_result)`, allowing the user to deeply inspect
/// memory metrics (allocations, deallocations, bytes requested) dynamically.
///
/// # Example
/// ```rust
/// // Assuming global allocator is already set
/// use alloc_count::alloc_count;
/// 
/// let (stats, result) = alloc_count!({
///     let mut vec = Vec::new();
///     vec.push(1);
///     vec.push(2);
///     vec.push(3);
///     vec
/// });
/// 
/// println!("Made {} allocations requesting {} bytes minimum", stats.alloc_calls, stats.bytes_allocated);
/// ```
#[macro_export]
macro_rules! alloc_count {
    ($b:block) => {{
        let __start = $crate::stats();
        let __res = { $b };
        let __end = $crate::stats();
        (__end.saturating_sub(__start), __res)
    }};
}
