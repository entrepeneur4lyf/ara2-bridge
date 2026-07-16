#[test]
fn pinned_ara_api_matches_manifest() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    xtask::provenance::verify(root, root.join("sdk-provenance.toml")).unwrap();
}

#[test]
fn pinned_vst3_sdk_is_the_mit_licensed_3_8_release() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let sdk_lock = std::fs::read_to_string(root.join("ci/reference-sdks.lock.toml")).unwrap();

    assert!(sdk_lock.contains("tag = \"v3.8.0_build_66\""));
    assert!(sdk_lock.contains("commit = \"9fad9770f2ae8542ab1a548a68c1ad1ac690abe0\""));
    assert!(sdk_lock.contains("accepted_licenses = [\"MIT\"]"));
    assert!(!sdk_lock.contains("LicenseRef-Steinberg-VST3"));
}

#[test]
fn every_companion_manifest_tracks_the_shared_build_script() {
    use sha2::{Digest, Sha256};

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let build_script = std::fs::read(root.join("ara2-bridge-companion/build.rs")).unwrap();
    let build_script_hash = format!("{:x}", Sha256::digest(build_script));

    for component in ["clap", "vst3", "audio-unit"] {
        let manifest = std::fs::read_to_string(
            root.join(format!("ara2-bridge-companion/provenance/{component}.toml")),
        )
        .unwrap();
        let expected_entry = format!(
            "path = \"ara2-bridge-companion/build.rs\"\nrole = \"generated-declaration\"\nsha256 = \"{build_script_hash}\""
        );

        assert!(
            manifest.contains(&expected_entry),
            "{component} provenance does not track the current shared build script"
        );
    }
}

#[test]
fn sdk_bootstrap_forces_byte_stable_git_checkouts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let bootstrap = std::fs::read_to_string(root.join("ci/bootstrap-reference-sdks.sh")).unwrap();

    assert!(bootstrap.contains("git -c core.autocrlf=false -c core.filemode=false clone"));
    assert!(bootstrap.contains("git -C \"$temporary\" config core.autocrlf false"));
    assert!(bootstrap.contains("git -C \"$temporary\" config core.filemode false"));
    assert!(bootstrap.contains("submodule foreach --recursive"));
}
