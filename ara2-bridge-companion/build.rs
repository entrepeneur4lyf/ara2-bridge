use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git_output(path: &Path, arguments: &[&str], label: &str) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("could not inspect {label}: {error}"));
    if !output.status.success() {
        panic!(
            "could not inspect {label} with git {}: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn normalize_git_url(value: &str) -> &str {
    value.trim_end_matches('/').trim_end_matches(".git")
}

fn verify_git_identity(path: &Path, label: &str, repository: &str, commit: &str, tree: &str) {
    let actual_commit = git_output(path, &["rev-parse", "HEAD"], label);
    let actual_tree = git_output(path, &["rev-parse", "HEAD^{tree}"], label);
    let actual_repository = git_output(path, &["remote", "get-url", "origin"], label);
    if actual_commit != commit
        || actual_tree != tree
        || normalize_git_url(&actual_repository) != normalize_git_url(repository)
    {
        panic!("{label} does not match its locked repository, commit, and tree");
    }
    if !git_output(
        path,
        &["status", "--porcelain", "--ignore-submodules=none"],
        label,
    )
    .is_empty()
    {
        panic!("{label} is dirty; refusing native companion compilation");
    }
}

fn require_sdk(feature: &str, variable: &str, version: &str) -> Option<PathBuf> {
    env::var_os(feature)?;
    println!("cargo:rerun-if-env-changed={variable}");
    let path = env::var_os(variable).unwrap_or_else(|| {
        panic!(
            "{version} is required when this companion feature is enabled; set {variable} to the locked SDK checkout"
        )
    });
    if !Path::new(&path).is_dir() {
        panic!(
            "{variable} does not name a readable {version} SDK directory: {}",
            Path::new(&path).display()
        );
    }
    Some(PathBuf::from(path))
}

fn require_ara_sdk(feature: &str) -> Option<PathBuf> {
    env::var_os(feature)?;
    println!("cargo:rerun-if-env-changed=ARA_SDK_DIR");
    let path = env::var_os("ARA_SDK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            panic!(
                "ARA_SDK_DIR must point at the project-local SDK installed by scripts/install-ara-sdk.sh"
            )
        });
    let interface = path.join("ARA_API/ARAInterface.h");
    if !interface.is_file() {
        panic!(
            "ARA_SDK_DIR does not contain ARA_API/ARAInterface.h: {}",
            path.display()
        );
    }
    verify_git_identity(
        &path,
        "ARA_SDK_DIR",
        "https://github.com/Celemony/ARA_SDK.git",
        "a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c",
        "305a0dc9ba4759963c1e974353a999c3810b2319",
    );
    verify_git_identity(
        &path.join("ARA_API"),
        "ARA_SDK_DIR/ARA_API",
        "https://github.com/Celemony/ARA_API.git",
        "65ec5c43b943a48cb5446f448a0492db6af8534b",
        "2e3b0455f61314068d34501c5f71407d6ed0051b",
    );
    Some(path)
}

fn require_vst3() {
    let Some(path) = require_sdk(
        "CARGO_FEATURE_VST3",
        "ARA_VST3_SDK_DIR",
        "VST3 SDK v3.8.0_build_66",
    ) else {
        return;
    };
    let required = [
        "pluginterfaces/base/funknown.h",
        "pluginterfaces/base/falignpush.h",
        "pluginterfaces/base/falignpop.h",
    ];
    if let Some(missing) = required
        .iter()
        .find(|relative| !path.join(relative).is_file())
    {
        panic!(
            "ARA_VST3_SDK_DIR must name the locked VST3 SDK v3.8.0_build_66 checkout; missing {missing} under {}",
            path.display()
        );
    }
    verify_git_identity(
        &path,
        "ARA_VST3_SDK_DIR",
        "https://github.com/steinbergmedia/vst3sdk.git",
        "9fad9770f2ae8542ab1a548a68c1ad1ac690abe0",
        "2b7fc6abf314f6a16e57cc8ef71529d74a4ecce9",
    );
    verify_git_identity(
        &path.join("pluginterfaces"),
        "ARA_VST3_SDK_DIR/pluginterfaces",
        "https://github.com/steinbergmedia/vst3_pluginterfaces.git",
        "31d6eeba6daaa3e2a8bfbe3e7a90ca0b7fbfbc1c",
        "ebffbe4eb40bbf1f6fec030feeed30da47e6c719",
    );
    let ara = require_ara_sdk("CARGO_FEATURE_VST3").expect("feature was checked above");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .file("native/vst3/ara_vst3_shim.cpp")
        .include("native/vst3")
        .include(&path)
        .include(ara.join("ARA_API"))
        .warnings(true)
        .flag_if_supported("-fvisibility=hidden");
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        build.flag_if_supported("/EHsc");
    }
    build.compile("ara2_vst3_shim");
    println!("cargo:rerun-if-changed=native/vst3/ara_vst3_shim.hpp");
    println!("cargo:rerun-if-changed=native/vst3/ara_vst3_shim.cpp");
    println!(
        "cargo:rerun-if-changed={}",
        ara.join("ARA_API/ARAVST3.h").display()
    );
}

fn require_audio_unit() {
    if env::var_os("CARGO_FEATURE_AUDIO_UNIT_V2").is_none() {
        return;
    }
    if env::var("CARGO_CFG_TARGET_VENDOR").as_deref() != Ok("apple") {
        panic!("audio-unit-v2 is supported only on Apple targets");
    }
    let path = require_sdk(
        "CARGO_FEATURE_AUDIO_UNIT_V2",
        "ARA_AUDIO_UNIT_SDK_DIR",
        "AudioUnitSDK-1.0.0",
    )
    .expect("feature was checked above");
    if !path.join("Source/AudioUnitSDK.h").is_file() {
        panic!(
            "ARA_AUDIO_UNIT_SDK_DIR must name the locked AudioUnitSDK-1.0.0 checkout; missing Source/AudioUnitSDK.h under {}",
            path.display()
        );
    }
    verify_git_identity(
        &path,
        "ARA_AUDIO_UNIT_SDK_DIR",
        "https://github.com/apple/AudioUnitSDK.git",
        "53ea94e5efebf864b70afb673bdd60c977818ec7",
        "bb8b75ec63fe7d9036073287c4f01bf96d8a49f5",
    );
    let ara = require_ara_sdk("CARGO_FEATURE_AUDIO_UNIT_V2").expect("feature was checked above");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("native/audio_unit/ara_au_shim.mm")
        .include("native/audio_unit")
        .include(path.join("Source"))
        .include(ara.join("ARA_API"))
        .warnings(true)
        .compile("ara2_audio_unit_shim");
    println!("cargo:rustc-link-lib=framework=AudioToolbox");
    println!("cargo:rerun-if-changed=native/audio_unit/ara_au_shim.h");
    println!("cargo:rerun-if-changed=native/audio_unit/ara_au_shim.mm");
    println!(
        "cargo:rerun-if-changed={}",
        ara.join("ARA_API/ARAAudioUnit.h").display()
    );
}

fn main() {
    require_vst3();
    require_audio_unit();
}
