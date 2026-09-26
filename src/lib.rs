use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

mod macros;

static GLOBAL_ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_DEALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_REALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_BYTES_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_BYTES_DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_BYTES_REALLOCATED: AtomicUsize = AtomicUsize::new(0);

/// Detailed statistics about allocations performed on the current thread.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AllocStats {
    pub alloc_calls: usize,
    pub dealloc_calls: usize,
    pub realloc_calls: usize,
    pub bytes_allocated: usize,
    pub bytes_deallocated: usize,
    pub bytes_reallocated: usize,
}

impl AllocStats {
    pub const fn new() -> Self {
        AllocStats {
            alloc_calls: 0,
            dealloc_calls: 0,
            realloc_calls: 0,
            bytes_allocated: 0,
            bytes_deallocated: 0,
            bytes_reallocated: 0,
        }
    }

    /// Computes the net memory change in bytes (allocated minus deallocated).
    /// Can be negative if more memory was deallocated than allocated in the measured scope.
    #[inline]
    pub fn net_bytes(&self) -> isize {
        self.bytes_allocated as isize - self.bytes_deallocated as isize
    }

    /// Computes the net number of allocation calls (allocations minus deallocations).
    #[inline]
    pub fn net_calls(&self) -> isize {
        self.alloc_calls as isize - self.dealloc_calls as isize
    }

    /// Returns `true` if zero allocations, deallocations, and reallocations occurred.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.alloc_calls == 0 && self.dealloc_calls == 0 && self.realloc_calls == 0
    }

    /// Subtracts the values of another `AllocStats` from this one, saturating at zero.
    /// Useful for computing the difference before and after a scope.
    pub fn saturating_sub(self, other: Self) -> Self {
        Self {
            alloc_calls: self.alloc_calls.saturating_sub(other.alloc_calls),
            dealloc_calls: self.dealloc_calls.saturating_sub(other.dealloc_calls),
            realloc_calls: self.realloc_calls.saturating_sub(other.realloc_calls),
            bytes_allocated: self.bytes_allocated.saturating_sub(other.bytes_allocated),
            bytes_deallocated: self.bytes_deallocated.saturating_sub(other.bytes_deallocated),
            bytes_reallocated: self.bytes_reallocated.saturating_sub(other.bytes_reallocated),
        }
    }
}

impl std::fmt::Display for AllocStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AllocStats {{ allocs: {}, deallocs: {}, reallocs: {}, allocated: {} B, deallocated: {} B, net: {} B }}",
            self.alloc_calls,
            self.dealloc_calls,
            self.realloc_calls,
            self.bytes_allocated,
            self.bytes_deallocated,
            self.net_bytes()
        )
    }
}

std::thread_local! {
    static ALLOC_STATS: Cell<AllocStats> = const { Cell::new(AllocStats::new()) };
    static IN_ALLOC: Cell<usize> = const { Cell::new(0) };
}

#[cfg(feature = "tokio")]
tokio::task_local! {
    #[doc(hidden)]
    pub static ASYNC_ALLOC_STATS: Cell<AllocStats>;
}

