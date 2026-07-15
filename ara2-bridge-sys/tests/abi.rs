mod generated_assertions {
    include!("generated/core_abi_assertions.rs");
}

#[test]
fn current_target_layout_and_constants_match_c_and_cpp() {
    let family = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86") {
        "i686"
    } else {
        panic!("unsupported core ABI test target")
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generated")
        .join(format!("{family}-core-abi.json"));
    let envelope: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path)
            .unwrap_or_else(|error| panic!("missing or unreadable {}: {error}", path.display())),
    )
    .unwrap();
    generated_assertions::assert_current(&envelope["payload"]);
}
