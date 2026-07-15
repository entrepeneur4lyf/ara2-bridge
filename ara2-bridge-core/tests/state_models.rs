use ara2_bridge_core::{AraError, Lifecycle};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

struct ReaderAccessModel {
    enabled: AtomicBool,
    reads: AtomicUsize,
}

impl ReaderAccessModel {
    fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            reads: AtomicUsize::new(0),
        }
    }

    fn begin_read(&self) -> bool {
        if !self.enabled.load(Ordering::Acquire) {
            return false;
        }
        self.reads.fetch_add(1, Ordering::AcqRel);
        if self.enabled.load(Ordering::Acquire) {
            true
        } else {
            self.reads.fetch_sub(1, Ordering::AcqRel);
            false
        }
    }

    fn finish_read(&self) {
        self.reads.fetch_sub(1, Ordering::Release);
    }

    fn revoke(&self) {
        self.enabled.store(false, Ordering::Release);
    }

    fn is_drained(&self) -> bool {
        self.reads.load(Ordering::Acquire) == 0
    }
}

#[test]
fn reader_revocation_rejects_queued_reads_and_drains_in_flight_reads() {
    let model = Arc::new(ReaderAccessModel::new());
    assert!(model.begin_read());

    model.revoke();
    assert!(!model.begin_read());
    assert!(!model.is_drained());

    model.finish_read();
    assert!(model.is_drained());
}

const ANALYSIS_ACTIVE: u8 = 0;
const ANALYSIS_CANCELLED: u8 = 1;
const ANALYSIS_COMPLETE: u8 = 2;

struct AnalysisModel {
    state: AtomicU8,
    callbacks: AtomicUsize,
}

impl AnalysisModel {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(ANALYSIS_ACTIVE),
            callbacks: AtomicUsize::new(0),
        }
    }

    fn cancel(&self) {
        let _ = self.state.compare_exchange(
            ANALYSIS_ACTIVE,
            ANALYSIS_CANCELLED,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    fn publish_update(&self) -> bool {
        if self.state.load(Ordering::Acquire) != ANALYSIS_ACTIVE {
            return false;
        }
        self.callbacks.fetch_add(1, Ordering::AcqRel);
        true
    }

    fn complete(&self) -> bool {
        self.state
            .compare_exchange(
                ANALYSIS_ACTIVE,
                ANALYSIS_COMPLETE,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }
}

#[test]
fn analysis_cancellation_suppresses_late_updates_and_completion() {
    let model = AnalysisModel::new();
    assert!(model.publish_update());
    model.cancel();

    assert!(!model.publish_update());
    assert!(!model.complete());
    assert_eq!(model.state.load(Ordering::Acquire), ANALYSIS_CANCELLED);
    assert_eq!(model.callbacks.load(Ordering::Acquire), 1);
}

#[test]
fn render_activation_blocks_controller_teardown_until_worker_release() {
    let lifecycle = Arc::new(Lifecycle::new_on_current_thread());
    let activated = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let worker_lifecycle = Arc::clone(&lifecycle);
    let worker_activated = Arc::clone(&activated);
    let worker_release = Arc::clone(&release);

    let worker = std::thread::spawn(move || {
        let activation = worker_lifecycle.begin_render_activation().unwrap();
        worker_activated.wait();
        worker_release.wait();
        activation.finish().unwrap();
    });

    activated.wait();
    assert!(matches!(
        lifecycle.begin_teardown(),
        Err(AraError::InvalidState(
            "cannot teardown while scoped activity is active"
        ))
    ));
    release.wait();
    worker.join().unwrap();
    lifecycle.begin_teardown().unwrap().finish().unwrap();
}

struct ExtensionOwners {
    controller: AtomicBool,
    companion: AtomicBool,
    updates: AtomicUsize,
}

impl ExtensionOwners {
    fn new() -> Self {
        Self {
            controller: AtomicBool::new(true),
            companion: AtomicBool::new(true),
            updates: AtomicUsize::new(0),
        }
    }

    fn update(&self) -> bool {
        if !self.controller.load(Ordering::Acquire) || !self.companion.load(Ordering::Acquire) {
            return false;
        }
        self.updates.fetch_add(1, Ordering::AcqRel);
        true
    }

    fn destroy_controller(&self) {
        self.controller.store(false, Ordering::Release);
    }

    fn destroy_companion(&self) {
        self.companion.store(false, Ordering::Release);
    }

    fn storage_releasable(&self) -> bool {
        !self.controller.load(Ordering::Acquire) && !self.companion.load(Ordering::Acquire)
    }
}

#[test]
fn editor_updates_and_both_extension_teardown_orders_are_deterministic() {
    for controller_first in [true, false] {
        let owners = ExtensionOwners::new();
        assert!(owners.update());

        if controller_first {
            owners.destroy_controller();
            assert!(!owners.update());
            assert!(!owners.storage_releasable());
            owners.destroy_companion();
        } else {
            owners.destroy_companion();
            assert!(!owners.update());
            assert!(!owners.storage_releasable());
            owners.destroy_controller();
        }

        assert!(owners.storage_releasable());
        assert_eq!(owners.updates.load(Ordering::Acquire), 1);
    }
}