#[inline]
fn record_alloc_event(
    alloc_calls: usize,
    dealloc_calls: usize,
    realloc_calls: usize,
    bytes_allocated: usize,
    bytes_deallocated: usize,
    bytes_reallocated: usize,
) {
    IN_ALLOC.with(|in_alloc| {
        let count = in_alloc.get();
        if count == 0 {
            in_alloc.set(1);
            ALLOC_STATS.with(|stats| {
                let mut current = stats.get();
                current.alloc_calls += alloc_calls;
                current.dealloc_calls += dealloc_calls;
                current.realloc_calls += realloc_calls;
                current.bytes_allocated += bytes_allocated;
                current.bytes_deallocated += bytes_deallocated;
                current.bytes_reallocated += bytes_reallocated;
                stats.set(current);
            });

            #[cfg(feature = "tokio")]
            let _ = ASYNC_ALLOC_STATS.try_with(|stats| {
                let mut current = stats.get();
                current.alloc_calls += alloc_calls;
                current.dealloc_calls += dealloc_calls;
                current.realloc_calls += realloc_calls;
                current.bytes_allocated += bytes_allocated;
                current.bytes_deallocated += bytes_deallocated;
                current.bytes_reallocated += bytes_reallocated;
                stats.set(current);
            });

            if alloc_calls > 0 {
                GLOBAL_ALLOC_CALLS.fetch_add(alloc_calls, Relaxed);
            }
            if dealloc_calls > 0 {
                GLOBAL_DEALLOC_CALLS.fetch_add(dealloc_calls, Relaxed);
            }
            if realloc_calls > 0 {
                GLOBAL_REALLOC_CALLS.fetch_add(realloc_calls, Relaxed);
            }
            if bytes_allocated > 0 {
                GLOBAL_BYTES_ALLOCATED.fetch_add(bytes_allocated, Relaxed);
            }
            if bytes_deallocated > 0 {
                GLOBAL_BYTES_DEALLOCATED.fetch_add(bytes_deallocated, Relaxed);
            }
            if bytes_reallocated > 0 {
                GLOBAL_BYTES_REALLOCATED.fetch_add(bytes_reallocated, Relaxed);
            }

            in_alloc.set(0);
        }
    });
}

#[doc(hidden)]
pub struct IgnoreGuard;

impl IgnoreGuard {
    #[inline]
    pub fn new() -> Self {
        IN_ALLOC.with(|in_alloc| {
            in_alloc.set(in_alloc.get() + 1);
        });
        IgnoreGuard
    }
}

impl Default for IgnoreGuard {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for IgnoreGuard {
    #[inline]
    fn drop(&mut self) {
        IN_ALLOC.with(|in_alloc| {
            let val = in_alloc.get();
            if val > 0 {
                in_alloc.set(val - 1);
            }
        });
    }
}

/// A generic, thread-local allocation counter wrapper around a standard `GlobalAlloc`.
/// It intercepts allocation calls, tallies them per-thread, and forwards them to the underlying allocator.
pub struct AllocCounter<A = std::alloc::System>(pub A);

unsafe impl<A> GlobalAlloc for AllocCounter<A> where A: GlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_alloc_event(1, 0, 0, layout.size(), 0, 0);
        self.0.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record_alloc_event(0, 1, 0, 0, layout.size(), 0);
        self.0.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_alloc_event(1, 0, 0, layout.size(), 0, 0);
        self.0.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record_alloc_event(0, 0, 1, new_size, layout.size(), new_size);
        self.0.realloc(ptr, layout, new_size)
    }
}

/// Returns the current thread's total recorded memory statistics.
pub fn stats() -> AllocStats {
    ALLOC_STATS.with(|s| s.get())
}

/// Returns the process-wide total recorded memory statistics across all threads.
pub fn global_stats() -> AllocStats {
    AllocStats {
        alloc_calls: GLOBAL_ALLOC_CALLS.load(Relaxed),
        dealloc_calls: GLOBAL_DEALLOC_CALLS.load(Relaxed),
        realloc_calls: GLOBAL_REALLOC_CALLS.load(Relaxed),
        bytes_allocated: GLOBAL_BYTES_ALLOCATED.load(Relaxed),
        bytes_deallocated: GLOBAL_BYTES_DEALLOCATED.load(Relaxed),
        bytes_reallocated: GLOBAL_BYTES_REALLOCATED.load(Relaxed),
    }
}

/// Resets the calling thread's total recorded memory statistics back to zero.
///
/// # Example
/// ```rust
/// use std::alloc::System;
/// use alloc_count::{stats, reset, AllocCounter};
///
/// #[global_allocator]
/// static GLOBAL: AllocCounter<System> = AllocCounter(System);
///
/// let _vec = vec![1, 2, 3];
/// assert!(stats().alloc_calls > 0);
///
/// reset();
/// assert_eq!(stats().alloc_calls, 0);
/// ```
#[inline]
pub fn reset() {
    ALLOC_STATS.with(|s| s.set(AllocStats::new()));
}

