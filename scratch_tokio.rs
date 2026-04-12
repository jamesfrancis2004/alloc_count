use std::cell::Cell;
use std::alloc::{GlobalAlloc, System, Layout};

tokio::task_local! {
    static ASYNC_STATS: Cell<usize>;
}

struct TrackingAlloc;

unsafe impl GlobalAlloc for TrackingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Warning: try_with might allocate!
        // We MUST prevent recursion
        thread_local! {
            static IN_ALLOC: Cell<bool> = const { Cell::new(false) };
        }
        
        IN_ALLOC.with(|in_alloc| {
            if !in_alloc.get() {
                in_alloc.set(true);
                let _ = ASYNC_STATS.try_with(|s| {
                    s.set(s.get() + 1);
                });
                in_alloc.set(false);
            }
        });
        
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: TrackingAlloc = TrackingAlloc;

#[tokio::main]
async fn main() {
    ASYNC_STATS.scope(Cell::new(0), async {
        let mut v = Vec::new();
        v.push(1);
        let val = ASYNC_STATS.with(|s| s.get());
        println!("Allocations in async block: {}", val);
    }).await;
}
