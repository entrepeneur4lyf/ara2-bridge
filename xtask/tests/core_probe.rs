#[test]
fn all_core_probe_families_are_present_and_current() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    xtask::core_probe::check_all(root).unwrap();
}
