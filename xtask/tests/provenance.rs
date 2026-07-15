#[test]
fn pinned_ara_api_matches_manifest() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    xtask::provenance::verify(root, root.join("sdk-provenance.toml")).unwrap();
}
