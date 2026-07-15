use ara2_bridge_core::ApiGeneration;
use ara2_bridge_plugin::{ExtensionBinding, ExtensionRoles};
use ara2_bridge_sys::*;
use std::mem::offset_of;

#[test]
fn role_enablement_matches_the_sdk_formula() {
    let all = ExtensionRoles::all();
    assert_eq!(
        ExtensionRoles::resolve(ExtensionRoles::empty(), ExtensionRoles::empty(), all).unwrap(),
        all
    );
    assert_eq!(
        ExtensionRoles::resolve(all, ExtensionRoles::empty(), all).unwrap(),
        ExtensionRoles::empty()
    );
    let assigned = ExtensionRoles::PLAYBACK_RENDERER | ExtensionRoles::EDITOR_VIEW;
    assert_eq!(
        ExtensionRoles::resolve(all, assigned, all).unwrap(),
        assigned
    );
    assert!(ExtensionRoles::resolve(
        ExtensionRoles::PLAYBACK_RENDERER,
        ExtensionRoles::EDITOR_RENDERER,
        all,
    )
    .is_err());
}

#[test]
fn role_calls_validate_assignment_and_controller_lifetime() {
    let (binding, controller) = ExtensionBinding::new(
        ApiGeneration::V23Final,
        ExtensionRoles::all(),
        ExtensionRoles::PLAYBACK_RENDERER,
        ExtensionRoles::all(),
    )
    .unwrap();
    assert!(binding
        .add_playback_region(ExtensionRoles::PLAYBACK_RENDERER, 11)
        .is_ok());
    assert!(binding
        .add_playback_region(ExtensionRoles::EDITOR_RENDERER, 11)
        .is_err());
    controller.destroy();
    assert!(binding
        .remove_playback_region(ExtensionRoles::PLAYBACK_RENDERER, 11)
        .is_err());
}

#[test]
fn interface_storage_survives_either_owner_destruction_order() {
    let (binding, controller) = ExtensionBinding::new(
        ApiGeneration::V23Final,
        ExtensionRoles::empty(),
        ExtensionRoles::empty(),
        ExtensionRoles::all(),
    )
    .unwrap();
    let raw = binding.as_raw();
    drop(binding);
    assert!(controller.storage_is_alive());
    // SAFETY: the controller lease still owns the complete interface allocation.
    let size = unsafe { std::ptr::addr_of!((*raw).structSize).read_unaligned() };
    assert!(size >= ara2_bridge_sys::kARAPlugInExtensionInstanceMinSize as usize);
    controller.destroy();

    let (binding, controller) = ExtensionBinding::new(
        ApiGeneration::V23Final,
        ExtensionRoles::empty(),
        ExtensionRoles::empty(),
        ExtensionRoles::all(),
    )
    .unwrap();
    controller.destroy();
    assert!(binding.storage_is_alive());
    drop(binding);
}

#[test]
fn ara1_binding_exposes_legacy_extension_prefix() {
    if ApiGeneration::V1Final.supported_on_target() {
        let (binding, _controller) = ExtensionBinding::new(
            ApiGeneration::V1Final,
            ExtensionRoles::empty(),
            ExtensionRoles::empty(),
            ExtensionRoles::all(),
        )
        .unwrap();
        assert!(binding.has_legacy_extension());
        assert_eq!(binding.enabled_roles(), ExtensionRoles::empty());
        let raw = binding.as_raw();
        // SAFETY: complete live extension instance fields.
        let reference = unsafe {
            ara2_bridge_sys::access::read_field::<ARAPlugInExtensionRef>(
                raw.cast(),
                offset_of!(ARAPlugInExtensionInstance, plugInExtensionRef),
            )
        };
        // SAFETY: same live instance contract.
        let interface = unsafe {
            ara2_bridge_sys::access::read_field::<*const ARAPlugInExtensionInterface>(
                raw.cast(),
                offset_of!(ARAPlugInExtensionInstance, plugInExtensionInterface),
            )
        };
        // SAFETY: the legacy prefix represents this non-null callback.
        let set = unsafe {
            ara2_bridge_sys::access::read_field::<
                Option<unsafe extern "C" fn(ARAPlugInExtensionRef, ARAPlaybackRegionRef)>,
            >(
                interface.cast(),
                offset_of!(ARAPlugInExtensionInterface, setPlaybackRegion),
            )
        }
        .unwrap();
        let identity = Box::new(0_u8);
        let region: ARAPlaybackRegionRef = std::ptr::from_ref(identity.as_ref()).cast_mut().cast();
        // SAFETY: role reference and playback identity remain live for the call.
        unsafe { set(reference, region) };
        assert_eq!(
            binding.view_selection().unwrap().playback_regions(),
            [region as usize]
        );
    }
}

