#![no_main]

use ara2_bridge_core::{SizedInput, SizedRecord};
use ara2_bridge_sys::{
    ARAAudioSourceProperties, ARADocumentControllerInterface, ARADocumentProperties, ARAFactory,
    ARAInterfaceConfiguration, ARAPlugInExtensionInterface,
};
use libfuzzer_sys::fuzz_target;
use std::mem::size_of;

fn validate<T: SizedRecord>(data: &[u8]) {
    let requested = data
        .get(1..=2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) as usize)
        .unwrap_or(0);
    let advertised = requested.min(4_096);
    let mut storage = vec![0_u8; advertised.max(size_of::<usize>())];
    let payload = data.get(3..).unwrap_or_default();
    let copied = payload.len().min(storage.len().saturating_sub(size_of::<usize>()));
    storage[size_of::<usize>()..size_of::<usize>() + copied]
        .copy_from_slice(&payload[..copied]);
    // SAFETY: `storage` contains at least one native ARASize and remains live for validation. The
    // written advertised extent is bounded by the same allocation.
    unsafe {
        storage
            .as_mut_ptr()
            .cast::<usize>()
            .write_unaligned(advertised);
        let _ = SizedInput::<T>::from_ptr(storage.as_ptr().cast());
    }
}

fuzz_target!(|data: &[u8]| {
    let data = &data[..data.len().min(4_096)];
    match data.first().copied().unwrap_or_default() % 6 {
        0 => validate::<ARADocumentProperties>(data),
        1 => validate::<ARAAudioSourceProperties>(data),
        2 => validate::<ARAInterfaceConfiguration>(data),
        3 => validate::<ARAFactory>(data),
        4 => validate::<ARADocumentControllerInterface>(data),
        _ => validate::<ARAPlugInExtensionInterface>(data),
    }
});
