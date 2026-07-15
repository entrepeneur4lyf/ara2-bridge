use sha2::{Digest, Sha256};
use std::fs;

#[test]
fn chunk_xml_generation_rejects_missing_and_stale_outputs() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    assert!(xtask::fixtures::generate(root, xtask::Mode::Check, "chunk-xml").is_err());

    let output = root.join("ara2-bridge-testkit/fixtures/chunks/legacy-missing-distinct.xml");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(output, [0_u8]).unwrap();
    assert!(xtask::fixtures::generate(root, xtask::Mode::Check, "chunk-xml").is_err());
    xtask::fixtures::generate(root, xtask::Mode::Write, "chunk-xml").unwrap();
    xtask::fixtures::generate(root, xtask::Mode::Check, "chunk-xml").unwrap();
}

#[test]
fn audio_container_generation_rejects_missing_and_stale_outputs() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    assert!(xtask::fixtures::generate(root, xtask::Mode::Check, "audio-containers").is_err());

    let output = root.join("ara2-bridge-testkit/fixtures/audio/wave-unknown-odd.wav");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(output, [0_u8]).unwrap();
    assert!(xtask::fixtures::generate(root, xtask::Mode::Check, "audio-containers").is_err());
    xtask::fixtures::generate(root, xtask::Mode::Write, "audio-containers").unwrap();
    xtask::fixtures::generate(root, xtask::Mode::Check, "audio-containers").unwrap();
}

#[test]
fn upstream_scenario_generation_rejects_missing_and_stale_outputs() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    assert!(xtask::fixtures::generate(root, xtask::Mode::Check, "upstream-scenarios").is_err());

    let output = root.join("ara2-bridge-testkit/fixtures/scenarios/ara1-full.archive");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(output, [0_u8]).unwrap();
    assert!(xtask::fixtures::generate(root, xtask::Mode::Check, "upstream-scenarios").is_err());
    xtask::fixtures::generate(root, xtask::Mode::Write, "upstream-scenarios").unwrap();
    xtask::fixtures::generate(root, xtask::Mode::Check, "upstream-scenarios").unwrap();
}

#[test]
fn upstream_scenario_manifest_matches_runners_and_fixture_hashes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let manifest: toml::Value = toml::from_str(
        &fs::read_to_string(root.join("docs/conformance/upstream-scenarios.toml")).unwrap(),
    )
    .unwrap();
    let documented = manifest["scenario"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    let runners = ara2_bridge_testkit::scenarios::upstream_scenarios()
        .iter()
        .map(|scenario| scenario.name)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(documented, runners);

    for fixture in manifest["fixture"].as_array().unwrap() {
        let path = fixture["path"].as_str().unwrap();
        let expected = fixture["sha256"].as_str().unwrap();
        let actual = format!("{:x}", Sha256::digest(fs::read(root.join(path)).unwrap()));
        assert_eq!(actual, expected, "fixture hash mismatch for {path}");
    }
}
