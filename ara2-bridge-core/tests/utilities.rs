use ara2_bridge_core::{
    intersect_content_ranges, sample_to_time, time_to_sample, BarMap, BarSignatureEvent,
    ChannelArrangement, ChannelFormat, ChordEvent, ChordIntervalUsage, ContentTimeRange,
    CoreAudioChannelDescription, CoreAudioChannelLayout, KeySignatureEvent,
    KeySignatureIntervalUsage, LicenseCapabilities, LicenseRequest, OpaqueChannelArrangement,
    PitchInterpreter, PlaybackTransformationFlags, ProcessingAlgorithmCatalog,
    ProcessingAlgorithmProperties, ScaleMode, TempoEvent, TempoMap,
};

#[test]
fn sample_time_rounding_matches_ara_library() {
    assert_eq!(time_to_sample(0.5, 44_100.0).unwrap(), 22_050);
    assert_eq!(time_to_sample(-0.5 / 44_100.0, 44_100.0).unwrap(), 0);
    assert_eq!(time_to_sample(-1.5 / 44_100.0, 44_100.0).unwrap(), -1);
    assert_eq!(sample_to_time(-1, 44_100.0).unwrap(), -1.0 / 44_100.0);
    assert!(time_to_sample(f64::MAX, 192_000.0).is_err());
}

#[test]
fn tempo_and_bar_maps_cover_extrapolation_negative_time_and_unusual_meter() {
    let tempo = TempoMap::new(vec![
        TempoEvent::new(0.0, 0.0).unwrap(),
        TempoEvent::new(2.0, 4.0).unwrap(),
        TempoEvent::new(3.0, 7.0).unwrap(),
    ])
    .unwrap();
    assert_eq!(tempo.quarter_at_time(-1.0).unwrap(), -2.0);
    assert_eq!(tempo.quarter_at_time(2.5).unwrap(), 5.5);
    assert_eq!(tempo.time_at_quarter(10.0).unwrap(), 4.0);
    for quarter in [-4.0, 0.0, 1.25, 4.0, 8.0] {
        let time = tempo.time_at_quarter(quarter).unwrap();
        assert!((tempo.quarter_at_time(time).unwrap() - quarter).abs() < 1.0e-12);
    }

    let bars = BarMap::new(vec![
        BarSignatureEvent::new(7, 8, 0.0).unwrap(),
        BarSignatureEvent::new(3, 4, 7.0).unwrap(),
    ])
    .unwrap();
    assert_eq!(bars.beat_at_quarter(3.5).unwrap(), 7.0);
    assert_eq!(bars.quarter_at_beat(7.0).unwrap(), 3.5);
    // The upstream implementation adds 0.5 before truncating, including for negative bars.
    assert_eq!(bars.bar_index_at_quarter(-3.5).unwrap(), 0);
    assert_eq!(bars.quarter_at_bar_index(2).unwrap(), 7.0);
    assert!(BarMap::new(vec![
        BarSignatureEvent::new(4, 4, 0.0).unwrap(),
        BarSignatureEvent::new(3, 4, 3.0).unwrap(),
    ])
    .is_err());
}

