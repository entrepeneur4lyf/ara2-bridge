#[test]
fn all_core_probe_families_are_present_and_current() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    if !root
        .join(".third-party/ARA_SDK/ARA_API/ARAInterface.h")
        .is_file()
        || !root.join(".third-party/ARA_SDK/.git").exists()
    {
        eprintln!("skipping core probe freshness without the maintainer ARA SDK checkout");
        return;
    }
    xtask::core_probe::check_all(root).unwrap();
}
