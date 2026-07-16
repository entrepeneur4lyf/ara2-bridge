use std::path::PathBuf;

fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("ara2-bridge-{name}-{}", std::process::id()))
}

#[test]
fn companion_probe_command_routes_help_and_rejects_unknown_components() {
    assert!(xtask::run(["ara", "companion-probe", "--help"].map(str::to_owned)).is_ok());
    let error = xtask::run(["ara", "companion-probe", "unknown", "--check-all"].map(str::to_owned))
        .unwrap_err();
    assert!(error.contains("unknown companion component"));
}

#[test]
fn clap_probe_emit_import_and_check_are_deterministic() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let envelope = temporary("clap-probe.json");
    let import = temporary("clap-probes");
    let _ = std::fs::remove_dir_all(&import);
    std::fs::create_dir_all(&import).unwrap();

    xtask::companion_probe::emit(&root, "clap", &envelope, "x86_64-unknown-linux-gnu").unwrap();
    std::fs::copy(&envelope, import.join("clap-x86_64.probe.json")).unwrap();
    xtask::companion_probe::validate_envelope(&root, "clap", &envelope).unwrap();
    let duplicate = import.join("duplicate.probe.json");
    std::fs::copy(&envelope, &duplicate).unwrap();
    assert!(xtask::companion_probe::import_dir(&root, "clap", &import).is_err());
    std::fs::remove_file(duplicate).unwrap();

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    xtask::companion_probe::check_target(&root, "clap", "x86_64-unknown-linux-gnu").unwrap();

    let _ = std::fs::remove_file(envelope);
    let _ = std::fs::remove_dir_all(import);
}
