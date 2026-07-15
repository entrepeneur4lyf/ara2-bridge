use ara2_bridge_core::{AraBool, AraError, ForeignSlice, ForeignStr, SizedInput};
use ara2_bridge_sys::{
    kARAAudioSourcePropertiesMinSize, kARAFalse, kARATrue, layout, ARAAudioSourceProperties,
    ARABool, ARASize,
};
use proptest::prelude::*;
use std::mem::{align_of, size_of};

#[test]
fn rejects_partial_tail_without_reading_it() {
    let advertised = size_of::<ARASize>() + 1;
    let mut bytes = vec![0_u8; advertised];
    bytes[..size_of::<ARASize>()].copy_from_slice(&(advertised as ARASize).to_ne_bytes());

    // SAFETY: `bytes` is readable for the complete advertised extent encoded in its first word.
    let result = unsafe { SizedInput::<ARAAudioSourceProperties>::from_ptr(bytes.as_ptr().cast()) };
    assert!(matches!(result, Err(AraError::Abi("struct too small"))));
}

#[test]
fn accepts_complete_prefixes_future_tails_and_unaligned_storage() {
    for advertised in layout::ARAAUDIO_SOURCE_PROPERTIES_FIELD_EXTENTS
        .iter()
        .copied()
        .filter(|size| *size >= kARAAudioSourcePropertiesMinSize as usize)
        .chain([size_of::<ARAAudioSourceProperties>() + 16])
    {
        let mut storage = vec![0_u8; advertised + 1];
        storage[1..1 + size_of::<ARASize>()]
            .copy_from_slice(&(advertised as ARASize).to_ne_bytes());
        // SAFETY: the deliberately unaligned pointer is readable for `advertised` initialized bytes.
        let input = unsafe {
            SizedInput::<ARAAudioSourceProperties>::from_ptr(storage.as_ptr().add(1).cast())
        }
        .unwrap();
        assert_eq!(input.advertised_size(), advertised);
    }
}

#[test]
fn rejects_nonzero_count_with_null_pointer_and_arithmetic_overflow() {
    // SAFETY: a null pointer with a nonzero count is rejected before any access.
    assert!(matches!(
        unsafe { ForeignSlice::<u32>::copy_from_raw(std::ptr::null(), 1) },
        Err(AraError::InvalidArgument("null array with nonzero count"))
    ));
    // SAFETY: overflow is rejected before any access to this dangling but aligned pointer.
    assert!(matches!(
        unsafe {
            ForeignSlice::<u32>::copy_from_raw(std::ptr::NonNull::dangling().as_ptr(), usize::MAX)
        },
        Err(AraError::InvalidArgument("array extent overflow"))
    ));
}

#[test]
fn copies_aligned_arrays_and_rejects_misalignment() {
    let values = [3_u32, 5, 8];
    // SAFETY: `values` is aligned, initialized, and readable for three elements.
    let copied = unsafe { ForeignSlice::copy_from_raw(values.as_ptr(), values.len()) }.unwrap();
    assert_eq!(copied.as_slice(), values);

    let storage = [0_u8; size_of::<u32>() + align_of::<u32>()];
    let offset = (0..align_of::<u32>())
        .find(|offset| (storage.as_ptr() as usize + offset) % align_of::<u32>() != 0)
        .unwrap();
    // SAFETY: the pointer is readable but deliberately misaligned and is rejected before copying.
    let result =
        unsafe { ForeignSlice::<u32>::copy_from_raw(storage.as_ptr().add(offset).cast(), 1) };
    assert!(matches!(
        result,
        Err(AraError::InvalidArgument("misaligned array pointer"))
    ));
}

#[test]
fn validates_display_strings_and_persistent_ids_with_bounds() {
    let display = b"take.wav\0";
    // SAFETY: `display` is readable through its terminator within the supplied bound.
    assert_eq!(
        unsafe { ForeignStr::copy_display(display.as_ptr().cast(), display.len()) }
            .unwrap()
            .as_str(),
        "take.wav"
    );

    let invalid_utf8 = [0xff_u8, 0];
    // SAFETY: `invalid_utf8` is readable through its terminator.
    assert!(matches!(
        unsafe { ForeignStr::copy_display(invalid_utf8.as_ptr().cast(), invalid_utf8.len()) },
        Err(AraError::InvalidArgument("display string is not UTF-8"))
    ));

    let empty = b"\0";
    // SAFETY: `empty` is readable through its terminator.
    assert!(matches!(
        unsafe { ForeignStr::copy_persistent_id(empty.as_ptr().cast(), empty.len()) },
        Err(AraError::InvalidArgument(
            "persistent ID must be nonempty ASCII"
        ))
    ));
}

#[test]
fn ara_bool_uses_nonzero_input_and_canonical_output() {
    assert!(!AraBool::from_raw(0).get());
    assert!(AraBool::from_raw(1).get());
    assert!(AraBool::from_raw(2).get());
    assert!(AraBool::from_raw(-1).get());
    assert_eq!(AraBool::new(false).into_raw(), kARAFalse);
    assert_eq!(AraBool::new(true).into_raw(), kARATrue);
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn every_raw_bool_round_trips_canonically(raw in any::<ARABool>()) {
        let value = AraBool::from_raw(raw);
        prop_assert_eq!(value.get(), raw != 0);
        let canonical = value.into_raw();
        prop_assert!(canonical == kARAFalse || canonical == kARATrue);
    }
}

#[cfg(not(miri))]
#[test]
fn raw_bool_conversion_is_centralized() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut pending = vec![root];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rs")
                && path.file_name().and_then(std::ffi::OsStr::to_str) != Some("scalar.rs")
            {
                let source = std::fs::read_to_string(&path).unwrap();
                let forbidden = [
                    "raw != 0",
                    "raw!=0",
                    "!= kARAFalse",
                    "as ARABool",
                    "ARABool = 0",
                    "ARABool = 1",
                ];
                assert!(
                    forbidden.iter().all(|pattern| !source.contains(pattern)),
                    "direct ARABool conversion outside ffi::scalar: {}",
                    path.display()
                );
            }
        }
    }
}
