use ara2_bridge_core::{
    dispatch_bool, dispatch_i32, dispatch_ref, dispatch_time_pair, dispatch_void, AraError,
    BoundedDiagnosticSink, Diagnostic, DiagnosticSink, DispatchRuntime, DocumentId, InstanceId,
    PoisonState,
};
use ara2_bridge_sys::{kARAFalse, kARATrue, ARABool};
use std::ptr::NonNull;

#[derive(Default)]
struct Runtime {
    poison: PoisonState,
    diagnostics: BoundedDiagnosticSink,
}

impl DispatchRuntime for Runtime {
    fn is_poisoned(&self) -> bool {
        self.poison.is_poisoned()
    }

    fn poison(&self, diagnostic: Diagnostic) {
        self.poison.poison(diagnostic);
    }

    fn record_diagnostic(&self, diagnostic: Diagnostic) {
        self.diagnostics.record(diagnostic);
    }

    fn document_id(&self) -> Option<DocumentId> {
        Some(DocumentId::new(3))
    }

    fn instance_id(&self) -> Option<InstanceId> {
        Some(InstanceId::new(7))
    }
}

#[test]
fn panic_is_recorded_poisoned_and_mapped_to_false() {
    let runtime = Runtime::default();
    let result = dispatch_bool(&runtime, "Interface", "method", || {
        panic!("boom");
    });

    assert_eq!(result, kARAFalse);
    assert!(runtime.is_poisoned());
    let diagnostics = runtime.diagnostics.snapshot();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].interface(), Some("Interface"));
    assert_eq!(diagnostics[0].method(), Some("method"));
    assert_eq!(diagnostics[0].document(), Some(DocumentId::new(3)));
    assert_eq!(diagnostics[0].instance(), Some(InstanceId::new(7)));
}

#[test]
fn expected_errors_map_to_method_sentinels_without_poisoning() {
    let runtime = Runtime::default();
    assert_eq!(
        dispatch_bool(&runtime, "I", "bool", || Err(AraError::Peer("no"))),
        kARAFalse
    );
    assert_eq!(
        dispatch_i32(&runtime, "I", "int", || Err(AraError::Peer("no"))),
        0
    );
    assert_eq!(
        dispatch_ref::<u8>(&runtime, "I", "ref", || Err(AraError::Peer("no"))),
        std::ptr::null_mut()
    );
    assert_eq!(
        dispatch_time_pair(&runtime, "I", "time", || Err(AraError::Peer("no"))),
        (0.0, 0.0)
    );
    dispatch_void(&runtime, "I", "void", || Err(AraError::Peer("no")));
    assert!(!runtime.is_poisoned());
    assert_eq!(runtime.diagnostics.snapshot().len(), 5);
}

#[test]
fn successful_and_nested_dispatch_preserve_values() {
    let runtime = Runtime::default();
    assert_eq!(
        dispatch_bool(&runtime, "Outer", "bool", || {
            let nested = dispatch_i32(&runtime, "Inner", "int", || Ok(42));
            Ok(nested == 42)
        }),
        kARATrue
    );
    let mut value = 9_u8;
    let expected = std::ptr::addr_of_mut!(value);
    assert_eq!(
        dispatch_ref(&runtime, "I", "ref", || Ok(NonNull::new(expected))),
        expected
    );
    assert_eq!(
        dispatch_time_pair(&runtime, "I", "time", || Ok((0.25, 0.5))),
        (0.25, 0.5)
    );
}

unsafe extern "C" fn panic_boundary(runtime: *const Runtime) -> ARABool {
    // SAFETY: the test passes a live runtime pointer for this call.
    let runtime = unsafe { &*runtime };
    dispatch_bool(runtime, "Extern", "panicBoundary", || panic!("contained"))
}

#[test]
fn no_unwind_crosses_an_extern_c_boundary() {
    let runtime = Runtime::default();
    // SAFETY: `runtime` remains live for the complete call.
    assert_eq!(unsafe { panic_boundary(&runtime) }, kARAFalse);
    assert!(runtime.is_poisoned());
}
