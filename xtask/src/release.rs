//! Release-candidate audits, packaging, and evidence verification.

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) mod bundle;

pub(crate) const VERSION: &str = "0.3.0";
pub(crate) const PACKAGES: &[&str] = &[
    "ara2-bridge-sys",
    "ara2-bridge-core",
    "ara2-bridge-plugin",
    "ara2-bridge-host",
    "ara2-bridge-companion",
    "ara2-bridge-testkit",
    "ara2-bridge",
];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Recipe {
    pub(crate) schema: u32,
    pub(crate) version: String,
    pub(crate) repository: String,
    pub(crate) packages: Vec<String>,
    pub(crate) required_files: Vec<String>,
    pub(crate) required_trees: Vec<String>,
}

/// Runs the `cargo xtask release` command family.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let command = args
        .next()
        .ok_or_else(|| "release requires a command or --help".to_owned())?;
    if command == "--help" {
        return Ok(());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child");
    match command.as_str() {
        "audit-api" => no_args(args, "audit-api").and_then(|_| audit_api(root)),
        "audit-unsafe" => no_args(args, "audit-unsafe").and_then(|_| audit_unsafe(root)),
        "audit-licenses" => no_args(args, "audit-licenses").and_then(|_| audit_licenses(root)),
        "verify-source-inputs" => {
            let version = version_arg(&mut args)?;
            no_args(args, "verify-source-inputs")?;
            validate_version(&version)?;
            verify_source_inputs(root)
        }
        "source-bundle" => {
            let version = version_arg(&mut args)?;
            let output = path_arg(&mut args, "--output", "source-bundle")?;
            no_args(args, "source-bundle")?;
            validate_version(&version)?;
            create_source_bundle(root, &output, false)
        }
        "verify-source-bundle" => {
            let bundle = path_arg(&mut args, "--bundle", "verify-source-bundle")?;
            no_args(args, "verify-source-bundle")?;
            let commit = current_commit(root)?;
            bundle::verify_for_commit(&bundle, &commit)
        }
        _ => Err(format!("unknown release command: {command}")),
    }
}

fn no_args(mut args: impl Iterator<Item = String>, command: &str) -> Result<(), String> {
    if args.next().is_some() {
        Err(format!("release {command} takes no arguments"))
    } else {
        Ok(())
    }
}

fn version_arg(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    if args.next().as_deref() != Some("--version") {
        return Err(format!("release command requires --version {VERSION}"));
    }
    args.next()
        .ok_or_else(|| format!("release command requires --version {VERSION}"))
}

fn path_arg(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
    command: &str,
) -> Result<PathBuf, String> {
    if args.next().as_deref() != Some(flag) {
        return Err(format!("release {command} requires {flag} <path>"));
    }
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("release {command} requires {flag} <path>"))
}

fn validate_version(version: &str) -> Result<(), String> {
    if version == VERSION {
        Ok(())
    } else {
        Err(format!("release version must be exactly {VERSION}"))
    }
}

fn current_commit(root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("cannot inspect release candidate commit: {error}"))?;
    if !output.status.success() {
        return Err("cannot inspect release candidate commit with Git".to_owned());
    }
    String::from_utf8(output.stdout)
        .map(|commit| commit.trim().to_owned())
        .map_err(|error| format!("release candidate commit is not UTF-8: {error}"))
}

/// Validates the closed source-bundle recipe and every tracked input path.
pub fn verify_recipe(root: &Path, path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let recipe: Recipe = toml::from_str(&source)
        .map_err(|error| format!("invalid source-bundle recipe: {error}"))?;
    if recipe.schema != 1 || recipe.version != VERSION {
        return Err(format!(
            "source recipe must use schema 1 and version {VERSION}"
        ));
    }
    if recipe.repository != "entrepeneur4lyf/ara2-bridge" {
        return Err("source recipe repository mismatch".to_owned());
    }
    if recipe.packages != PACKAGES {
        return Err("source recipe package order mismatch".to_owned());
    }
    for relative in recipe.required_files {
        if !safe_relative(&relative) || !root.join(&relative).is_file() {
            return Err(format!("missing source-bundle input {relative}"));
        }
    }
    for relative in recipe.required_trees {
        if !safe_relative(&relative) || !root.join(&relative).is_dir() {
            return Err(format!("missing source-bundle tree {relative}"));
        }
    }
    Ok(())
}

fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && !Path::new(value).is_absolute()
        && Path::new(value)
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
}

/// Builds the deterministic seven-crate source bundle.
///
/// `allow_dirty` is reserved for the precommit `verify-source-inputs` path. Release artifacts
/// must pass `false` so their metadata binds to a clean candidate commit.
#[doc(hidden)]
pub fn create_source_bundle(root: &Path, output: &Path, allow_dirty: bool) -> Result<(), String> {
    bundle::create(root, output, allow_dirty)
}

/// Verifies the clean tracked-tree precondition used by candidate packaging.
#[doc(hidden)]
pub fn verify_clean_candidate(root: &Path) -> Result<(), String> {
    bundle::require_clean_tree(root)
}

