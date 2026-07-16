use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

fn chapter(number: u8) -> String {
    format!(
        r#"
[[chapter]]
number = {number}
title = "Chapter {number}"
normative_specs = ["docs/specs/ara2-bridge/08-packaging-versioning-and-manual.md"]
public_apis = ["ara2_bridge::core"]
examples = ["ara2-bridge/examples/minimal-plugin.rs"]
conformance_commands = ["cargo test --workspace"]
testhost_args = ["not-applicable: unit-level chapter"]
companion_binaries = ["not-applicable: unit-level chapter"]
sdk_environment = ["not-applicable: unit-level chapter"]
required_capabilities = ["ARA 2.3"]
expected_skips = 0
fixture_hashes = ["not-applicable: no fixture"]
platform_steps = ["not-applicable: portable chapter"]
gui_main_loop = ["not-applicable: no GUI"]
timeouts = ["30 seconds"]
troubleshooting = ["docs/troubleshooting.md#general-diagnostics"]
"#
    )
}

fn valid_map() -> String {
    let mut source = String::from("# Manual Source Map\n\n```toml manual-source-map\nschema = 1\n");
    for number in 1..=12 {
        source.push_str(&chapter(number));
    }
    source.push_str("```\n");
    source
}

fn fixture_root() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs/specs/ara2-bridge")).unwrap();
    fs::create_dir_all(temp.path().join("docs")).unwrap();
    fs::create_dir_all(temp.path().join("ara2-bridge/examples")).unwrap();
    fs::create_dir_all(temp.path().join("ara2-bridge-core/src")).unwrap();
    fs::write(
        temp.path()
            .join("docs/specs/ara2-bridge/08-packaging-versioning-and-manual.md"),
        "# Spec\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("docs/troubleshooting.md"),
        "# General diagnostics\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("ara2-bridge/examples/minimal-plugin.rs"),
        "fn main() {}\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("ara2-bridge-core/src/lib.rs"),
        "pub struct ExistingApi;\n",
    )
    .unwrap();
    temp
}

fn write_map(root: &Path, source: &str) -> PathBuf {
    let path = root.join("docs/manual-source-map.md");
    fs::write(&path, source).unwrap();
    path
}

#[test]
fn docs_help_is_registered() {
    xtask::run(["docs".to_owned(), "--help".to_owned()]).unwrap();
}

#[test]
fn crate_root_contract_rejects_missing_sections_and_fabricated_symbols() {
    let complete = r#"
//! # Role and boundaries
//! No direct C counterpart.
//! # Lifecycle and threading
//! Ownership and failure behavior.
//! # Features and platforms
//! Portable.
//! # Compatibility and licensing
//! MIT OR Apache-2.0. https://github.com/Celemony/ARA_API
//! # Example
//! ```rust
//! fn main() {}
//! ```
"#;
    xtask::docs::verify_crate_root_contract("fixture", complete, &["ARAFactory"]).unwrap();

    let error = xtask::docs::verify_crate_root_contract(
        "fixture",
        &complete.replace("# Features and platforms", "# Platforms"),
        &["ARAFactory"],
    )
    .unwrap_err();
    assert!(error.contains("Features and platforms"), "{error}");

    let fabricated = complete.replace(
        "No direct C counterpart.",
        "Direct ARA C counterpart: `ARAFabricatedFactory`.",
    );
    let error = xtask::docs::verify_crate_root_contract("fixture", &fabricated, &["ARAFactory"])
        .unwrap_err();
    assert!(error.contains("fabricated ARA C symbol"), "{error}");
}

#[test]
fn checked_in_crate_root_documentation_contracts_validate() {
    xtask::docs::verify_public_docs_path(&root()).unwrap();
}

#[test]
fn complete_fixture_validates() {
    let temp = fixture_root();
    let map = write_map(temp.path(), &valid_map());
    xtask::docs::verify_manual_map_path(temp.path(), &map).unwrap();
}

