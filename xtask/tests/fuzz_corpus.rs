use std::fs;

#[test]
fn generation_rejects_missing_stale_extra_empty_and_unlicensed_seeds() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    assert!(xtask::fuzz_corpus::generate(root, xtask::Mode::Check).is_err());

    let stale = root.join("fuzz/corpus/versioned_structs/generation-1.bin");
    fs::create_dir_all(stale.parent().unwrap()).unwrap();
    fs::write(&stale, [0_u8]).unwrap();
    assert!(xtask::fuzz_corpus::generate(root, xtask::Mode::Check).is_err());

    xtask::fuzz_corpus::generate(root, xtask::Mode::Write).unwrap();
    xtask::fuzz_corpus::generate(root, xtask::Mode::Check).unwrap();

    let extra = root.join("fuzz/corpus/references/extra.bin");
    fs::write(&extra, b"extra").unwrap();
    assert!(xtask::fuzz_corpus::generate(root, xtask::Mode::Check).is_err());
    fs::remove_file(extra).unwrap();

    fs::write(&stale, []).unwrap();
    assert!(xtask::fuzz_corpus::generate(root, xtask::Mode::Check).is_err());

    xtask::fuzz_corpus::generate(root, xtask::Mode::Write).unwrap();
    let manifest = root.join("fuzz/corpus-manifest.toml");
    let mut document: toml::Value =
        toml::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
    document["seed"][0]["source_license"] = toml::Value::String(String::new());
    fs::write(manifest, toml::to_string_pretty(&document).unwrap()).unwrap();
    assert!(xtask::fuzz_corpus::generate(root, xtask::Mode::Check).is_err());
}

#[test]
fn command_router_registers_fuzz_corpus_write_and_check() {
    assert!(xtask::ara::run(["fuzz-corpus".to_owned(), "--help".to_owned()]).is_ok());
}

#[test]
fn repository_corpus_is_fresh_complete_and_nonempty() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    xtask::fuzz_corpus::generate(root, xtask::Mode::Check).unwrap();

    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(root.join("fuzz/corpus-manifest.toml")).unwrap())
            .unwrap();
    let seeds = manifest["seed"].as_array().unwrap();
    let paths = seeds
        .iter()
        .map(|seed| seed["path"].as_str().unwrap())
        .collect::<Vec<_>>();
    let tracked = std::process::Command::new("git")
        .current_dir(root)
        .args(["ls-files", "--error-unmatch", "--"])
        .args(&paths)
        .output()
        .unwrap();
    assert!(
        tracked.status.success(),
        "canonical fuzz seeds must be tracked:\n{}",
        String::from_utf8_lossy(&tracked.stderr)
    );

    let targets = seeds
        .iter()
        .map(|seed| seed["target"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        targets,
        [
            "archive_filters",
            "audio_file_chunks",
            "audio_file_container",
            "audio_file_xml",
            "content_events",
            "dispatch",
            "references",
            "versioned_structs",
        ]
        .into_iter()
        .collect()
    );
}
