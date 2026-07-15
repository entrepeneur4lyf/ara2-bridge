use std::fs;

#[test]
fn host_dispatch_generation_rejects_missing_and_stale_output() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("ara2-bridge-sys/src/generated/x86_64.rs");
    let input = root.join("ara2-bridge-sys/src/generated/x86_64.rs");
    fs::create_dir_all(input.parent().unwrap()).unwrap();
    fs::copy(source, input).unwrap();

    assert!(xtask::host_dispatch::generate(root, xtask::Mode::Check).is_err());
    let output = root.join("ara2-bridge-host/src/plugin/generated_dispatch.rs");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(&output, [0_u8]).unwrap();
    assert!(xtask::host_dispatch::generate(root, xtask::Mode::Check).is_err());
    xtask::host_dispatch::generate(root, xtask::Mode::Write).unwrap();
    xtask::host_dispatch::generate(root, xtask::Mode::Check).unwrap();

    let generated = fs::read_to_string(output).unwrap();
    assert_eq!(generated.matches("DispatchMethod {").count(), 55);
    assert_eq!(generated.matches("\npub(crate) unsafe fn ").count(), 54);
    assert_eq!(generated.matches("pub(crate) unsafe fn raw_").count(), 53);
    assert!(generated.contains("DO NOT EDIT"));
}
