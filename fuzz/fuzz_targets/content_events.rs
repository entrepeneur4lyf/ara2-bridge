#![no_main]

use ara2_bridge_core::{
    copy_event_from_ffi, BarSignatures, KeySignatures, Notes, SheetChords, StaticTuning, Tempo,
};
use ara2_bridge_sys::{
    kARAContentTypeBarSignatures, kARAContentTypeKeySignatures, kARAContentTypeSheetChords,
    kARAContentTypeNotes, kARAContentTypeStaticTuning, kARAContentTypeTempoEntries,
    ARAContentChord, ARAContentKeySignature, ARAContentTuning,
};
use libfuzzer_sys::fuzz_target;
use std::ffi::{c_char, c_void};
use std::mem::offset_of;
use std::ptr::null;

fn clear_nested_name(kind: u8, storage: &mut [u8]) {
    let offset = match kind {
        3 => offset_of!(ARAContentTuning, name),
        4 => offset_of!(ARAContentKeySignature, name),
        5 => offset_of!(ARAContentChord, name),
        _ => return,
    };
    if storage.len() >= offset + std::mem::size_of::<*const c_char>() {
        // SAFETY: the checked byte extent contains the complete pointer field; unaligned storage is
        // intentional and the event decoder also accepts it.
        unsafe {
            storage
                .as_mut_ptr()
                .add(offset)
                .cast::<*const c_char>()
                .write_unaligned(null());
        }
    }
}

fn decode(kind: u8, storage: &[u8]) {
    let pointer = storage.as_ptr().cast::<c_void>();
    let extent = storage.len();
    // SAFETY: `storage` is caller-valid for exactly `extent` bytes. Nested display-name pointers
    // are cleared before kinds that contain them reach the production decoder.
    unsafe {
        match kind {
            0 => {
                let _ = copy_event_from_ffi::<Tempo>(
                    kARAContentTypeTempoEntries as i32,
                    pointer,
                    extent,
                );
            }
            1 => {
                let _ = copy_event_from_ffi::<BarSignatures>(
                    kARAContentTypeBarSignatures as i32,
                    pointer,
                    extent,
                );
            }
            2 => {
                let _ = copy_event_from_ffi::<Notes>(
                    kARAContentTypeNotes as i32,
                    pointer,
                    extent,
                );
            }
            3 => {
                let _ = copy_event_from_ffi::<StaticTuning>(
                    kARAContentTypeStaticTuning as i32,
                    pointer,
                    extent,
                );
            }
            4 => {
                let _ = copy_event_from_ffi::<KeySignatures>(
                    kARAContentTypeKeySignatures as i32,
                    pointer,
                    extent,
                );
            }
            _ => {
                let _ = copy_event_from_ffi::<SheetChords>(
                    kARAContentTypeSheetChords as i32,
                    pointer,
                    extent,
                );
            }
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let data = &data[..data.len().min(64 * 1024)];
    let mut cursor = 0;
    while cursor + 3 <= data.len() {
        let kind = data[cursor] % 6;
        let requested = u16::from_le_bytes([data[cursor + 1], data[cursor + 2]]) as usize;
        cursor += 3;
        let available = requested.min(data.len() - cursor);
        let mut storage = data[cursor..cursor + available].to_vec();
        clear_nested_name(kind, &mut storage);
        decode(kind, &storage);
        cursor += available;
    }
});
