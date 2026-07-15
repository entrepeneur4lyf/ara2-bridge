use ara2_bridge_core::{
    copy_event_from_ffi, validate_event_sequence, BarSignatures, ChordIntervalUsage, ContentGrade,
    ContentKind, ContentUpdateScopes, KeySignatureIntervalUsage, KeySignatures, Notes, SheetChords,
    StaticTuning, Tempo,
};
use ara2_bridge_sys::{
    kARAContentTypeBarSignatures, kARAContentTypeKeySignatures, kARAContentTypeNotes,
    kARAContentTypeSheetChords, kARAContentTypeStaticTuning, kARAContentTypeTempoEntries,
    kARAInvalidFrequency, kARAInvalidPitchNumber, ARAContentBarSignature, ARAContentChord,
    ARAContentKeySignature, ARAContentNote, ARAContentTempoEntry, ARAContentTuning, ARAContentType,
};
use std::ffi::{c_void, CString};

fn assert_kind<K: ContentKind>(expected: ARAContentType) {
    assert_eq!(K::RAW_TYPE, expected);
}

unsafe fn copy<K: ContentKind, R>(raw_type: ARAContentType, raw: &R) -> K::Event {
    // SAFETY: `raw` is a complete initialized event of the kind selected by each call site.
    unsafe {
        copy_event_from_ffi::<K>(
            raw_type,
            (raw as *const R).cast::<c_void>(),
            std::mem::size_of::<R>(),
        )
    }
    .expect("valid content event")
}

#[test]
fn every_released_kind_has_a_typed_event() {
    assert_kind::<Tempo>(kARAContentTypeTempoEntries as ARAContentType);
    assert_kind::<BarSignatures>(kARAContentTypeBarSignatures as ARAContentType);
    assert_kind::<Notes>(kARAContentTypeNotes as ARAContentType);
    assert_kind::<StaticTuning>(kARAContentTypeStaticTuning as ARAContentType);
    assert_kind::<KeySignatures>(kARAContentTypeKeySignatures as ARAContentType);
    assert_kind::<SheetChords>(kARAContentTypeSheetChords as ARAContentType);
}

#[test]
fn packed_input_is_copied_into_aligned_owned_events() {
    let raw = ARAContentTempoEntry {
        timePosition: 1.25,
        quarterPosition: 2.5,
    };
    let mut packed = vec![0_u8; std::mem::size_of_val(&raw) + 1];
    // SAFETY: the destination has enough initialized storage and deliberately starts unaligned.
    unsafe {
        std::ptr::copy_nonoverlapping(
            (&raw as *const ARAContentTempoEntry).cast::<u8>(),
            packed.as_mut_ptr().add(1),
            std::mem::size_of_val(&raw),
        );
    }
    // SAFETY: `packed[1..]` contains a complete initialized tempo event for this call.
    let event = unsafe {
        copy_event_from_ffi::<Tempo>(
            kARAContentTypeTempoEntries as ARAContentType,
            packed.as_ptr().add(1).cast::<c_void>(),
            std::mem::size_of::<ARAContentTempoEntry>(),
        )
    }
    .expect("valid packed event");
    assert_eq!(event.time_position(), 1.25);
    assert_eq!(event.quarter_position(), 2.5);
}

#[test]
fn all_event_families_copy_names_arrays_and_sentinels() {
    let bar = ARAContentBarSignature {
        numerator: 4,
        denominator: 4,
        position: 0.0,
    };
    // SAFETY: `bar` is a complete bar-signature event.
    assert_eq!(unsafe { copy::<BarSignatures, _>(21, &bar) }.numerator(), 4);

    let note = ARAContentNote {
        frequency: 440.0,
        pitchNumber: 69,
        volume: 0.75,
        startPosition: 1.0,
        attackDuration: 0.1,
        noteDuration: 0.8,
        signalDuration: 1.0,
    };
    // SAFETY: `note` is a complete note event.
    let note = unsafe { copy::<Notes, _>(10, &note) };
    assert_eq!(note.pitch_number(), 69);
    assert_eq!(note.frequency(), Some(440.0));

    let unpitched = ARAContentNote {
        frequency: kARAInvalidFrequency,
        pitchNumber: kARAInvalidPitchNumber,
        volume: 0.5,
        startPosition: 2.0,
        attackDuration: 0.0,
        noteDuration: 0.1,
        signalDuration: 0.1,
    };
    // SAFETY: `unpitched` is a complete note using the paired ARA sentinels.
    assert_eq!(unsafe { copy::<Notes, _>(10, &unpitched) }.pitch(), None);

    let tuning_name = CString::new("Rast").unwrap();
    let tuning = ARAContentTuning {
        concertPitchFrequency: 440.0,
        root: 0,
        tunings: [0.0; 12],
        name: tuning_name.as_ptr(),
    };
    // SAFETY: `tuning` and its NUL-terminated name remain alive for the copy.
    assert_eq!(
        unsafe { copy::<StaticTuning, _>(31, &tuning) }.name(),
        Some("Rast")
    );

    let key_name = CString::new("C major").unwrap();
    let key = ARAContentKeySignature {
        root: 0,
        intervals: [0xAB; 12],
        name: key_name.as_ptr(),
        position: 0.0,
    };
    // SAFETY: `key` and its NUL-terminated name remain alive for the copy.
    let key = unsafe { copy::<KeySignatures, _>(42, &key) };
    assert_eq!(key.intervals()[0].as_raw(), 0xAB);
    assert_eq!(key.name(), Some("C major"));

    let chord = ARAContentChord {
        root: 0,
        bass: 4,
        intervals: [0xFF; 12],
        name: std::ptr::null(),
        position: 4.0,
    };
    // SAFETY: `chord` is a complete chord event with no nested name.
    let chord = unsafe { copy::<SheetChords, _>(45, &chord) };
    assert_eq!(chord.bass(), 4);
    assert_eq!(chord.intervals()[0], ChordIntervalUsage::USED);
}

