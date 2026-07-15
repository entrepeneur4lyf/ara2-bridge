use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/clippy-fixtures")
        .join(name)
        .join("Cargo.toml")
}

fn clippy_failure(name: &str, lint: &str) -> String {
    let output = Command::new(env!("CARGO"))
        .args([
            "clippy",
            "--quiet",
            "--manifest-path",
            fixture(name).to_str().unwrap(),
            "--",
            "-D",
            lint,
        ])
        .env(
            "CARGO_TARGET_DIR",
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../target/clippy-ui")
                .join(name),
        )
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "fixture unexpectedly passed: {name}"
    );
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn missing_safety_sections_are_rejected() {
    let stderr = clippy_failure("missing-safety-doc", "clippy::missing-safety-doc");
    assert!(stderr.contains("unsafe function's docs are missing a `# Safety` section"));
}

#[test]
fn undocumented_unsafe_blocks_are_rejected() {
    let stderr = clippy_failure(
        "undocumented-unsafe-block",
        "clippy::undocumented-unsafe-blocks",
    );
    assert!(stderr.contains("unsafe block missing a safety comment"));
}
