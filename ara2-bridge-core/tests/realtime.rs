use ara2_bridge_core::{
    AraError, HeadTailEntry, RealtimeFailureCode, RealtimeFailureQueue, RealtimeHeadTailView,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct TrackingAllocator;

thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TRACKING.with(|tracking| {
            if tracking.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        // SAFETY: the layout and allocator contract are forwarded unchanged to `System`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the pointer/layout pair originated from `System` above.
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

#[test]
fn head_tail_queries_are_bounded_and_allocation_free() {
    let view = RealtimeHeadTailView::new(
        [
            HeadTailEntry::new(20, 0.2, 0.3).unwrap(),
            HeadTailEntry::new(10, 0.1, 0.4).unwrap(),
        ],
        8,
    )
    .unwrap();

    ALLOCATIONS.with(|count| count.set(0));
    TRACKING.with(|tracking| tracking.set(true));
    let result = view.query(20);
    TRACKING.with(|tracking| tracking.set(false));

    assert_eq!(result, Some((0.2, 0.3)));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert_eq!(view.query(99), None);
}

#[test]
fn missing_view_and_over_capacity_snapshots_fail_explicitly() {
    let missing: Option<RealtimeHeadTailView> = None;
    assert_eq!(missing.as_ref().and_then(|view| view.query(1)), None);
    assert!(matches!(
        RealtimeHeadTailView::new([HeadTailEntry::new(1, 0.0, 0.0).unwrap()], 0),
        Err(AraError::InvalidArgument(
            "realtime snapshot exceeds configured capacity"
        ))
    ));
}

#[test]
fn realtime_failures_use_a_preallocated_bounded_queue() {
    let queue = RealtimeFailureQueue::new(2).unwrap();
    assert!(queue.report(RealtimeFailureCode::MissingRegion));
    assert!(queue.report(RealtimeFailureCode::InvalidState));
    assert!(!queue.report(RealtimeFailureCode::PeerFailure));
    assert_eq!(queue.pop(), Some(RealtimeFailureCode::MissingRegion));
    assert_eq!(queue.pop(), Some(RealtimeFailureCode::InvalidState));
    assert_eq!(queue.pop(), None);
}
