//! Release-candidate audits, packaging, and evidence verification.

use serde::Deserialize;
use std::fs;
use std::path::Path;

const VERSION: &str = "0.2.0-alpha.1";
const PACKAGES: &[&str] = &[
    "ara2-bridge-sys",
    "ara2-bridge-core",
    "ara2-bridge-plugin",
    "ara2-bridge-host",
    "ara2-bridge-companion",
    "ara2-bridge-testkit",
    "ara2-bridge",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    schema: u32,
    version: String,
    repository: String,
    packages: Vec<String>,
    required_files: Vec<String>,
    required_trees: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceReceipt {
    schema: u32,
    repository: String,
    commit: String,
    subject_sha256: String,
    issuer: String,
    workflow: String,
    verified_by: String,
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
            verify_recipe(root, &root.join("docs/releases/source-bundle.toml"))
        }
        "source-bundle" | "verify-source-bundle" | "import-evidence" | "verify" => Err(format!(
            "release {command} implementation is pending clean-room tests"
        )),
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

fn validate_version(version: &str) -> Result<(), String> {
    if version == VERSION {
        Ok(())
    } else {
        Err(format!("release version must be exactly {VERSION}"))
    }
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

/// Rejects generated derivatives without complete source and generator provenance.
pub fn validate_generated_derivative(source: &str) -> Result<(), String> {
    for required in [
        "DO NOT EDIT",
        "Source repository:",
        "Source tag:",
        "Normative ARA API commit:",
        "Generator: ara2-bridge xtask 0.2.0-alpha.1",
        "SPDX-License-Identifier: Apache-2.0",
    ] {
        if !source.contains(required) {
            return Err(format!("generated derivative missing {required}"));
        }
    }
    Ok(())
}

/// Validates the identity fields produced only after external attestation verification.
pub fn validate_evidence_receipt(
    value: &serde_json::Value,
    expected_commit: &str,
    expected_digest: &str,
) -> Result<(), String> {
    let receipt: EvidenceReceipt = serde_json::from_value(value.clone())
        .map_err(|error| format!("invalid evidence receipt: {error}"))?;
    for (field, actual, expected) in [
        (
            "repository",
            receipt.repository.as_str(),
            "entrepeneur4lyf/ara2-bridge",
        ),
        ("commit", receipt.commit.as_str(), expected_commit),
        (
            "subject_sha256",
            receipt.subject_sha256.as_str(),
            expected_digest,
        ),
        (
            "issuer",
            receipt.issuer.as_str(),
            "https://token.actions.githubusercontent.com",
        ),
        (
            "workflow",
            receipt.workflow.as_str(),
            ".github/workflows/release.yml",
        ),
        (
            "verified_by",
            receipt.verified_by.as_str(),
            "gh attestation verify",
        ),
    ] {
        if actual != expected {
            return Err(format!(
                "evidence receipt {field} mismatch: expected {expected}, found {actual}"
            ));
        }
    }
    if receipt.schema != 1 {
        return Err("evidence receipt schema mismatch".to_owned());
    }
    Ok(())
}

/// Audits checked-in coverage and compatibility artifacts.
pub fn audit_api(root: &Path) -> Result<(), String> {
    for relative in [
        "ara2-bridge-sys/generated/symbol-coverage.json",
        "docs/conformance/interface-coverage.json",
        "docs/specs/ara2-bridge/api-compatibility.toml",
    ] {
        let source = fs::read_to_string(root.join(relative))
            .map_err(|error| format!("cannot read {relative}: {error}"))?;
        if relative.ends_with(".json") {
            serde_json::from_str::<serde_json::Value>(&source)
                .map_err(|error| format!("invalid {relative}: {error}"))?;
        } else {
            toml::from_str::<toml::Value>(&source)
                .map_err(|error| format!("invalid {relative}: {error}"))?;
        }
    }
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
