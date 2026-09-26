/// Measures the total allocation statistics that occurred exclusively on the calling thread
/// during the execution of the provided expression or block.
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
    ($e:expr) => {{
        let __start = $crate::stats();
        let __res = $e;
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

/// Temporarily ignores any memory allocations, deallocations, or reallocations
/// performed during the execution of the provided expression or block.
///
/// Any memory operations within the expression will not be recorded in either
/// thread-local or task-local allocation statistics.
///
/// # Example
/// ```rust
/// use std::alloc::System;
/// use alloc_count::{alloc_count, alloc_ignore, AllocCounter};
///
/// #[global_allocator]
/// static GLOBAL: AllocCounter<System> = AllocCounter(System);
///
/// let (stats, _) = alloc_count!({
///     let _v = vec![1, 2, 3];
///     // Can be used on single expressions:
///     alloc_ignore!(println!("Debugging: {:?}", _v));
///     // Or on blocks:
///     alloc_ignore!({
///         let _s = format!("hello {}", 42);
///     });
/// });
/// assert_eq!(stats.alloc_calls, 1);
/// ```
#[macro_export]
macro_rules! alloc_ignore {
    ($e:expr) => {{
        let _guard = $crate::IgnoreGuard::new();
        $e
    }};
}



