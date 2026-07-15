#[test]
fn coverage_command_is_registered_and_report_is_current() {
    xtask::run(["ara", "coverage", "--help"].map(str::to_owned)).unwrap();
    xtask::run(["ara", "coverage", "--check"].map(str::to_owned)).unwrap();
}

#[test]
fn coverage_report_closes_core_and_companion_inventories() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("docs/conformance/interface-coverage.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["summary"]["callback_slots"], 95);
    assert_eq!(report["summary"]["source_symbols"], 547);
    assert_eq!(report["summary"]["core_abi_symbols"], 498);
    assert_eq!(report["summary"]["companion_records"], 49);
    assert_eq!(report["summary"]["companion_unique_symbols"], 47);
    assert_eq!(report["summary"]["unresolved_symbols"], 0);
}
