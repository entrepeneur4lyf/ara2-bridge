use ara2_bridge_core::{
    ApiGeneration, AraBool, AraError, AudioSourceProperties, Color, ContentTimeRange,
    DocumentProperties, MusicalContextKind, MusicalContextProperties, PlaybackRegionKind,
    PlaybackRegionProperties, RegionSequenceKind, Registry, ViewSelection,
};
#[cfg(not(target_arch = "aarch64"))]
use ara2_bridge_sys::layout;
use ara2_bridge_sys::{access, ARAAudioSourceProperties, ARASize};
use std::ffi::{CStr, CString};
use std::mem::offset_of;

#[test]
fn audio_source_properties_copy_ephemeral_strings() {
    let name = CString::new("take.wav").unwrap();
    let persistent_id = CString::new("source-1").unwrap();
    let input = ARAAudioSourceProperties {
        structSize: std::mem::size_of::<ARAAudioSourceProperties>() as ARASize,
        name: name.as_ptr(),
        persistentID: persistent_id.as_ptr(),
        sampleCount: 48_000,
        sampleRate: 48_000.0,
        channelCount: 2,
        merits64BitSamples: 2,
        channelArrangementDataType: 0,
        channelArrangement: std::ptr::null(),
    };

    // SAFETY: `input` and its C strings are initialized and live for the complete copy.
    let owned = unsafe { AudioSourceProperties::copy_from_ffi(&input) }.unwrap();
    drop(name);
    drop(persistent_id);

    assert_eq!(owned.name(), Some("take.wav"));
    assert_eq!(owned.persistent_id(), "source-1");
    assert_eq!(owned.sample_count(), 48_000);
    assert_eq!(owned.sample_rate(), 48_000.0);
    assert_eq!(owned.channel_count(), 2);
    assert!(owned.merits_64_bit_samples());
}

#[test]
fn audio_source_output_retains_backing_and_uses_generation_prefix() {
    let owned = AudioSourceProperties::new(
        Some("take.wav"),
        "source-1",
        48_000,
        48_000.0,
        2,
        AraBool::new(false),
    )
    .unwrap();

    #[cfg(not(target_arch = "aarch64"))]
    {
        let legacy = owned.as_ffi(ApiGeneration::V1Final).unwrap();
        let legacy_pointer = legacy.as_ref().as_ptr().cast::<u8>();
        // SAFETY: the pinned guard owns a fully initialized raw record.
        let legacy_size: ARASize = unsafe {
            access::read_field(
                legacy_pointer,
                offset_of!(ARAAudioSourceProperties, structSize),
            )
        };
        assert_eq!(
            legacy_size,
            layout::ARAAUDIO_SOURCE_PROPERTIES_MERITS64_BIT_SAMPLES
        );
    }
    #[cfg(target_arch = "aarch64")]
    assert!(matches!(
        owned.as_ffi(ApiGeneration::V1Final),
        Err(AraError::Unsupported(_))
    ));

    let current = owned.as_ffi(ApiGeneration::V23Final).unwrap();
    let current_pointer = current.as_ref().as_ptr().cast::<u8>();
    // SAFETY: the pinned guard owns the raw record and the name pointer remains backed by `owned`.
    let name_pointer = unsafe {
        access::read_field::<*const std::os::raw::c_char>(
            current_pointer,
            offset_of!(ARAAudioSourceProperties, name),
        )
    };
    // SAFETY: the pointer targets `owned`'s retained NUL-terminated name.
    assert_eq!(
        unsafe { CStr::from_ptr(name_pointer) }.to_str().unwrap(),
        "take.wav"
    );
    assert_eq!(
        current.as_ref().raw_bytes().len(),
        std::mem::size_of::<ARAAudioSourceProperties>()
    );
}