/// Resets the process-wide global memory statistics back to zero across all threads.
pub fn reset_global() {
    GLOBAL_ALLOC_CALLS.store(0, Relaxed);
    GLOBAL_DEALLOC_CALLS.store(0, Relaxed);
    GLOBAL_REALLOC_CALLS.store(0, Relaxed);
    GLOBAL_BYTES_ALLOCATED.store(0, Relaxed);
    GLOBAL_BYTES_DEALLOCATED.store(0, Relaxed);
    GLOBAL_BYTES_REALLOCATED.store(0, Relaxed);
}

/// A snapshot of the current thread's memory statistics at a specific moment in time.
///
/// Similar in concept to [`std::time::Instant`], allowing you to query the allocation
/// difference since the snapshot was created using [`.elapsed()`](AllocSnapshot::elapsed).
///
/// # Example
/// ```rust
/// use std::alloc::System;
/// use alloc_count::{AllocCounter, AllocSnapshot};
///
/// #[global_allocator]
/// static GLOBAL: AllocCounter<System> = AllocCounter(System);
///
/// let snapshot = AllocSnapshot::now();
/// let _vec = vec![1, 2, 3];
/// let stats = snapshot.elapsed();
/// assert_eq!(stats.alloc_calls, 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocSnapshot {
    start: AllocStats,
}

impl AllocSnapshot {
    /// Takes a snapshot of the current thread's allocation statistics.
    #[inline]
    pub fn now() -> Self {
        Self {
            start: stats(),
        }
    }

    /// Calculates the allocation statistics that occurred since this snapshot was taken.
    #[inline]
    pub fn elapsed(&self) -> AllocStats {
        stats().saturating_sub(self.start)
    }
}

/// A snapshot of the process-wide memory statistics across all threads at a specific moment in time.
///
/// Similar to [`AllocSnapshot`], but captures allocations across all threads (such as Rayon worker pools
/// or `std::thread::spawn`).
///
/// # Example
/// ```rust
/// use std::alloc::System;
/// use alloc_count::{AllocCounter, GlobalAllocSnapshot};
///
/// #[global_allocator]
/// static GLOBAL: AllocCounter<System> = AllocCounter(System);
///
/// let snapshot = GlobalAllocSnapshot::now();
/// let handle = std::thread::spawn(|| vec![1, 2, 3]);
/// handle.join().unwrap();
///
/// let stats = snapshot.elapsed();
/// assert!(stats.alloc_calls >= 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalAllocSnapshot {
    start: AllocStats,
}

impl GlobalAllocSnapshot {
    /// Takes a snapshot of the process-wide allocation statistics across all threads.
    #[inline]
    pub fn now() -> Self {
        Self {
            start: global_stats(),
        }
    }

    /// Calculates the allocation statistics that occurred across all threads since this snapshot was taken.
    #[inline]
    pub fn elapsed(&self) -> AllocStats {
        global_stats().saturating_sub(self.start)
    }
}

#[cfg(feature = "tokio")]
#[doc(hidden)]
pub async fn __track_tokio<F, R>(fut: F) -> (AllocStats, R)
where
    F: std::future::Future<Output = R>,
{
    let in_scope = ASYNC_ALLOC_STATS.try_with(|_| ()).is_ok();
    if in_scope {
        let start = ASYNC_ALLOC_STATS.with(|s| s.get());
        let res = fut.await;
        let end = ASYNC_ALLOC_STATS.with(|s| s.get());
        (end.saturating_sub(start), res)
    } else {
        ASYNC_ALLOC_STATS
            .scope(
                std::cell::Cell::new(AllocStats::new()),
                async move {
                    let start = ASYNC_ALLOC_STATS.with(|s| s.get());
                    let res = fut.await;
                    let end = ASYNC_ALLOC_STATS.with(|s| s.get());
                    (end.saturating_sub(start), res)
                },
            )
            .await
    }
}
