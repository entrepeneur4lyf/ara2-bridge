#![no_main]

use ara2_bridge_core::{AraChunkSet, ChunkLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = ChunkLimits {
        max_xml_bytes: 256 * 1024,
        max_archive_bytes: 256 * 1024,
        max_entries: 4_096,
    };
    let _ = AraChunkSet::from_audio_with_limits(data, limits);
});
