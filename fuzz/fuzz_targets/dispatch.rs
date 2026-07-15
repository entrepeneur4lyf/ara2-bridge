#![no_main]

use ara2_bridge_core::{
    dispatch_bool, dispatch_i32, dispatch_ref, dispatch_time_pair, dispatch_void, AraError,
    BoundedDiagnosticSink, Diagnostic, DiagnosticSink, DispatchRuntime, PoisonState,
};
use libfuzzer_sys::fuzz_target;
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
}

fuzz_target!(|data: &[u8]| {
    let runtime = Runtime::default();
    let operation = data.get(1).copied().unwrap_or_default() % 5;
    let outcome = data.get(2).copied().unwrap_or_default() % 2;
    let mut value = 0_u8;

    match operation {
        0 => {
            let _ = dispatch_bool(&runtime, "Fuzz", "bool", || match outcome {
                0 => Ok(true),
                _ => Err(AraError::Peer("fuzzed failure")),
            });
        }
        1 => {
            let _ = dispatch_i32(&runtime, "Fuzz", "i32", || match outcome {
                0 => Ok(i32::from(data.first().copied().unwrap_or_default())),
                _ => Err(AraError::Peer("fuzzed failure")),
            });
        }
        2 => {
            let pointer = NonNull::new(&mut value);
            let _ = dispatch_ref(&runtime, "Fuzz", "ref", || match outcome {
                0 => Ok(pointer),
                _ => Err(AraError::Peer("fuzzed failure")),
            });
        }
        3 => {
            let _ = dispatch_time_pair(&runtime, "Fuzz", "time", || match outcome {
                0 => Ok((0.25, 0.5)),
                _ => Err(AraError::Peer("fuzzed failure")),
            });
        }
        _ => dispatch_void(&runtime, "Fuzz", "void", || match outcome {
            0 => Ok(()),
            _ => Err(AraError::Peer("fuzzed failure")),
        }),
    }

    assert!(runtime.diagnostics.snapshot().len() <= 1);
});