#[test]
fn malformed_storage_and_values_are_rejected() {
    let raw = ARAContentTempoEntry {
        timePosition: f64::NAN,
        quarterPosition: 0.0,
    };
    // SAFETY: storage is readable; the test intentionally supplies a malformed value.
    assert!(unsafe {
        copy_event_from_ffi::<Tempo>(20, (&raw as *const ARAContentTempoEntry).cast(), 16)
    }
    .is_err());
    // SAFETY: storage is readable; the test intentionally supplies a mismatched type.
    assert!(unsafe {
        copy_event_from_ffi::<Tempo>(10, (&raw as *const ARAContentTempoEntry).cast(), 16)
    }
    .is_err());
    // SAFETY: a null pointer is permitted as an input to the validator and rejected before reading.
    assert!(unsafe { copy_event_from_ffi::<Tempo>(20, std::ptr::null(), 16) }.is_err());
    // SAFETY: storage is readable; the extent is intentionally too short and rejected before reading.
    assert!(unsafe {
        copy_event_from_ffi::<Tempo>(20, (&raw as *const ARAContentTempoEntry).cast(), 8)
    }
    .is_err());

    let bad_note = ARAContentNote {
        frequency: 0.0,
        pitchNumber: 60,
        volume: 2.0,
        startPosition: 0.0,
        attackDuration: 0.0,
        noteDuration: 0.0,
        signalDuration: 0.0,
    };
    // SAFETY: storage is readable; sentinel mismatch and range violations are intentional.
    assert!(unsafe {
        copy_event_from_ffi::<Notes>(
            10,
            (&bad_note as *const ARAContentNote).cast(),
            std::mem::size_of_val(&bad_note),
        )
    }
    .is_err());

    assert!(ChordIntervalUsage::from_raw(8).is_err());
}

#[test]
fn note_volume_matches_the_upstream_nonnegative_contract() {
    let amplified =
        ara2_bridge_core::NoteEvent::new(Some(440.0), Some(69), 2.0, 0.0, 0.0, 1.0, 1.0)
            .expect("ARA permits nonnegative volume above unity");
    assert_eq!(amplified.volume(), 2.0);
    assert!(
        ara2_bridge_core::NoteEvent::new(Some(440.0), Some(69), -0.01, 0.0, 0.0, 1.0, 1.0,)
            .is_err()
    );
}

#[test]
fn sequence_rules_match_upstream_and_future_values_are_retained() {
    let tempo = [
        ara2_bridge_core::TempoEvent::new(0.0, 0.0).unwrap(),
        ara2_bridge_core::TempoEvent::new(1.0, 2.0).unwrap(),
    ];
    assert!(validate_event_sequence::<Tempo>(&tempo).is_ok());
    let reversed = [tempo[1], tempo[0]];
    assert!(validate_event_sequence::<Tempo>(&reversed).is_err());
    assert!(validate_event_sequence::<StaticTuning>(&[]).is_err());

    assert_eq!(ContentGrade::from_raw(99).as_raw(), 99);
    assert_eq!(KeySignatureIntervalUsage::from_raw(0x7A).as_raw(), 0x7A);
    let flags = ContentUpdateScopes::from_bits_retain(1 << 29);
    assert_eq!(flags.bits(), 1 << 29);
}