#[test]
fn invalid_numeric_and_text_values_are_rejected() {
    assert!(matches!(
        AudioSourceProperties::new(None, "source-1", -1, 48_000.0, 2, AraBool::new(false)),
        Err(AraError::InvalidArgument(
            "sample count must be nonnegative"
        ))
    ));
    assert!(matches!(
        AudioSourceProperties::new(None, "source-1", 1, f64::NAN, 2, AraBool::new(false)),
        Err(AraError::InvalidArgument(
            "sample rate must be finite and positive"
        ))
    ));
    assert!(Color::new(0.0, 0.5, 1.0).is_ok());
    assert!(Color::new(0.0, f32::NAN, 1.0).is_err());
    assert!(DocumentProperties::new(Some("nul\0inside")).is_err());
}

#[test]
fn ara2_playback_regions_require_a_region_sequence() {
    let mut contexts = Registry::<MusicalContextKind, ()>::new(1);
    let mut sequences = Registry::<RegionSequenceKind, ()>::new(1);
    let context_handle = contexts.insert(()).unwrap();
    let sequence_handle = sequences.insert(()).unwrap();
    let context = contexts.model_ref(context_handle).unwrap();
    let sequence = sequences.model_ref(sequence_handle).unwrap();

    let legacy =
        PlaybackRegionProperties::for_ara1(0, 0.0, 1.0, 2.0, 1.0, context, None, None).unwrap();
    #[cfg(not(target_arch = "aarch64"))]
    assert!(legacy.as_ffi(ApiGeneration::V1Final).is_ok());
    #[cfg(target_arch = "aarch64")]
    assert!(matches!(
        legacy.as_ffi(ApiGeneration::V1Final),
        Err(AraError::Unsupported(_))
    ));
    assert!(matches!(
        legacy.as_ffi(ApiGeneration::V2Final),
        Err(AraError::InvalidArgument(
            "ARA2 playback region requires a region sequence"
        ))
    ));

    let current = PlaybackRegionProperties::for_ara2(
        0,
        0.0,
        1.0,
        2.0,
        1.0,
        sequence,
        Some("verse"),
        Some(Color::new(0.2, 0.3, 0.4).unwrap()),
    )
    .unwrap();
    assert!(current.as_ffi(ApiGeneration::V23Final).is_ok());
}

#[test]
fn optional_document_and_musical_context_data_are_owned() {
    let document = DocumentProperties::new(Some("Song")).unwrap();
    assert_eq!(document.name(), Some("Song"));

    let musical =
        MusicalContextProperties::new(Some("Verse"), 4, Some(Color::new(0.1, 0.2, 0.3).unwrap()))
            .unwrap();
    assert_eq!(musical.name(), Some("Verse"));
    assert_eq!(musical.order_index(), 4);
}

#[test]
fn view_selection_copies_reference_arrays_through_registry_resolvers() {
    let mut playbacks = Registry::<PlaybackRegionKind, ()>::new(1);
    let mut sequences = Registry::<RegionSequenceKind, ()>::new(1);
    let playback = playbacks.insert(()).unwrap();
    let sequence = sequences.insert(()).unwrap();
    let selection = ViewSelection::new(
        &[playbacks.model_ref(playback).unwrap()],
        &[sequences.model_ref(sequence).unwrap()],
        Some(ContentTimeRange::new(-1.0, 4.0).unwrap()),
    )
    .unwrap();
    let raw = selection.as_ffi(ApiGeneration::V23Final).unwrap();

    // SAFETY: the pinned guard and both registries retain every represented pointer for the copy.
    let copied = unsafe {
        ViewSelection::copy_from_ffi_with_refs(
            raw.as_ref().as_ptr(),
            |reference| {
                let handle = playbacks.handle_from_opaque(reference.cast()).unwrap();
                playbacks.model_ref(handle)
            },
            |reference| {
                let handle = sequences.handle_from_opaque(reference.cast()).unwrap();
                sequences.model_ref(handle)
            },
        )
    }
    .unwrap();

    assert_eq!(copied.playback_region_count(), 1);
    assert_eq!(copied.region_sequence_count(), 1);
    assert_eq!(copied.time_range().unwrap().start(), -1.0);
    assert_eq!(copied.time_range().unwrap().duration(), 4.0);
}
