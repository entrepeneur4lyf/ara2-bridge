use std::path::Path;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

#[test]
fn sys_package_has_no_build_time_binding_generation() {
    let root = repository_root();
    let manifest = std::fs::read_to_string(root.join("ara2-bridge-sys/Cargo.toml")).unwrap();

    assert!(!manifest.contains("[build-dependencies]"));
    assert!(!manifest.contains("bindgen"));
    assert!(!root.join("ara2-bridge-sys/build.rs").exists());
    for header in [
        "ARAInterface.h",
        "ARAAudioFileChunks.h",
        "ARACLAP.h",
        "ARAVST3.h",
        "ARAAudioUnit.h",
    ] {
        assert!(
            !root.join("ara2-bridge-sys").join(header).exists(),
            "SDK header must not be copied into the package: {header}"
        );
    }
}

#[test]
fn sys_public_boundary_is_generated_and_target_explicit() {
    let root = repository_root();
    let library = std::fs::read_to_string(root.join("ara2-bridge-sys/src/lib.rs")).unwrap();
    let selector =
        std::fs::read_to_string(root.join("ara2-bridge-sys/src/generated/mod.rs")).unwrap();

    for constant in [
        "ARA_SOURCE_REPOSITORY",
        "ARA_SOURCE_TAG",
        "ARA_API_COMMIT",
        "ARA_SDK_COMMIT",
    ] {
        assert!(library.contains(constant), "missing {constant}");
    }
    assert!(library.contains("compile_error!"));
    assert!(library.contains("target_arch = \"arm\""));
    assert!(library.contains("unsupported target architecture"));

    for module in ["x86_64.rs", "aarch64.rs", "i686.rs"] {
        assert!(selector.contains(module), "missing selector for {module}");
    }
    assert!(selector.contains("pub use target::*"));
}
