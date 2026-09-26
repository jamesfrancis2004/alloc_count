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

/// Measures the total allocation statistics that occurred across ALL threads in the process
/// during the execution of the provided expression or block.
///
/// This is ideal for measuring parallel pipelines (e.g. Rayon iterators) or multi-threaded background workers.
///
/// Note: In multi-threaded test runners (like `cargo test`), concurrent tests running on other threads may
/// contribute to this measurement. When writing unit tests with `alloc_count_global!`, use the `serial_test`
/// crate (`#[serial]`) or run tests with `-- --test-threads=1` to prevent cross-test contamination.
///
/// # Example
/// ```rust
/// use std::alloc::System;
/// use alloc_count::{alloc_count_global, AllocCounter};
///
/// #[global_allocator]
/// static GLOBAL: AllocCounter<System> = AllocCounter(System);
///
/// let (stats, result) = alloc_count_global!({
///     let handle = std::thread::spawn(|| vec![1, 2, 3]);
///     handle.join().unwrap()
/// });
/// assert!(stats.alloc_calls >= 1);
/// assert_eq!(result, vec![1, 2, 3]);
/// ```
#[macro_export]
macro_rules! alloc_count_global {
    ($e:expr) => {{
        let __start = $crate::global_stats();
        let __res = $e;
        let __end = $crate::global_stats();
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
        $crate::__track_tokio($b)
    };
}

/// Asserts that zero memory allocations occurred during the execution of the
/// provided expression or block.
///
/// Returns the result of the evaluated expression. Panics with a formatted diff on failure.
///
/// # Example
/// ```rust
/// use std::alloc::System;
/// use alloc_count::{assert_no_alloc, AllocCounter};
///
/// #[global_allocator]
/// static GLOBAL: AllocCounter<System> = AllocCounter(System);
///
/// let val = assert_no_alloc!({
///     let x = 10;
///     let y = 20;
///     x + y
/// });
/// assert_eq!(val, 30);
/// ```
#[macro_export]
macro_rules! assert_no_alloc {
    ($e:expr) => {{
        let (stats, res) = $crate::alloc_count!($e);
        assert!(
            stats.alloc_calls == 0,
            "assertion failed: expected zero allocations, but {} allocation(s) occurred (total: {} bytes):\n{}",
            stats.alloc_calls,
            stats.bytes_allocated,
            stats
        );
        res
    }};
}

/// Asserts that no more than `$max` memory allocations occurred during the execution of the
/// provided expression or block.
///
/// Returns a tuple `(AllocStats, result)`.
///
/// # Example
/// ```rust
/// use std::alloc::System;
/// use alloc_count::{assert_max_allocs, AllocCounter};
///
/// #[global_allocator]
/// static GLOBAL: AllocCounter<System> = AllocCounter(System);
///
/// let (stats, res) = assert_max_allocs!(1, {
///     Box::new(42)
/// });
/// assert_eq!(*res, 42);
/// ```
#[macro_export]
macro_rules! assert_max_allocs {
    ($max:expr, $e:expr) => {{
        let (stats, res) = $crate::alloc_count!($e);
        let max_val: usize = $max;
        assert!(
            stats.alloc_calls <= max_val,
            "assertion failed: expected at most {} allocation(s), but {} occurred:\n{}",
            max_val,
            stats.alloc_calls,
            stats
        );
        (stats, res)
    }};
}

/// Asserts that no more than `$max` bytes were allocated during the execution of the
/// provided expression or block.
///
/// Returns a tuple `(AllocStats, result)`.
///
/// # Example
/// ```rust
/// use std::alloc::System;
/// use alloc_count::{assert_max_bytes, AllocCounter};
///
/// #[global_allocator]
/// static GLOBAL: AllocCounter<System> = AllocCounter(System);
///
/// let (stats, res) = assert_max_bytes!(64, {
///     Box::new(42i64)
/// });
/// assert_eq!(*res, 42);
/// ```
#[macro_export]
macro_rules! assert_max_bytes {
    ($max:expr, $e:expr) => {{
        let (stats, res) = $crate::alloc_count!($e);
        let max_val: usize = $max;
        assert!(
            stats.bytes_allocated <= max_val,
            "assertion failed: expected at most {} byte(s) allocated, but {} bytes were allocated:\n{}",
            max_val,
            stats.bytes_allocated,
            stats
        );
        (stats, res)
    }};
}

/// Asserts that zero memory allocations occurred during the execution of the
/// provided asynchronous expression or block.
///
/// Note: Only available when the `tokio` feature is enabled.
///
/// # Example
/// ```rust
/// use std::alloc::System;
/// use alloc_count::{assert_no_alloc_tokio, AllocCounter};
///
/// #[global_allocator]
/// static GLOBAL: AllocCounter<System> = AllocCounter(System);
///
/// #[tokio::main]
/// async fn main() {
///     let val = assert_no_alloc_tokio!(async {
///         tokio::task::yield_now().await;
///         10 + 20
///     }).await;
///     assert_eq!(val, 30);
/// }
/// ```
#[cfg(feature = "tokio")]
#[macro_export]
macro_rules! assert_no_alloc_tokio {
    ($e:expr) => {
        async move {
            let (stats, res) = $crate::alloc_count_tokio!($e).await;
            assert!(
                stats.alloc_calls == 0,
                "assertion failed: expected zero async allocations, but {} allocation(s) occurred (total: {} bytes):\n{}",
                stats.alloc_calls,
                stats.bytes_allocated,
                stats
            );
            res
        }
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



