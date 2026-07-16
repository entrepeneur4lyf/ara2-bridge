#[cfg(not(windows))]
#[test]
fn generated_bindings_are_current() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    if !root
        .join(".third-party/ARA_SDK/ARA_API/ARAInterface.h")
        .is_file()
    {
        eprintln!("skipping binding freshness without the maintainer ARA SDK checkout");
        return;
    }
    xtask::bindings::generate(xtask::Mode::Check).unwrap();
}

#[cfg(windows)]
#[test]
fn generated_bindings_reject_the_unsupported_windows_host() {
    let error = xtask::bindings::generate(xtask::Mode::Check).unwrap_err();
    assert!(
        error.to_string().contains("not supported on Windows"),
        "{error}"
    );
}

#[test]
fn every_generated_metadata_field_is_required() {
    let complete = concat!(
        "DO NOT EDIT\n",
        "https://github.com/Celemony/ARA_API\n",
        "releases/2.3.0\n",
        "65ec5c43b943a48cb5446f448a0492db6af8534b\n",
        "ara2-bridge xtask 0.3.0\n",
        "Apache-2.0\n",
    );
    xtask::bindings::validate_generated_metadata(complete).unwrap();

    for expected in [
        "DO NOT EDIT",
        "https://github.com/Celemony/ARA_API",
        "releases/2.3.0",
        "65ec5c43b943a48cb5446f448a0492db6af8534b",
        "ara2-bridge xtask 0.3.0",
        "Apache-2.0",
    ] {
        let altered = complete.replace(expected, "");
        let error = xtask::bindings::validate_generated_metadata(&altered).unwrap_err();
        assert!(error.to_string().contains("metadata field"));
    }
}

#[test]
fn raw_callbacks_are_nullable_and_enums_are_integer_aliases() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let bindings =
        std::fs::read_to_string(root.join("ara2-bridge-sys/src/generated/x86_64.rs")).unwrap();
    let callbacks = bindings.matches("unsafe extern \"C\" fn").count();
    let nullable = bindings.matches("::std::option::Option<").count();
    assert!(callbacks > 50, "unexpectedly small callback inventory");
    assert_eq!(callbacks, nullable, "every callback must remain nullable");
    assert!(!bindings.contains("pub enum "));
}

#[test]
fn symbol_coverage_is_unique_classified_and_source_spanned() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("ara2-bridge-sys/generated/symbol-coverage.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["metadata"]["normative_commit"],
        "65ec5c43b943a48cb5446f448a0492db6af8534b"
    );

    let records = manifest["records"].as_array().unwrap();
    assert!(records.len() > 450, "core inventory is unexpectedly small");
    let mut keys = std::collections::BTreeSet::new();
    for record in records {
        let key = (
            record["header"].as_str().unwrap(),
            record["kind"].as_str().unwrap(),
            record["symbol"].as_str().unwrap(),
        );
        assert!(keys.insert(key), "duplicate coverage record: {key:?}");
        assert!(record["span"]["start_line"].as_u64().unwrap() > 0);
        assert!(record["span"]["start_column"].as_u64().unwrap() > 0);
        match record["classification"].as_str().unwrap() {
            "core-abi" => assert!(record["required_sdks"].as_array().unwrap().is_empty()),
            "companion-deferred" => {
                assert_eq!(record["required_sdks"].as_array().unwrap().len(), 1)
            }
            other => panic!("unknown classification: {other}"),
        }
    }
}
