#[test]
fn generated_compatibility_metadata_is_current() {
    xtask::compatibility::generate(xtask::Mode::Check).unwrap();
}

#[test]
fn document_controller_manifest_has_all_slots_in_header_order() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let generated =
        std::fs::read_to_string(root.join("ara2-bridge-sys/src/generated/compatibility.rs"))
            .unwrap();
    let callbacks = generated
        .split("pub const DOCUMENT_CONTROLLER_CALLBACKS")
        .nth(1)
        .unwrap()
        .split("];")
        .next()
        .unwrap();
    assert_eq!(callbacks.matches('"').count() / 2, 54);
    assert!(callbacks.contains("destroyDocumentController"));
    assert!(callbacks.contains("isAudioModificationPreservingAudioSourceSignal"));
}
