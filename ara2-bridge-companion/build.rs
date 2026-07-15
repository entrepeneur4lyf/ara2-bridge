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

fn require_vst3() {
    let Some(path) = require_sdk(
        "CARGO_FEATURE_VST3",
        "ARA_VST3_SDK_DIR",
        "VST3 SDK v3.7.11_build_10",
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
            "ARA_VST3_SDK_DIR must name the locked VST3 SDK v3.7.11_build_10 checkout; missing {missing} under {}",
            path.display()
        );
    }
    verify_git_identity(
        &path,
        "ARA_VST3_SDK_DIR",
        "https://github.com/steinbergmedia/vst3sdk.git",
        "7d92338ae922db2d559ac458824a4df40f37e82e",
        "1509aeb4426b5e51ff7586b8096f2ab8ccbbd2a4",
    );
    verify_git_identity(
        &path.join("pluginterfaces"),
        "ARA_VST3_SDK_DIR/pluginterfaces",
        "https://github.com/steinbergmedia/vst3_pluginterfaces.git",
        "cf187058ae79fe557fd854f2cc66a2b587d8cbe9",
        "f4aebd86e7bff7b0d6d653715ddc1cce202adbf4",
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("companion crate is a workspace child");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("native/vst3/ara_vst3_shim.cpp")
        .include("native/vst3")
        .include(&path)
        .include(root.join("reference/ARA_SDK/ARA_API"))
        .warnings(true)
        .flag_if_supported("-fvisibility=hidden")
        .compile("ara2_vst3_shim");
    println!("cargo:rerun-if-changed=native/vst3/ara_vst3_shim.hpp");
    println!("cargo:rerun-if-changed=native/vst3/ara_vst3_shim.cpp");
    println!("cargo:rerun-if-changed=../reference/ARA_SDK/ARA_API/ARAVST3.h");
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("companion crate is a workspace child");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("native/audio_unit/ara_au_shim.mm")
        .include("native/audio_unit")
        .include(path.join("Source"))
        .include(root.join("reference/ARA_SDK/ARA_API"))
        .warnings(true)
        .compile("ara2_audio_unit_shim");
    println!("cargo:rustc-link-lib=framework=AudioToolbox");
    println!("cargo:rerun-if-changed=native/audio_unit/ara_au_shim.h");
    println!("cargo:rerun-if-changed=native/audio_unit/ara_au_shim.mm");
    println!("cargo:rerun-if-changed=../reference/ARA_SDK/ARA_API/ARAAudioUnit.h");
}

fn main() {
    require_vst3();
    require_audio_unit();
}