#[test]
fn missing_chapter_is_rejected() {
    let temp = fixture_root();
    let source = valid_map().replace(&chapter(12), "");
    let error = xtask::docs::verify_manual_map_path(temp.path(), &write_map(temp.path(), &source))
        .unwrap_err();
    assert!(error.contains("missing chapter 12"), "{error}");
}

#[test]
fn missing_example_is_rejected() {
    let temp = fixture_root();
    let source = valid_map().replacen(
        "ara2-bridge/examples/minimal-plugin.rs",
        "ara2-bridge/examples/missing.rs",
        1,
    );
    let error = xtask::docs::verify_manual_map_path(temp.path(), &write_map(temp.path(), &source))
        .unwrap_err();
    assert!(error.contains("missing example"), "{error}");
}

#[test]
fn invalid_command_reference_is_rejected() {
    let temp = fixture_root();
    let source = valid_map().replacen("cargo test --workspace", "not-a-command", 1);
    let error = xtask::docs::verify_manual_map_path(temp.path(), &write_map(temp.path(), &source))
        .unwrap_err();
    assert!(error.contains("invalid conformance command"), "{error}");
}

#[test]
fn fabricated_public_api_reference_is_rejected() {
    let temp = fixture_root();
    let source = valid_map().replacen(
        "ara2_bridge::core\"]",
        "ara2_bridge::core::FabricatedApi\"]",
        1,
    );
    let error = xtask::docs::verify_manual_map_path(temp.path(), &write_map(temp.path(), &source))
        .unwrap_err();
    assert!(error.contains("missing public API"), "{error}");
}

#[test]
fn missing_cargo_target_is_rejected() {
    let temp = fixture_root();
    let source = valid_map().replacen(
        "cargo test --workspace",
        "cargo test -p ara2-bridge --test missing",
        1,
    );
    let error = xtask::docs::verify_manual_map_path(temp.path(), &write_map(temp.path(), &source))
        .unwrap_err();
    assert!(error.contains("missing cargo test target"), "{error}");
}

#[test]
fn stale_fixture_hash_is_rejected() {
    let temp = fixture_root();
    let source = valid_map().replacen(
        "not-applicable: no fixture",
        "ara2-bridge/examples/minimal-plugin.rs@0000000000000000000000000000000000000000000000000000000000000000",
        1,
    );
    let error = xtask::docs::verify_manual_map_path(temp.path(), &write_map(temp.path(), &source))
        .unwrap_err();
    assert!(error.contains("fixture hash mismatch"), "{error}");
}

#[test]
fn missing_troubleshooting_anchor_is_rejected() {
    let temp = fixture_root();
    let source = valid_map().replacen("#general-diagnostics", "#missing-heading", 1);
    let error = xtask::docs::verify_manual_map_path(temp.path(), &write_map(temp.path(), &source))
        .unwrap_err();
    assert!(error.contains("missing troubleshooting anchor"), "{error}");
}

#[test]
fn omitted_conformance_fields_are_rejected_independently() {
    for (field, value) in [
        ("testhost_args", "not-applicable: unit-level chapter"),
        ("companion_binaries", "not-applicable: unit-level chapter"),
        ("sdk_environment", "not-applicable: unit-level chapter"),
        ("required_capabilities", "ARA 2.3"),
    ] {
        let temp = fixture_root();
        let source = valid_map().replacen(
            &format!("{field} = [\"{value}\"]"),
            &format!("{field} = []"),
            1,
        );
        let error =
            xtask::docs::verify_manual_map_path(temp.path(), &write_map(temp.path(), &source))
                .unwrap_err();
        assert!(error.contains(field), "{field}: {error}");
    }
}

#[test]
fn checked_in_manual_map_is_complete() {
    let root = root();
    xtask::docs::verify_manual_map_path(&root, &root.join("docs/manual-source-map.md")).unwrap();
}