#[test]
fn pitch_chord_and_key_names_match_upstream_spelling() {
    let unicode = PitchInterpreter::new(false, false);
    let ascii_german = PitchInterpreter::new(true, true);
    assert_eq!(unicode.note_name(6), "F♯");
    assert_eq!(unicode.note_name(-2), "B♭");
    assert_eq!(ascii_german.note_name(5), "H");
    assert_eq!(ascii_german.note_name(-2), "B");

    let mut major = [KeySignatureIntervalUsage::UNUSED; 12];
    for index in [0, 2, 4, 5, 7, 9, 11] {
        major[index] = KeySignatureIntervalUsage::USED;
    }
    let key = KeySignatureEvent::new(0, major, None, 0.0).unwrap();
    assert_eq!(PitchInterpreter::scale_mode(&key), ScaleMode::Ionian);
    assert_eq!(unicode.key_name(&key).as_deref(), Some("C"));

    let unused = ChordIntervalUsage::UNUSED;
    let mut intervals = [unused; 12];
    intervals[0] = ChordIntervalUsage::from_raw(1).unwrap();
    intervals[4] = ChordIntervalUsage::from_raw(3).unwrap();
    intervals[7] = ChordIntervalUsage::from_raw(5).unwrap();
    let chord = ChordEvent::new(0, 0, intervals, None, 0.0).unwrap();
    assert_eq!(unicode.chord_name(&chord), "C");
    let no_chord = ChordEvent::new(0, 0, [unused; 12], None, 0.0).unwrap();
    assert!(PitchInterpreter::is_no_chord(&no_chord));
    assert_eq!(unicode.chord_name(&no_chord), "N.C.");

    let chord = |third: Option<(usize, u8)>,
                 fifth: Option<(usize, u8)>,
                 seventh: Option<(usize, u8)>,
                 bass| {
        let mut values = [ChordIntervalUsage::UNUSED; 12];
        values[0] = ChordIntervalUsage::from_raw(1).unwrap();
        for (index, usage) in [third, fifth, seventh].into_iter().flatten() {
            values[index] = ChordIntervalUsage::from_raw(usage).unwrap();
        }
        ChordEvent::new(0, bass, values, None, 0.0).unwrap()
    };
    assert_eq!(
        unicode.chord_name(&chord(Some((3, 3)), Some((7, 5)), None, 0)),
        "Cm"
    );
    assert_eq!(
        unicode.chord_name(&chord(Some((4, 3)), Some((7, 5)), Some((10, 7)), 0)),
        "C7"
    );
    assert_eq!(
        ascii_german.chord_name(&chord(Some((4, 3)), Some((7, 5)), Some((11, 7)), 0)),
        "Cmaj7"
    );
    assert_eq!(
        unicode.chord_name(&chord(None, Some((7, 5)), None, 0)),
        "C5"
    );
    assert_eq!(
        unicode.chord_name(&chord(Some((5, 4)), Some((7, 5)), None, 1)),
        "Csus4/G"
    );

    let modes: [(&[usize], ScaleMode, &str); 7] = [
        (&[0, 2, 4, 5, 7, 9, 11], ScaleMode::Ionian, "C"),
        (&[0, 2, 3, 5, 7, 9, 10], ScaleMode::Dorian, "C Dorian"),
        (&[0, 1, 3, 5, 7, 8, 10], ScaleMode::Phrygian, "C Phrygian"),
        (&[0, 2, 4, 6, 7, 9, 11], ScaleMode::Lydian, "C Lydian"),
        (
            &[0, 2, 4, 5, 7, 9, 10],
            ScaleMode::Mixolydian,
            "C Mixolydian",
        ),
        (&[0, 2, 3, 5, 7, 8, 10], ScaleMode::Aeolian, "Cm"),
        (&[0, 1, 3, 5, 6, 8, 10], ScaleMode::Locrian, "C Locrian"),
    ];
    for (used, mode, name) in modes {
        let mut intervals = [KeySignatureIntervalUsage::UNUSED; 12];
        for index in used {
            intervals[*index] = KeySignatureIntervalUsage::USED;
        }
        let key = KeySignatureEvent::new(0, intervals, None, 0.0).unwrap();
        assert_eq!(PitchInterpreter::scale_mode(&key), mode);
        assert_eq!(unicode.key_name(&key).as_deref(), Some(name));
    }
}

#[test]
fn content_ranges_intersect_with_half_open_semantics() {
    let left = ContentTimeRange::new(1.0, 4.0).unwrap();
    let right = ContentTimeRange::new(3.0, 4.0).unwrap();
    let overlap = intersect_content_ranges(left, right).unwrap().unwrap();
    assert_eq!((overlap.start(), overlap.duration()), (3.0, 2.0));
    assert!(intersect_content_ranges(
        ContentTimeRange::new(0.0, 1.0).unwrap(),
        ContentTimeRange::new(1.0, 2.0).unwrap(),
    )
    .unwrap()
    .is_none());
    assert!(intersect_content_ranges(
        ContentTimeRange::new(f64::MAX, f64::MAX).unwrap(),
        ContentTimeRange::new(0.0, 1.0).unwrap(),
    )
    .is_err());
}

