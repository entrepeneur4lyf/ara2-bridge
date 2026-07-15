use ara2_bridge_sys::audio_file_chunks;

#[test]
fn released_cpp_only_chunk_name_has_a_reviewed_rust_binding() {
    assert_eq!(
        audio_file_chunks::kARAXMLName_CreateDistinctAudioModification,
        "createDistinctAudioModification"
    );
}
