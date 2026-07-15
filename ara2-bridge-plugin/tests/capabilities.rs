use ara2_bridge_core::{ApiGeneration, DocumentProperties};
use ara2_bridge_plugin::{ControllerCapabilities, PluginBuilder};
use ara2_bridge_sys::ARADocumentControllerInterface;
use std::mem::offset_of;

#[test]
fn later_signal_capability_populates_ara_defined_intervening_defaults() {
    let plugin = PluginBuilder::new(())
        .signal_preservation(|_| false)
        .build()
        .unwrap();
    let interface = plugin
        .document_controller_interface(ApiGeneration::V23Final)
        .unwrap();
    assert_eq!(interface.represented_callback_count(), 54);
    assert!(interface.represented_callbacks_are_non_null());
    let raw = interface.raw_copy();
    let count = callback::<
        unsafe extern "C" fn(
            ara2_bridge_sys::ARADocumentControllerRef,
        ) -> ara2_bridge_sys::ARAInt32,
    >(
        &raw,
        offset_of!(ARADocumentControllerInterface, getProcessingAlgorithmsCount),
    );
    let licensed = callback::<
        unsafe extern "C" fn(
            ara2_bridge_sys::ARADocumentControllerRef,
            ara2_bridge_sys::ARABool,
            ara2_bridge_sys::ARASize,
            *const ara2_bridge_sys::ARAContentType,
            ara2_bridge_sys::ARAPlaybackTransformationFlags,
        ) -> ara2_bridge_sys::ARABool,
    >(
        &raw,
        offset_of!(ARADocumentControllerInterface, isLicensedForCapabilities),
    );
    // SAFETY: null controller refs select generated semantic defaults without dereferencing state.
    assert_eq!(unsafe { count.unwrap()(std::ptr::null_mut()) }, 0);
    // SAFETY: empty supported request uses the ARA-defined permissive licensing default.
    assert_eq!(
        unsafe { licensed.unwrap()(std::ptr::null_mut(), 0, 0, std::ptr::null(), 0) },
        ara2_bridge_sys::kARATrue
    );
}

#[test]
fn every_single_tail_capability_builds_a_consecutive_non_null_prefix() {
    let capabilities = [
        ControllerCapabilities::default().with_processing_algorithms(true),
        ControllerCapabilities::default().with_licensing(true),
        ControllerCapabilities::default().with_audio_file_chunk_storage(true),
        ControllerCapabilities::default().with_signal_preservation(true),
    ];
    for capability in capabilities {
        let interface =
            ara2_bridge_plugin::document_controller_interface(ApiGeneration::V23Final, capability)
                .unwrap();
        assert!(interface.represented_callbacks_are_non_null());
    }
}

#[test]
fn plugin_builder_retains_model_until_runtime_creation() {
    let plugin = PluginBuilder::new(7_u32).build().unwrap();
    assert_eq!(*plugin.model(), 7);
    assert_eq!(
        DocumentProperties::new(Some("document")).unwrap().name(),
        Some("document")
    );
}

fn callback<T: Copy>(raw: &ARADocumentControllerInterface, offset: usize) -> Option<T> {
    // SAFETY: `offset` is produced by `offset_of!` for the requested callback field, and the
    // complete packed record remains readable for this unaligned copy.
    unsafe {
        ara2_bridge_sys::access::read_field::<Option<T>>(
            (raw as *const ARADocumentControllerInterface).cast(),
            offset,
        )
    }
}