#[test]
fn every_owned_channel_arrangement_validates_its_implied_count() {
    assert!(ChannelFormat::new(2, ChannelArrangement::Undefined).is_ok());
    assert!(ChannelFormat::new(3, ChannelArrangement::Undefined).is_err());
    assert!(ChannelFormat::new(2, ChannelArrangement::Vst3(0b11)).is_ok());
    assert!(ChannelFormat::new(1, ChannelArrangement::Aax(2)).is_err());
    assert!(ChannelFormat::new(3, ChannelArrangement::ClapMap(vec![1, 2, 3])).is_ok());
    assert!(ChannelFormat::new(
        4,
        ChannelArrangement::ClapAmbisonic {
            ordering: 1,
            normalization: 2
        },
    )
    .is_ok());
    assert!(ChannelFormat::new(
        1,
        ChannelArrangement::CoreAudio(CoreAudioChannelLayout::Descriptions(vec![
            CoreAudioChannelDescription::new(1, 0, [0.0, 0.0, 0.0]).unwrap(),
        ])),
    )
    .is_ok());
    assert!(ChannelArrangement::from_raw(99, &[1, 2], 2).is_err());
    assert_eq!(
        ChannelArrangement::from_raw(1, &3_u64.to_ne_bytes(), 2).unwrap(),
        ChannelArrangement::Vst3(3)
    );
    assert!(ChannelFormat::new(
        2,
        ChannelArrangement::CoreAudio(CoreAudioChannelLayout::Tag(0x0065_0002)),
    )
    .is_ok());
    let mut raw_layout = Vec::new();
    raw_layout.extend_from_slice(&0_u32.to_ne_bytes());
    raw_layout.extend_from_slice(&0_u32.to_ne_bytes());
    raw_layout.extend_from_slice(&1_u32.to_ne_bytes());
    raw_layout.extend_from_slice(&1_u32.to_ne_bytes());
    raw_layout.extend_from_slice(&0_u32.to_ne_bytes());
    for coordinate in [0.0_f32, 1.0, -1.0] {
        raw_layout.extend_from_slice(&coordinate.to_ne_bytes());
    }
    let decoded = ChannelArrangement::from_raw(2, &raw_layout, 1).unwrap();
    assert!(ChannelFormat::new(1, decoded).is_ok());
    // SAFETY: this test deliberately supplies a complete fictional future representation.
    let opaque = unsafe { OpaqueChannelArrangement::new_unchecked(99, vec![1, 2].into()) };
    assert!(ChannelFormat::new(2, ChannelArrangement::Opaque(opaque)).is_ok());
}

#[test]
fn processing_indices_are_stable_and_license_requests_are_subsets() {
    let catalog = ProcessingAlgorithmCatalog::new(vec![
        ProcessingAlgorithmProperties::new("clean", "Clean").unwrap(),
        ProcessingAlgorithmProperties::new("solo", "Solo").unwrap(),
    ])
    .unwrap();
    let first_pointer = catalog.raw(0).unwrap().persistent_id();
    assert_eq!(catalog.get(1).unwrap().persistent_id(), "solo");
    assert_eq!(catalog.raw(0).unwrap().persistent_id(), first_pointer);
    assert!(catalog.get(-1).is_err());
    assert!(catalog.get(2).is_err());
    let raw = catalog.raw(0).unwrap().as_ara();
    // SAFETY: the packed generated field is initialized and read without creating a reference.
    assert_eq!(
        unsafe { std::ptr::addr_of!(raw.structSize).read_unaligned() },
        std::mem::size_of_val(&raw)
    );

    let supported_flags =
        PlaybackTransformationFlags::TIMESTRETCH | PlaybackTransformationFlags::REFLECT_TEMPO;
    let capabilities = LicenseCapabilities::new([1, 2, 3], supported_flags).unwrap();
    let request = LicenseRequest::new(
        false,
        [2, 1],
        PlaybackTransformationFlags::TIMESTRETCH,
        &capabilities,
    )
    .unwrap();
    assert_eq!(request.content_types(), &[2, 1]);
    assert!(LicenseRequest::new(
        false,
        [4],
        PlaybackTransformationFlags::empty(),
        &capabilities
    )
    .is_err());
    assert!(LicenseRequest::new(
        false,
        [1],
        PlaybackTransformationFlags::CONTENT_FADE_HEAD,
        &capabilities
    )
    .is_err());
    let future = PlaybackTransformationFlags::from_bits_retain(1 << 31);
    let future_capabilities = LicenseCapabilities::new([1], future).unwrap();
    assert!(LicenseRequest::new(false, [1], future, &future_capabilities).is_ok());
}