/// Verifies a source bundle without network or ambient Cargo state.
#[doc(hidden)]
pub fn verify_source_bundle(bundle: &Path) -> Result<(), String> {
    bundle::verify(bundle)
}

fn verify_source_inputs(root: &Path) -> Result<(), String> {
    verify_recipe(root, &root.join("docs/releases/source-bundle.toml"))?;
    fs::create_dir_all(root.join("target"))
        .map_err(|error| format!("cannot create target directory: {error}"))?;
    let temp = tempfile::Builder::new()
        .prefix("ara2-source-preflight-")
        .tempdir_in(root.join("target"))
        .map_err(|error| format!("cannot create source preflight directory: {error}"))?;
    let bundle = temp
        .path()
        .join(format!("ara2-bridge-{VERSION}-source.tar.zst"));
    create_source_bundle(root, &bundle, true)?;
    verify_source_bundle(&bundle)
}

/// Rejects generated derivatives without complete source and generator provenance.
pub fn validate_generated_derivative(source: &str) -> Result<(), String> {
    for required in [
        "DO NOT EDIT",
        "Source repository: https://github.com/Celemony/ARA_API",
        "Source tag: releases/2.3.0",
        "Normative ARA API commit: 65ec5c43b943a48cb5446f448a0492db6af8534b",
        "Generator: ara2-bridge xtask 0.3.0",
        "SPDX-License-Identifier: Apache-2.0",
    ] {
        if !source.contains(required) {
            return Err(format!("generated derivative missing {required}"));
        }
    }
    Ok(())
}

/// Validates normalized package metadata and sibling dependency versions.
#[doc(hidden)]
pub fn validate_packaged_manifest(_source: &str, _expected_name: &str) -> Result<(), String> {
    let value: toml::Value =
        toml::from_str(_source).map_err(|error| format!("invalid packaged Cargo.toml: {error}"))?;
    let package = value
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "packaged Cargo.toml is missing [package]".to_owned())?;
    for (field, expected) in [
        ("name", _expected_name),
        ("version", VERSION),
        ("edition", "2021"),
        ("rust-version", "1.82"),
        ("license", "MIT OR Apache-2.0"),
        (
            "repository",
            "https://github.com/entrepeneur4lyf/ara2-bridge",
        ),
    ] {
        let actual = package
            .get(field)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("packaged Cargo.toml is missing package.{field}"))?;
        if actual != expected {
            return Err(format!(
                "packaged Cargo.toml package.{field} mismatch: expected {expected}, found {actual}"
            ));
        }
    }
    for section in ["dependencies", "build-dependencies", "dev-dependencies"] {
        let Some(dependencies) = value.get(section).and_then(toml::Value::as_table) else {
            continue;
        };
        for (name, dependency) in dependencies {
            if !name.starts_with("ara2-bridge") {
                continue;
            }
            let table = dependency.as_table().ok_or_else(|| {
                format!("packaged {section}.{name} must use an explicit dependency table")
            })?;
            let version = table
                .get("version")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| format!("packaged {section}.{name} is missing version"))?;
            if version != format!("={VERSION}") || table.contains_key("path") {
                return Err(format!(
                    "packaged {section}.{name} must use exact registry version ={VERSION}"
                ));
            }
        }
    }
    Ok(())
}

/// Audits checked-in coverage and compatibility artifacts.
pub fn audit_api(root: &Path) -> Result<(), String> {
    crate::bindings::generate(crate::Mode::Check)
        .map_err(|error| format!("raw ABI generation audit failed: {error}"))?;
    crate::compatibility::generate(crate::Mode::Check)
        .map_err(|error| format!("compatibility audit failed: {error}"))?;
    crate::coverage::generate(root, crate::Mode::Check)
        .map_err(|error| format!("semantic coverage audit failed: {error}"))?;
    Ok(())
}

/// Audits the source tree's unsafe-comment policy.
pub fn audit_unsafe(root: &Path) -> Result<(), String> {
    crate::docs::verify_public_docs_path(root)
}

/// Audits project, upstream, and companion license inputs.
pub fn audit_licenses(root: &Path) -> Result<(), String> {
    for relative in [
        "LICENSE",
        "LICENSE-MIT",
        "LICENSES/ARA-SDK-Apache-2.0.txt",
        "LICENSES/third-party.md",
        "sdk-provenance.toml",
        "ci/reference-sdks.lock.toml",
    ] {
        let metadata = fs::metadata(root.join(relative))
            .map_err(|error| format!("missing license input {relative}: {error}"))?;
        if metadata.len() == 0 {
            return Err(format!("empty license input {relative}"));
        }
    }
    for package in PACKAGES {
        let manifest = fs::read_to_string(root.join(package).join("Cargo.toml"))
            .map_err(|error| format!("cannot read {package}/Cargo.toml: {error}"))?;
        if !manifest.contains("license.workspace = true") {
            return Err(format!("{package} does not inherit the workspace license"));
        }
    }
    Ok(())
}
