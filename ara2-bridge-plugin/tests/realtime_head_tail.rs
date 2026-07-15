use ara2_bridge_core::{HeadTailEntry, RealtimeFailureCode};
use ara2_bridge_plugin::RealtimeHeadTailAdapter;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct TrackingAllocator;

thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

// SAFETY: every operation delegates unchanged to `System`; thread-local counters observe calls
// without changing allocation layout, ownership, or deallocation pairing.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TRACKING.with(|tracking| {
            if tracking.get() {
                ALLOCATIONS.with(|allocations| allocations.set(allocations.get() + 1));
            }
        });
        // SAFETY: this method forwards the allocator contract and unchanged layout to `System`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: this pair is forwarded unchanged to the allocator that created the allocation.
        unsafe { System.dealloc(pointer, layout) };
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

#[test]
fn head_tail_queries_use_only_the_installed_immutable_snapshot() {
    let adapter = RealtimeHeadTailAdapter::new(8).unwrap();
    adapter
        .install([
            HeadTailEntry::new(2, 0.2, 0.3).unwrap(),
            HeadTailEntry::new(1, 0.1, 0.4).unwrap(),
        ])
        .unwrap();
    assert_eq!(adapter.query(1), Some((0.1, 0.4)));
    assert_eq!(adapter.query(2), Some((0.2, 0.3)));
    assert_eq!(adapter.query(9), None);
    assert_eq!(
        adapter.pop_failure(),
        Some(RealtimeFailureCode::MissingRegion)
    );
}

#[test]
fn replacing_a_snapshot_keeps_prior_storage_owned_and_queries_lock_free() {
    let adapter = RealtimeHeadTailAdapter::new(4).unwrap();
    adapter
        .install([HeadTailEntry::new(1, 0.1, 0.2).unwrap()])
        .unwrap();
    adapter
        .install([HeadTailEntry::new(1, 0.3, 0.4).unwrap()])
        .unwrap();
    assert_eq!(adapter.query(1), Some((0.3, 0.4)));
    assert_eq!(adapter.retained_snapshot_count(), 3);
}

#[test]
fn realtime_queries_and_deferred_failure_reporting_do_not_allocate() {
    let adapter = RealtimeHeadTailAdapter::new(8).unwrap();
    adapter
        .install([HeadTailEntry::new(1, 0.1, 0.2).unwrap()])
        .unwrap();
    ALLOCATIONS.with(|allocations| allocations.set(0));
    TRACKING.with(|tracking| tracking.set(true));
    for _ in 0..32 {
        assert_eq!(adapter.query(1), Some((0.1, 0.2)));
        assert_eq!(adapter.query(2), None);
    }
    TRACKING.with(|tracking| tracking.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
}