#[test]
fn editor_view_callbacks_copy_selection_and_hidden_sequence_arrays() {
    let (binding, _controller) = ExtensionBinding::new(
        ApiGeneration::V23Final,
        ExtensionRoles::all(),
        ExtensionRoles::EDITOR_VIEW,
        ExtensionRoles::all(),
    )
    .unwrap();
    let raw = binding.as_raw();
    // SAFETY: complete live extension instance fields.
    let reference = unsafe {
        ara2_bridge_sys::access::read_field::<ARAEditorViewRef>(
            raw.cast(),
            offset_of!(ARAPlugInExtensionInstance, editorViewRef),
        )
    };
    // SAFETY: same live instance contract.
    let interface = unsafe {
        ara2_bridge_sys::access::read_field::<*const ARAEditorViewInterface>(
            raw.cast(),
            offset_of!(ARAPlugInExtensionInstance, editorViewInterface),
        )
    };
    // SAFETY: the represented editor-view prefix contains both callbacks.
    let notify = unsafe {
        ara2_bridge_sys::access::read_field::<
            Option<unsafe extern "C" fn(ARAEditorViewRef, *const ARAViewSelection)>,
        >(
            interface.cast(),
            offset_of!(ARAEditorViewInterface, notifySelection),
        )
    }
    .unwrap();
    // SAFETY: same represented prefix.
    let hide = unsafe {
        ara2_bridge_sys::access::read_field::<
            Option<unsafe extern "C" fn(ARAEditorViewRef, ARASize, *const ARARegionSequenceRef)>,
        >(
            interface.cast(),
            offset_of!(ARAEditorViewInterface, notifyHideRegionSequences),
        )
    }
    .unwrap();
    let playback_identity = Box::new(0_u8);
    let sequence_identity = Box::new(0_u8);
    let playback: ARAPlaybackRegionRef = std::ptr::from_ref(playback_identity.as_ref())
        .cast_mut()
        .cast();
    let sequence: ARARegionSequenceRef = std::ptr::from_ref(sequence_identity.as_ref())
        .cast_mut()
        .cast();
    let playbacks = [playback];
    let sequences = [sequence];
    let range = ARAContentTimeRange {
        start: 1.0,
        duration: 2.0,
    };
    let selection = ARAViewSelection {
        structSize: std::mem::size_of::<ARAViewSelection>(),
        playbackRegionRefsCount: playbacks.len(),
        playbackRegionRefs: playbacks.as_ptr(),
        regionSequenceRefsCount: sequences.len(),
        regionSequenceRefs: sequences.as_ptr(),
        timeRange: &raw const range,
    };
    // SAFETY: all role, selection, nested-array, identity, and range storage remains live.
    unsafe {
        notify(reference, &raw const selection);
        hide(reference, sequences.len(), sequences.as_ptr());
    }
    let copied = binding.view_selection().unwrap();
    assert_eq!(copied.playback_regions(), [playback as usize]);
    assert_eq!(copied.region_sequences(), [sequence as usize]);
    let copied_range = copied.time_range().unwrap();
    assert_eq!((copied_range.start(), copied_range.duration()), (1.0, 2.0));
    assert_eq!(binding.hidden_region_sequences(), [sequence as usize]);
}
