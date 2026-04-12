/// Measures the total number of allocations that occurred exclusively on the calling thread
/// during the execution of the provided block.
///
/// Returns a tuple `(allocation_count, block_result)`, allowing the user to decide
/// whether to print, log, or programmatically react to the allocation count.
///
/// # Example
/// ```rust
/// // Assuming global allocator is already set
/// use alloc_count::alloc_count;
/// 
/// let (count, result) = alloc_count!({
///     let mut vec = Vec::new();
///     vec.push(1);
///     vec.push(2);
///     vec.push(3);
///     vec
/// });
/// 
/// println!("Made {} allocations to create a vec of length {}", count, result.len());
/// ```
#[macro_export]
macro_rules! alloc_count {
    ($b:block) => {{
        let __start = $crate::count();
        let __res = { $b };
        let __end = $crate::count();
        (__end.saturating_sub(__start), __res)
    }};
}
