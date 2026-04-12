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

/// Measures the total allocation statistics that occur within the provided `async` block.
/// This macro dynamically hooks into the `tokio` runtime to track allocations across
/// `.await` points and spawned thread-bounces perfectly!
///
/// Note: This is only available when the `tokio` feature is enabled.
///
/// # Example
/// ```rust
/// // Assuming global allocator is already set
/// use alloc_count::alloc_count_tokio;
/// 
/// #[tokio::main]
/// async fn main() {
///     let (stats, result) = alloc_count_tokio!(async {
///         // Await an asynchronous operation
///         tokio::task::yield_now().await;
///         let b = Box::new(42);
///         *b
///     }).await;
///     
///     println!("Async task allocations: {}", stats.alloc_calls);
/// }
/// ```
#[cfg(feature = "tokio")]
#[macro_export]
macro_rules! alloc_count_tokio {
    ($b:expr) => {
        $crate::ASYNC_ALLOC_STATS.scope(
            std::cell::Cell::new($crate::AllocStats::new()),
            async move {
                let result = $b.await;
                let stats = $crate::ASYNC_ALLOC_STATS.with(|s| s.get());
                (stats, result)
            }
        )
    };
}
