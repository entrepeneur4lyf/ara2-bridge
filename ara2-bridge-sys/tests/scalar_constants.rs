use ara2_bridge_sys::{
    kARADefaultConcertPitchFrequency, kARAFalse, kARAInvalidFrequency, kARAInvalidPitchNumber,
    kARATrue, ARABool, ARAPitchNumber,
};

#[test]
fn cast_style_ara_bool_macros_are_preserved() {
    assert_eq!(kARAFalse, 0 as ARABool);
    assert_eq!(kARATrue, 1 as ARABool);
}

#[test]
fn pitch_sentinels_preserve_their_c_types() {
    let invalid_frequency: f32 = kARAInvalidFrequency;
    let default_frequency: f32 = kARADefaultConcertPitchFrequency;
    let invalid_pitch: ARAPitchNumber = kARAInvalidPitchNumber;
    assert_eq!(invalid_frequency, 0.0);
    assert_eq!(default_frequency, 440.0);
    assert_eq!(invalid_pitch, i32::MIN);
}

#[test]
fn generated_layouts_expose_declaration_order_extents() {
    let extents = ara2_bridge_sys::layout::ARAAUDIO_SOURCE_PROPERTIES_FIELD_EXTENTS;
    assert_eq!(extents.first().copied(), Some(std::mem::size_of::<usize>()));
    assert!(extents.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        extents.last().copied(),
        Some(std::mem::size_of::<ara2_bridge_sys::ARAAudioSourceProperties>())
    );
}
