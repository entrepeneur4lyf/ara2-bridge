//! Release-candidate audits, packaging, and evidence verification.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

mod bundle;
mod evidence;

pub(crate) const VERSION: &str = "0.2.0-alpha.1";
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

#[derive(Debug, Deserialize, serde::Serialize)]
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

/// Imports an evidence archive only after an in-process verifier succeeds.
#[doc(hidden)]
pub fn import_evidence_with_verifier<F>(
    root: &Path,
    bundle: &Path,
    repository: &str,
    commit: &str,
    verifier: F,
) -> Result<PathBuf, String>
where
    F: FnOnce(&Path, &str, &str) -> Result<serde_json::Value, String>,
{
    validate_repository_commit(repository, commit)?;
    let digest = sha256_path(bundle)?;
    let verification = verifier(bundle, repository, commit)?;
    if verification
        .as_array()
        .is_none_or(|results| results.is_empty())
    {
        return Err("attestation verifier returned no verified statements".to_owned());
    }
    let receipt = EvidenceReceipt {
        schema: 1,
        repository: repository.to_owned(),
        commit: commit.to_owned(),
        subject_sha256: digest.clone(),
        issuer: "https://token.actions.githubusercontent.com".to_owned(),
        workflow: ".github/workflows/release.yml".to_owned(),
        verified_by: "gh attestation verify".to_owned(),
    };
    let value = serde_json::to_value(&receipt)
        .map_err(|error| format!("cannot serialize evidence receipt: {error}"))?;
    validate_evidence_receipt(&value, commit, &digest)?;
    let directory = root.join("target/release-evidence");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let path = directory.join(format!("receipt-{commit}.json"));
    let temporary = directory.join(format!(".receipt-{commit}.tmp"));
    let mut bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("cannot encode evidence receipt: {error}"))?;
    bytes.push(b'\n');
    fs::write(&temporary, bytes)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &path)
        .map_err(|error| format!("cannot publish {}: {error}", path.display()))?;
    Ok(path)
}

fn validate_repository_commit(repository: &str, commit: &str) -> Result<(), String> {
    if repository != "entrepeneur4lyf/ara2-bridge" {
        return Err("release repository must be entrepeneur4lyf/ara2-bridge".to_owned());
    }
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("release commit must be a 40-character hexadecimal SHA".to_owned());
    }
    Ok(())
}

fn sha256_path(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn import_evidence(
    root: &Path,
    bundle: &Path,
    repository: &str,
    commit: &str,
) -> Result<PathBuf, String> {
    import_evidence_with_verifier(root, bundle, repository, commit, verify_attestation)
}

fn verify_attestation(
    bundle: &Path,
    repository: &str,
    commit: &str,
) -> Result<serde_json::Value, String> {
    let signer = format!("{repository}/.github/workflows/release.yml");
    let output = Command::new("gh")
        .args(["attestation", "verify"])
        .arg(bundle)
        .args(["--repo", repository])
        .args(["--signer-workflow", &signer])
        .args(["--source-digest", commit])
        .args([
            "--cert-oidc-issuer",
            "https://token.actions.githubusercontent.com",
            "--deny-self-hosted-runners",
            "--format",
            "json",
        ])
        .output()
        .map_err(|error| format!("cannot execute gh attestation verify: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "gh attestation verify rejected the evidence archive: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("gh attestation verify returned invalid JSON: {error}"))
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
            verify_source_bundle(&bundle)
        }
        "import-evidence" => {
            let bundle = path_arg(&mut args, "--bundle", "import-evidence")?;
            let repository = string_arg(&mut args, "--repository", "import-evidence")?;
            let commit = string_arg(&mut args, "--commit", "import-evidence")?;
            no_args(args, "import-evidence")?;
            import_evidence(root, &bundle, &repository, &commit).map(|_| ())
        }
        "verify" => {
            let version = version_arg(&mut args)?;
            let commit = string_arg(&mut args, "--commit", "verify")?;
            no_args(args, "verify")?;
            verify_release_at(root, &version, &commit, true)
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

fn string_arg(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
    command: &str,
) -> Result<String, String> {
    if args.next().as_deref() != Some(flag) {
        return Err(format!("release {command} requires {flag} <value>"));
    }
    args.next()
        .ok_or_else(|| format!("release {command} requires {flag} <value>"))
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

/// Verifies one complete release candidate from locally imported evidence.
#[doc(hidden)]
pub fn verify_release_at(
    root: &Path,
    version: &str,
    commit: &str,
    require_clean: bool,
) -> Result<(), String> {
    validate_version(version)?;
    validate_repository_commit("entrepeneur4lyf/ara2-bridge", commit)?;
    if require_clean {
        let output = Command::new("git")
            .current_dir(root)
            .args(["status", "--porcelain", "--untracked-files=no"])
            .output()
            .map_err(|error| format!("cannot inspect release tree: {error}"))?;
        if !output.status.success() {
            return Err("cannot inspect release tree with Git".to_owned());
        }
        if !output.stdout.is_empty() {
            return Err("release verify requires a clean tracked candidate tree".to_owned());
        }
    }
    let directory = root.join("target/release-evidence");
    let archive = directory.join(format!("ara2-evidence-{commit}.tar.zst"));
    let receipt_path = directory.join(format!("receipt-{commit}.json"));
    let digest = sha256_path(&archive)?;
    let receipt: serde_json::Value = serde_json::from_slice(
        &fs::read(&receipt_path)
            .map_err(|error| format!("cannot read {}: {error}", receipt_path.display()))?,
    )
    .map_err(|error| format!("invalid {}: {error}", receipt_path.display()))?;
    validate_evidence_receipt(&receipt, commit, &digest)?;
    evidence::verify_archive(
        &archive,
        commit,
        &root.join("docs/conformance/ci-matrix.md"),
    )
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
        "Generator: ara2-bridge xtask 0.2.0-alpha.1",
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
