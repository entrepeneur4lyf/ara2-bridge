use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Clone, Copy)]
struct Case<'a> {
    name: &'a str,
    default_features: bool,
    features: &'a [&'a str],
    modules: &'a [&'a str],
}

#[test]
fn facade_feature_matrix_compiles_in_isolated_consumers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let cases = [
        Case {
            name: "default",
            default_features: true,
            features: &[],
            modules: &["plugin"],
        },
        Case {
            name: "no-default-features",
            default_features: false,
            features: &[],
            modules: &[],
        },
        Case {
            name: "plugin",
            default_features: false,
            features: &["plugin"],
            modules: &["plugin"],
        },
        Case {
            name: "host",
            default_features: false,
            features: &["host"],
            modules: &["host"],
        },
        Case {
            name: "clap",
            default_features: false,
            features: &["clap"],
            modules: &["companion"],
        },
        Case {
            name: "testkit",
            default_features: false,
            features: &["testkit"],
            modules: &["testkit"],
        },
        Case {
            name: "plugin-host",
            default_features: false,
            features: &["plugin", "host"],
            modules: &["plugin", "host"],
        },
    ];

    let mut failures = Vec::new();
    for case in cases {
        let output = run_case(root, case);
        if !output.status.success() {
            failures.push(format!(
                "{} failed:\n{}",
                case.name,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    let vst3_cases = [
        Case {
            name: "vst3",
            default_features: false,
            features: &["vst3"],
            modules: &["companion"],
        },
        Case {
            name: "full-portable",
            default_features: false,
            features: &["full-portable"],
            modules: &["plugin", "host", "companion"],
        },
    ];
    if sdk_is_configured(root, "ARA_VST3_SDK_DIR", ".third-party/vst3sdk") {
        for case in vst3_cases {
            let output = run_case(root, case);
            if !output.status.success() {
                failures.push(format!(
                    "{} failed:\n{}",
                    case.name,
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
    } else {
        for case in vst3_cases {
            assert_missing_vst3_error(root, case);
        }
    }

    if cfg!(target_vendor = "apple") {
        let output = run_case(
            root,
            Case {
                name: "audio-unit-v2",
                default_features: false,
                features: &["audio-unit-v2"],
                modules: &["companion"],
            },
        );
        if !output.status.success() {
            failures.push(format!(
                "audio-unit-v2 failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let output = run_case(
            root,
            Case {
                name: "full-apple",
                default_features: false,
                features: &["full-apple"],
                modules: &["plugin", "host", "companion"],
            },
        );
        if !output.status.success() {
            failures.push(format!(
                "full-apple failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    } else {
        assert_non_apple_audio_unit_error(root, "audio-unit-v2", &["audio-unit-v2"]);
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn native_build_scripts_resolve_ara_from_the_consuming_project() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for relative in [
        "ara2-bridge-companion/build.rs",
        "ara2-bridge-testkit/build.rs",
    ] {
        let source = fs::read_to_string(root.join(relative)).unwrap();
        assert!(
            source.contains("cargo:rerun-if-env-changed=ARA_SDK_DIR"),
            "{relative} does not track the consuming project's ARA SDK configuration"
        );
        assert!(
            !source.contains("reference/ARA_SDK")
                && !source.contains("root.join(\".third-party/ARA_SDK"),
            "{relative} assumes an SDK inside the ara2-bridge source tree"
        );
    }
}

fn assert_non_apple_audio_unit_error(root: &Path, name: &str, features: &[&str]) {
    let output = run_case(
        root,
        Case {
            name,
            default_features: false,
            features,
            modules: &["companion"],
        },
    );
    assert!(!output.status.success(), "{name} unexpectedly compiled");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("audio-unit-v2 is supported only on Apple targets"),
        "{name} produced the wrong diagnostic:\n{stderr}"
    );
}

fn assert_missing_vst3_error(root: &Path, case: Case<'_>) {
    let output = run_case(root, case);
    assert!(
        !output.status.success(),
        "{} unexpectedly compiled",
        case.name
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "VST3 SDK v3.8.0_build_66 is required when this companion feature is enabled"
        ),
        "{} produced the wrong diagnostic:\n{stderr}",
        case.name
    );
}

fn run_case(root: &Path, case: Case<'_>) -> Output {
    let directory = root.join("target/feature-matrix").join(case.name);
    let source = directory.join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(directory.join("Cargo.toml"), manifest(root, case)).unwrap();
    fs::write(source.join("lib.rs"), consumer_source(case.modules)).unwrap();

    let mut command = Command::new(env!("CARGO"));
    command
        .arg("check")
        .arg("--quiet")
        .arg("--offline")
        .current_dir(&directory)
        .env(
            "CARGO_TARGET_DIR",
            root.join("target/feature-matrix-target"),
        );
    configure_sdk(
        &mut command,
        "ARA_SDK_DIR",
        root.join(".third-party/ARA_SDK"),
    );
    configure_sdk(&mut command, "ARA_CLAP_DIR", root.join(".third-party/clap"));
    configure_sdk(
        &mut command,
        "ARA_VST3_SDK_DIR",
        root.join(".third-party/vst3sdk"),
    );
    configure_sdk(
        &mut command,
        "ARA_AUDIO_UNIT_SDK_DIR",
        root.join(".third-party/AudioUnitSDK"),
    );
    command.output().unwrap()
}

fn configure_sdk(command: &mut Command, variable: &str, fallback: PathBuf) {
    if let Some(value) = std::env::var_os(variable) {
        command.env(variable, value);
    } else if fallback.is_dir() {
        command.env(variable, fallback);
    }
}

fn sdk_is_configured(root: &Path, variable: &str, fallback: &str) -> bool {
    std::env::var_os(variable).is_some() || root.join(fallback).is_dir()
}

fn manifest(root: &Path, case: Case<'_>) -> String {
    let features = case
        .features
        .iter()
        .map(|feature| format!("\"{feature}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "[package]\nname = \"facade-case-{}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies.ara2-bridge]\npath = {:?}\ndefault-features = {}\nfeatures = [{}]\n",
        case.name,
        root.join("ara2-bridge"),
        case.default_features,
        features
    )
}

fn consumer_source(modules: &[&str]) -> String {
    let mut source = String::from(
        "#![allow(unused_imports)]\nuse ara2_bridge::core as _core;\nuse ara2_bridge::sys as _sys;\n",
    );
    for module in modules {
        source.push_str(&format!("use ara2_bridge::{module} as _{module};\n"));
    }
    source.push_str("pub fn consumer_compiles() {}\n");
    source
}
