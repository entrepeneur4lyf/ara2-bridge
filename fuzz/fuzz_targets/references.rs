#![no_main]

use ara2_bridge_core::Registry;
use libfuzzer_sys::fuzz_target;
use std::ffi::c_void;
use std::ptr::null_mut;

enum Kind {}

fuzz_target!(|data: &[u8]| {
    let mut first = Registry::<Kind, u8>::new(8);
    let handle = first.insert(data.get(1).copied().unwrap_or_default()).unwrap();
    let pointer = first.opaque_pointer(handle).unwrap();

    match data.first().copied().unwrap_or_default() % 4 {
        0 => {
            let _ = first.handle_from_opaque(null_mut::<c_void>());
        }
        1 => {
            first.remove(handle).unwrap();
            let _ = first.handle_from_opaque(pointer.as_ptr());
        }
        2 => {
            let second = Registry::<Kind, u8>::new(8);
            let _ = second.handle_from_opaque(pointer.as_ptr());
        }
        _ => {
            let resolved = first.handle_from_opaque(pointer.as_ptr()).unwrap();
            assert_eq!(resolved, handle);
        }
    }
});
