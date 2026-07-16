fn main() {
    if std::env::var_os("CARGO_FEATURE_CLAP").is_some() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("testkit is a workspace child");
        cc::Build::new()
            .file("native/clap_probe.c")
            .include(root.join(".third-party/clap/include"))
            .include(root.join(".third-party/ARA_SDK/ARA_API"))
            .warnings(true)
            .compile("ara2_clap_probe");
        println!("cargo:rerun-if-changed=native/clap_probe.c");
        println!("cargo:rerun-if-changed=../.third-party/ARA_SDK/ARA_API/ARACLAP.h");
    }

    if std::env::var_os("CARGO_FEATURE_CPP_INTEROP").is_some() {
        build_cpp_interop();
    }
}

fn build_cpp_interop() {
    let sdk = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("testkit is a workspace child")
        .join(".third-party/ARA_SDK");
    for required in ["ARA_API", "ARA_Library", "ARA_Examples"] {
        assert!(
            sdk.join(required).is_dir(),
            "the GitHub-provisioned ARA SDK is missing {required}; run ci/bootstrap-reference-sdks.sh fetch --component ara --accept-license Apache-2.0 (expected {})",
            sdk.display()
        );
    }

    let examples = sdk.join("ARA_Examples");
    let library = sdk.join("ARA_Library");
    let cpp_sources = [
        "ARA_Library/Utilities/ARAChannelFormat.cpp",
        "ARA_Library/Utilities/ARAPitchInterpretation.cpp",
        "ARA_Library/Dispatch/ARAHostDispatch.cpp",
        "ARA_Library/Dispatch/ARAPlugInDispatch.cpp",
        "ARA_Library/PlugIn/ARAPlug.cpp",
        "ARA_Examples/3rdParty/cpp-base64/base64.cpp",
        "ARA_Examples/3rdParty/ICST_AudioFile/AudioFile.cpp",
        "ARA_Examples/3rdParty/pugixml/src/pugixml.cpp",
        "ARA_Examples/ExamplesCommon/Archives/Archives.cpp",
        "ARA_Examples/ExamplesCommon/AudioFiles/AudioFiles.cpp",
        "ARA_Examples/TestPlugIn/ARATestAudioSource.cpp",
        "ARA_Examples/TestPlugIn/ARATestDocumentController.cpp",
        "ARA_Examples/TestPlugIn/ARATestPlaybackRenderer.cpp",
        "ARA_Examples/TestPlugIn/TestAnalysis.cpp",
        "ARA_Examples/TestPlugIn/TestPersistency.cpp",
        "ARA_Examples/TestHost/ARAHostInterfaces/ARAArchivingController.cpp",
        "ARA_Examples/TestHost/ARAHostInterfaces/ARAAudioAccessController.cpp",
        "ARA_Examples/TestHost/ARAHostInterfaces/ARAContentAccessController.cpp",
        "ARA_Examples/TestHost/ARAHostInterfaces/ARAModelUpdateController.cpp",
        "ARA_Examples/TestHost/ARAHostInterfaces/ARAPlaybackController.cpp",
        "ARA_Examples/TestHost/ARADocumentController.cpp",
        "ARA_Examples/TestHost/CompanionAPIs.cpp",
        "ARA_Examples/TestHost/ModelObjects.cpp",
        "ARA_Examples/TestHost/TestHost.cpp",
        "ARA_Examples/TestHost/TestCases.cpp",
    ];
    let c_sources = [
        "ARA_Library/Debug/ARADebug.c",
        "ARA_Examples/ExamplesCommon/SignalProcessing/PulsedSineSignal.c",
    ];
    let includes = [
        sdk.clone(),
        examples.clone(),
        examples.join("3rdParty/cpp-base64"),
        examples.join("3rdParty/ICST_AudioFile"),
        examples.join("3rdParty/pugixml/src"),
        examples.join("TestHost"),
        examples.join("TestPlugIn"),
        library
            .parent()
            .expect("ARA_Library has an SDK parent")
            .to_path_buf(),
    ];

    let mut cpp = cc::Build::new();
    cpp.cpp(true)
        .std("c++17")
        .warnings(false)
        .define("ARA_VALIDATE_API_CALLS", "1")
        .define("ARA_ENABLE_INTERNAL_ASSERTS", "1")
        .define("ARA_ENABLE_DEBUG_OUTPUT", "1")
        .define("ARA_MAJOR_VERSION", "2")
        .define("ARA_MINOR_VERSION", "3")
        .define("ARA_PATCH_VERSION", "0")
        .define("ARA_ENABLE_VST3", "0")
        .define("ARA_ENABLE_CLAP", "0")
        .define("ARA_ENABLE_IPC", "0")
        .file("native/test_host_bridge.cpp")
        .file("native/test_plugin_bridge.cpp");
    let prelude = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("native/cpp_prelude.h");
    let target = std::env::var("TARGET").expect("Cargo always provides TARGET to build scripts");
    if target.contains("msvc") {
        cpp.flag(format!("/FI{}", prelude.display()));
        cpp.flag_if_supported("/EHsc");
        cpp.define("NOMINMAX", "1");
    } else {
        cpp.flag(format!("-include{}", prelude.display()));
    }
    if target.contains("apple-darwin") {
        cpp.flag("-mmacosx-version-min=11.0");
    }
    for include in &includes {
        cpp.include(include);
    }
    for source in &cpp_sources {
        cpp.file(sdk.join(source));
    }
    cpp.compile("ara2_cpp_interop");

    let mut c = cc::Build::new();
    c.warnings(false)
        .define("ARA_VALIDATE_API_CALLS", "1")
        .define("ARA_ENABLE_INTERNAL_ASSERTS", "1")
        .define("ARA_ENABLE_DEBUG_OUTPUT", "1");
    if target.contains("apple-darwin") {
        c.flag("-mmacosx-version-min=11.0");
    }
    for include in &includes {
        c.include(include);
    }
    for source in &c_sources {
        c.file(sdk.join(source));
    }
    c.compile("ara2_cpp_interop_c");

    for local in [
        "native/cpp_interop.h",
        "native/cpp_prelude.h",
        "native/test_host_bridge.cpp",
        "native/test_plugin_bridge.cpp",
    ] {
        println!("cargo:rerun-if-changed={local}");
    }
    for source in cpp_sources.iter().chain(c_sources.iter()) {
        println!("cargo:rerun-if-changed={}", sdk.join(source).display());
    }
    if cfg!(all(unix, not(target_os = "macos"))) {
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=dl");
    }
    if target.contains("apple-darwin") {
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
}
