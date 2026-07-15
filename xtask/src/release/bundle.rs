use super::{
    validate_generated_derivative, validate_packaged_manifest, verify_recipe, Recipe, PACKAGES,
    VERSION,
};
use flate2::{read::GzDecoder, Compression, GzBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

const RECIPE_PATH: &str = "docs/releases/source-bundle.toml";
const MANIFEST_PATH: &str = "MANIFEST.sha256";
const METADATA_PATH: &str = "source-bundle.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BundleMetadata {
    schema: u32,
    version: String,
    repository: String,
    commit: String,
    commit_timestamp: u64,
    packages: Vec<PackageRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PackageRecord {
    name: String,
    archive: String,
    sha256: String,
}

pub(super) fn create(root: &Path, output: &Path, allow_dirty: bool) -> Result<(), String> {
    let recipe = read_recipe(root)?;
    verify_recipe(root, &root.join(RECIPE_PATH))?;
    if !allow_dirty {
        require_clean_tree(root)?;
    }
    let commit = git(root, &["rev-parse", "HEAD"])?;
    validate_commit(&commit)?;
    let timestamp = git(root, &["show", "-s", "--format=%ct", "HEAD"])?
        .parse::<u64>()
        .map_err(|error| format!("invalid candidate commit timestamp: {error}"))?;

    fs::create_dir_all(root.join("target"))
        .map_err(|error| format!("cannot create target directory: {error}"))?;
    let temp = tempfile::Builder::new()
        .prefix("ara2-source-bundle-")
        .tempdir_in(root.join("target"))
        .map_err(|error| format!("cannot create source-bundle staging directory: {error}"))?;
    let staging = temp.path().join("bundle");
    let vendor = staging.join("vendor");
    fs::create_dir_all(&staging)
        .map_err(|error| format!("cannot create {}: {error}", staging.display()))?;

    vendor_locked_graph(root, &vendor)?;
    write_source_config(
        &staging.join(".cargo/config.toml"),
        Path::new("vendor"),
        true,
    )?;
    copy_recipe_inputs(root, &staging, &recipe)?;

    let records = package_members(root, &staging, &vendor, allow_dirty)?;
    write_clean_room_manifest(&staging)?;
    generate_clean_room_lock(&staging)?;

    let metadata = BundleMetadata {
        schema: 1,
        version: VERSION.to_owned(),
        repository: recipe.repository.clone(),
        commit,
        commit_timestamp: timestamp,
        packages: records,
    };
    write_json(&staging.join(METADATA_PATH), &metadata)?;
    verify_staged_tree(&staging, &metadata)?;
    write_inventory(&staging)?;
    write_archive(&staging, output, timestamp)?;
    Ok(())
}

pub(super) fn verify(bundle: &Path) -> Result<(), String> {
    verify_inner(bundle, None)
}

pub(super) fn verify_for_commit(bundle: &Path, expected_commit: &str) -> Result<(), String> {
    verify_inner(bundle, Some(expected_commit))
}

fn verify_inner(bundle: &Path, expected_commit: Option<&str>) -> Result<(), String> {
    let parent = bundle
        .parent()
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| Path::new("."));
    let temp = tempfile::Builder::new()
        .prefix("ara2-source-verify-")
        .tempdir_in(parent)
        .or_else(|_| {
            tempfile::Builder::new()
                .prefix("ara2-source-verify-")
                .tempdir()
        })
        .map_err(|error| format!("cannot create source verification directory: {error}"))?;
    let extracted = temp.path().join("bundle");
    fs::create_dir(&extracted)
        .map_err(|error| format!("cannot create {}: {error}", extracted.display()))?;
    extract_archive(bundle, &extracted)?;
    verify_inventory(&extracted)?;
    let metadata: BundleMetadata = read_json(&extracted.join(METADATA_PATH))?;
    validate_metadata(&metadata)?;
    if expected_commit.is_some_and(|expected| metadata.commit != expected) {
        return Err("source-bundle candidate commit mismatch".to_owned());
    }
    verify_recipe(&extracted, &extracted.join(RECIPE_PATH))?;
    verify_staged_tree(&extracted, &metadata)
}

fn read_recipe(root: &Path) -> Result<Recipe, String> {
    let path = root.join(RECIPE_PATH);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    toml::from_str(&source).map_err(|error| format!("invalid source-bundle recipe: {error}"))
}

pub(super) fn require_clean_tree(root: &Path) -> Result<(), String> {
    let output = command_output(
        {
            let mut command = Command::new("git");
            command
                .current_dir(root)
                .args(["status", "--porcelain", "--untracked-files=no"]);
            command
        },
        "inspect candidate tree",
    )?;
    if output.stdout.is_empty() {
        Ok(())
    } else {
        Err("source-bundle requires a clean tracked candidate tree".to_owned())
    }
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = command_output(
        {
            let mut command = Command::new("git");
            command.current_dir(root).args(args);
            command
        },
        "read candidate Git metadata",
    )?;
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| format!("Git returned non-UTF-8 metadata: {error}"))
}

fn validate_commit(commit: &str) -> Result<(), String> {
    if commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!("invalid 40-character candidate commit: {commit}"))
    }
}

fn vendor_locked_graph(root: &Path, vendor: &Path) -> Result<(), String> {
    if let Some(parent) = vendor.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    command_output(
        {
            let mut command = Command::new(cargo());
            command
                .current_dir(root)
                .args(["vendor", "--locked", "--versioned-dirs"])
                .arg(vendor);
            command
        },
        "vendor locked registry graph",
    )?;
    normalize_vendored_sources(vendor)
}

fn normalize_vendored_sources(vendor: &Path) -> Result<(), String> {
    let mut packages = fs::read_dir(vendor)
        .map_err(|error| format!("cannot read {}: {error}", vendor.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate {}: {error}", vendor.display()))?;
    packages.sort_by_key(|entry| entry.file_name());
    for package in packages {
        let directory = package.path();
        if !directory.is_dir() {
            continue;
        }
        let checksum_path = directory.join(".cargo-checksum.json");
        let checksum: serde_json::Value = read_json(&checksum_path)?;
        let package_digest = checksum["package"].as_str().ok_or_else(|| {
            format!(
                "vendored package checksum has no archive digest: {}",
                checksum_path.display()
            )
        })?;
        for relative in list_files(&directory)? {
            if relative.file_name() == Some(OsStr::new(".gitignore")) {
                fs::remove_file(directory.join(&relative))
                    .map_err(|error| format!("cannot remove {}: {error}", relative.display()))?;
            }
        }
        write_directory_checksum(&directory, package_digest)?;
    }
    Ok(())
}

fn package_members(
    root: &Path,
    staging: &Path,
    vendor: &Path,
    allow_dirty: bool,
) -> Result<Vec<PackageRecord>, String> {
    let package_home = staging.join("package-cargo-home");
    write_source_config(&package_home.join("config.toml"), vendor, false)?;
    let package_target = staging.join("package-target");
    let packages_dir = staging.join("packages");
    let clean_crates = staging.join("clean-room/crates");
    fs::create_dir_all(&packages_dir)
        .and_then(|_| fs::create_dir_all(&clean_crates))
        .map_err(|error| format!("cannot create package staging directories: {error}"))?;

    let mut records = Vec::new();
    for name in PACKAGES {
        let mut command = Command::new(cargo());
        command
            .current_dir(root)
            .env("CARGO_HOME", &package_home)
            .args(["package", "--package", name, "--no-verify", "--locked"])
            .arg("--target-dir")
            .arg(&package_target);
        if allow_dirty {
            command.arg("--allow-dirty");
        }
        command_output(command, &format!("package {name}"))?;

        let filename = format!("{name}-{VERSION}.crate");
        let produced = package_target.join("package").join(&filename);
        if !produced.is_file() {
            return Err(format!(
                "cargo package did not produce {}",
                produced.display()
            ));
        }
        let packaged = packages_dir.join(&filename);
        canonicalize_crate_archive(&produced, &packaged)?;
        let package_digest = sha256_file(&packaged)?;

        let clean_dir = unpack_crate(&packaged, &clean_crates, name)?;
        reject_package_contamination(&clean_dir)?;
        let vendor_dir = vendor.join(format!("{name}-{VERSION}"));
        if vendor_dir.exists() {
            fs::remove_dir_all(&vendor_dir)
                .map_err(|error| format!("cannot replace {}: {error}", vendor_dir.display()))?;
        }
        let unpacked_vendor = unpack_crate(&packaged, vendor, name)?;
        if unpacked_vendor != vendor_dir {
            return Err(format!(
                "unexpected vendored package directory {}; expected {}",
                unpacked_vendor.display(),
                vendor_dir.display()
            ));
        }
        write_directory_checksum(&vendor_dir, &package_digest)?;
        records.push(PackageRecord {
            name: (*name).to_owned(),
            archive: format!("packages/{filename}"),
            sha256: package_digest,
        });
    }
    fs::remove_dir_all(&package_home)
        .and_then(|_| fs::remove_dir_all(&package_target))
        .map_err(|error| format!("cannot remove package scratch directory: {error}"))?;
    Ok(records)
}

fn canonicalize_crate_archive(source: &Path, destination: &Path) -> Result<(), String> {
    let input =
        File::open(source).map_err(|error| format!("cannot open {}: {error}", source.display()))?;
    let mut archive = tar::Archive::new(GzDecoder::new(input));
    let mut files = BTreeMap::<PathBuf, (u32, Vec<u8>)>::new();
    for entry in archive
        .entries()
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?
    {
        let mut entry = entry.map_err(|error| format!("invalid crate entry: {error}"))?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        if !entry.header().entry_type().is_file() {
            return Err(format!(
                "crate archive contains a non-file entry in {}",
                source.display()
            ));
        }
        let relative = entry
            .path()
            .map_err(|error| format!("invalid crate path: {error}"))?
            .into_owned();
        validate_relative_path(&relative)?;
        let mode = entry
            .header()
            .mode()
            .map_err(|error| format!("invalid crate mode: {error}"))?;
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read {}: {error}", relative.display()))?;
        if files.insert(relative.clone(), (mode, bytes)).is_some() {
            return Err(format!(
                "crate archive contains duplicate path {}",
                relative.display()
            ));
        }
    }

    let output = File::create(destination)
        .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
    let encoder = GzBuilder::new().mtime(0).write(output, Compression::new(6));
    let mut canonical = tar::Builder::new(encoder);
    for (relative, (source_mode, bytes)) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(if source_mode & 0o111 == 0 {
            0o644
        } else {
            0o755
        });
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        canonical
            .append_data(&mut header, &relative, bytes.as_slice())
            .map_err(|error| format!("cannot archive {}: {error}", relative.display()))?;
    }
    canonical
        .into_inner()
        .map_err(|error| format!("cannot finish {}: {error}", destination.display()))?
        .finish()
        .map_err(|error| format!("cannot finish {}: {error}", destination.display()))?;
    Ok(())
}

fn unpack_crate(archive: &Path, destination: &Path, name: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
    let file = File::open(archive)
        .map_err(|error| format!("cannot open {}: {error}", archive.display()))?;
    let mut tar = tar::Archive::new(GzDecoder::new(file));
    for entry in tar
        .entries()
        .map_err(|error| format!("cannot read {}: {error}", archive.display()))?
    {
        let mut entry = entry.map_err(|error| format!("invalid crate entry: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("invalid crate path: {error}"))?
            .into_owned();
        validate_relative_path(&path)?;
        entry
            .unpack_in(destination)
            .map_err(|error| format!("cannot unpack {}: {error}", path.display()))?;
    }
    let expected = destination.join(format!("{name}-{VERSION}"));
    if expected.is_dir() {
        Ok(expected)
    } else {
        Err(format!(
            "{} did not contain expected directory {}",
            archive.display(),
            expected.display()
        ))
    }
}

fn reject_package_contamination(package: &Path) -> Result<(), String> {
    for relative in list_files(package)? {
        let text = relative.to_string_lossy();
        if text.starts_with("target/")
            || text.starts_with("reference/")
            || text.starts_with(".third-party/")
            || text.starts_with(".git/")
        {
            return Err(format!("package contamination: {text}"));
        }
    }
    Ok(())
}

fn write_directory_checksum(directory: &Path, package_digest: &str) -> Result<(), String> {
    let mut files = BTreeMap::new();
    for relative in list_files(directory)? {
        if relative == Path::new(".cargo-checksum.json") {
            continue;
        }
        files.insert(
            path_text(&relative)?,
            sha256_file(&directory.join(&relative))?,
        );
    }
    let value = serde_json::json!({"files": files, "package": package_digest});
    write_json(&directory.join(".cargo-checksum.json"), &value)
}

fn write_clean_room_manifest(staging: &Path) -> Result<(), String> {
    let mut source = String::from("[workspace]\nresolver = \"2\"\nmembers = [\n");
    for package in PACKAGES {
        source.push_str(&format!("    \"crates/{package}-{VERSION}\",\n"));
    }
    source.push_str("]\n");
    write_bytes(&staging.join("clean-room/Cargo.toml"), source.as_bytes())
}

fn generate_clean_room_lock(staging: &Path) -> Result<(), String> {
    let cargo_home = staging.join("clean-room-cargo-home");
    fs::create_dir_all(&cargo_home)
        .map_err(|error| format!("cannot create {}: {error}", cargo_home.display()))?;
    let mut command = clean_cargo(staging, &cargo_home);
    command.args([
        "generate-lockfile",
        "--manifest-path",
        "clean-room/Cargo.toml",
        "--offline",
    ]);
    command_output(command, "generate clean-room Cargo.lock")?;
    fs::remove_dir_all(&cargo_home)
        .map_err(|error| format!("cannot remove {}: {error}", cargo_home.display()))
}

fn verify_staged_tree(staging: &Path, metadata: &BundleMetadata) -> Result<(), String> {
    validate_metadata(metadata)?;
    for record in &metadata.packages {
        let archive = staging.join(&record.archive);
        if sha256_file(&archive)? != record.sha256 {
            return Err(format!("package digest mismatch for {}", record.name));
        }
        let clean = staging
            .join("clean-room/crates")
            .join(format!("{}-{VERSION}", record.name));
        if !clean.is_dir() {
            return Err(format!("missing clean-room package {}", clean.display()));
        }
        reject_package_contamination(&clean)?;
        let manifest_path = clean.join("Cargo.toml");
        let manifest = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
        validate_packaged_manifest(&manifest, &record.name)
            .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
        let value: toml::Value = toml::from_str(&manifest)
            .map_err(|error| format!("invalid {}: {error}", manifest_path.display()))?;
        if let Some(readme) = value
            .get("package")
            .and_then(|package| package.get("readme"))
            .and_then(toml::Value::as_str)
        {
            let readme = Path::new(readme);
            validate_relative_path(readme)?;
            if !clean.join(readme).is_file() {
                return Err(format!("packaged README is missing for {}", record.name));
            }
        }
    }
    audit_packaged_derivatives(staging)?;
    verify_clean_room(staging)
}

fn validate_metadata(metadata: &BundleMetadata) -> Result<(), String> {
    if metadata.schema != 1
        || metadata.version != VERSION
        || metadata.repository != "entrepeneur4lyf/ara2-bridge"
    {
        return Err("source-bundle metadata identity mismatch".to_owned());
    }
    validate_commit(&metadata.commit)?;
    let names = metadata
        .packages
        .iter()
        .map(|record| record.name.as_str())
        .collect::<Vec<_>>();
    if names != PACKAGES {
        return Err("source-bundle package order mismatch".to_owned());
    }
    for record in &metadata.packages {
        if record.sha256.len() != 64
            || !record.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || record.archive != format!("packages/{}-{VERSION}.crate", record.name)
        {
            return Err(format!("invalid package metadata for {}", record.name));
        }
    }
    Ok(())
}

fn audit_packaged_derivatives(staging: &Path) -> Result<(), String> {
    let generated = staging
        .join("clean-room/crates")
        .join(format!("ara2-bridge-sys-{VERSION}/src/generated"));
    let files = list_files(&generated)?;
    if files.is_empty() {
        return Err("sys package contains no generated bindings".to_owned());
    }
    for relative in files {
        if relative.extension() == Some(OsStr::new("rs")) {
            let path = generated.join(&relative);
            let source = fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            validate_generated_derivative(&source)
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
    }
    let coverage = staging.join("clean-room/crates").join(format!(
        "ara2-bridge-sys-{VERSION}/generated/symbol-coverage.json"
    ));
    if !coverage.is_file() {
        return Err("sys package omits generated/symbol-coverage.json".to_owned());
    }
    Ok(())
}

fn verify_clean_room(staging: &Path) -> Result<(), String> {
    let lock = staging.join("clean-room/Cargo.lock");
    let saved =
        fs::read(&lock).map_err(|error| format!("cannot read {}: {error}", lock.display()))?;
    fs::remove_file(&lock).map_err(|error| format!("cannot remove {}: {error}", lock.display()))?;
    let cargo_home = staging.join("verify-cargo-home");
    fs::create_dir_all(&cargo_home)
        .map_err(|error| format!("cannot create {}: {error}", cargo_home.display()))?;
    let mut generate = clean_cargo(staging, &cargo_home);
    generate.args([
        "generate-lockfile",
        "--manifest-path",
        "clean-room/Cargo.toml",
        "--offline",
    ]);
    command_output(generate, "regenerate clean-room Cargo.lock")?;
    let regenerated = fs::read(&lock)
        .map_err(|error| format!("cannot read regenerated {}: {error}", lock.display()))?;
    if regenerated != saved {
        return Err("clean-room Cargo.lock is not reproducible".to_owned());
    }
    let mut build = clean_cargo(staging, &cargo_home);
    build.args([
        "build",
        "--manifest-path",
        "clean-room/Cargo.toml",
        "--workspace",
        "--offline",
        "--locked",
    ]);
    command_output(build, "build clean-room workspace offline")?;
    fs::remove_dir_all(staging.join("clean-room/target"))
        .map_err(|error| format!("cannot remove clean-room build output: {error}"))?;
    fs::remove_dir_all(&cargo_home)
        .map_err(|error| format!("cannot remove {}: {error}", cargo_home.display()))?;
    Ok(())
}

fn clean_cargo(staging: &Path, cargo_home: &Path) -> Command {
    let mut command = Command::new(cargo());
    command
        .current_dir(staging)
        .env("CARGO_HOME", cargo_home)
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("ARA_SDK_DIR")
        .env_remove("ARA_CLAP_DIR")
        .env_remove("ARA_VST3_SDK_DIR")
        .env_remove("ARA_AUDIO_UNIT_SDK_DIR")
        .env_remove("LIBCLANG_PATH")
        .env_remove("CC")
        .env_remove("CXX");
    command
}

fn copy_recipe_inputs(root: &Path, staging: &Path, recipe: &Recipe) -> Result<(), String> {
    for relative in &recipe.required_files {
        copy_file(root, staging, Path::new(relative))?;
    }
    for tree in &recipe.required_trees {
        let source = root.join(tree);
        for child in list_files(&source)? {
            copy_file(root, staging, &Path::new(tree).join(child))?;
        }
    }
    Ok(())
}

fn copy_file(root: &Path, staging: &Path, relative: &Path) -> Result<(), String> {
    validate_relative_path(relative)?;
    let source = root.join(relative);
    let metadata = fs::symlink_metadata(&source)
        .map_err(|error| format!("cannot inspect {}: {error}", source.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "source-bundle input is not a regular file: {}",
            source.display()
        ));
    }
    let destination = staging.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::copy(&source, &destination)
        .map_err(|error| format!("cannot copy {}: {error}", source.display()))?;
    Ok(())
}

fn write_source_config(path: &Path, vendor: &Path, offline: bool) -> Result<(), String> {
    let directory = serde_json::to_string(&vendor.to_string_lossy())
        .map_err(|error| format!("cannot encode vendor path: {error}"))?;
    let mut source = format!(
        "[source.crates-io]\nreplace-with = \"vendored-sources\"\n\n[source.vendored-sources]\ndirectory = {directory}\n"
    );
    if offline {
        source.push_str("\n[net]\noffline = true\n");
    }
    write_bytes(path, source.as_bytes())
}

fn write_inventory(staging: &Path) -> Result<(), String> {
    let mut lines = String::new();
    for relative in list_files(staging)? {
        if relative == Path::new(MANIFEST_PATH) {
            continue;
        }
        lines.push_str(&sha256_file(&staging.join(&relative))?);
        lines.push_str("  ");
        lines.push_str(&path_text(&relative)?);
        lines.push('\n');
    }
    write_bytes(&staging.join(MANIFEST_PATH), lines.as_bytes())
}

fn verify_inventory(staging: &Path) -> Result<(), String> {
    let source = fs::read_to_string(staging.join(MANIFEST_PATH))
        .map_err(|error| format!("cannot read {MANIFEST_PATH}: {error}"))?;
    let mut expected = BTreeMap::new();
    for (index, line) in source.lines().enumerate() {
        let (digest, path) = line
            .split_once("  ")
            .ok_or_else(|| format!("malformed manifest line {}", index + 1))?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("invalid digest on manifest line {}", index + 1));
        }
        let relative = PathBuf::from(path);
        validate_relative_path(&relative)?;
        if relative == Path::new(MANIFEST_PATH)
            || expected.insert(relative, digest.to_owned()).is_some()
        {
            return Err(format!(
                "duplicate or recursive manifest path on line {}",
                index + 1
            ));
        }
    }
    let actual = list_files(staging)?
        .into_iter()
        .filter(|path| path != Path::new(MANIFEST_PATH))
        .collect::<BTreeSet<_>>();
    let listed = expected.keys().cloned().collect::<BTreeSet<_>>();
    if actual != listed {
        return Err(format!(
            "source-bundle inventory differs; missing {:?}, extra {:?}",
            listed.difference(&actual).collect::<Vec<_>>(),
            actual.difference(&listed).collect::<Vec<_>>()
        ));
    }
    for (relative, digest) in expected {
        if sha256_file(&staging.join(&relative))? != digest {
            return Err(format!(
                "source-bundle digest mismatch for {}",
                relative.display()
            ));
        }
    }
    Ok(())
}

fn write_archive(staging: &Path, output: &Path, timestamp: u64) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let temporary = output.with_extension("tar.zst.tmp");
    let file = File::create(&temporary)
        .map_err(|error| format!("cannot create {}: {error}", temporary.display()))?;
    let mut encoder = zstd::stream::write::Encoder::new(file, 7)
        .map_err(|error| format!("cannot initialize zstd encoder: {error}"))?;
    encoder
        .include_checksum(true)
        .map_err(|error| format!("cannot configure zstd encoder: {error}"))?;
    {
        let mut archive = tar::Builder::new(&mut encoder);
        for relative in list_files(staging)? {
            let bytes = fs::read(staging.join(&relative))
                .map_err(|error| format!("cannot read {}: {error}", relative.display()))?;
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(timestamp);
            header.set_cksum();
            archive
                .append_data(&mut header, &relative, bytes.as_slice())
                .map_err(|error| format!("cannot archive {}: {error}", relative.display()))?;
        }
        archive
            .finish()
            .map_err(|error| format!("cannot finish source tar: {error}"))?;
    }
    encoder
        .finish()
        .map_err(|error| format!("cannot finish source archive: {error}"))?;
    fs::rename(&temporary, output)
        .map_err(|error| format!("cannot publish {}: {error}", output.display()))
}

fn extract_archive(bundle: &Path, destination: &Path) -> Result<(), String> {
    let file =
        File::open(bundle).map_err(|error| format!("cannot open {}: {error}", bundle.display()))?;
    let decoder = zstd::stream::read::Decoder::new(file)
        .map_err(|error| format!("cannot decode {}: {error}", bundle.display()))?;
    let mut archive = tar::Archive::new(decoder);
    let mut seen = BTreeSet::new();
    for entry in archive
        .entries()
        .map_err(|error| format!("cannot read source archive: {error}"))?
    {
        let mut entry = entry.map_err(|error| format!("invalid source archive entry: {error}"))?;
        if !entry.header().entry_type().is_file() {
            return Err("source archive contains a non-file entry".to_owned());
        }
        let relative = entry
            .path()
            .map_err(|error| format!("invalid source archive path: {error}"))?
            .into_owned();
        validate_relative_path(&relative)?;
        if !seen.insert(relative.clone()) {
            return Err(format!(
                "duplicate source archive entry {}",
                relative.display()
            ));
        }
        entry
            .unpack_in(destination)
            .map_err(|error| format!("cannot extract {}: {error}", relative.display()))?;
    }
    Ok(())
}

fn list_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries = fs::read_dir(current)
        .map_err(|error| format!("cannot read {}: {error}", current.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate {}: {error}", current.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "source bundle cannot contain symlink {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            collect_files(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(
                path.strip_prefix(root)
                    .map_err(|error| format!("cannot relativize {}: {error}", path.display()))?
                    .to_owned(),
            );
        } else {
            return Err(format!(
                "unsupported source-bundle entry {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        Err(format!("unsafe source-bundle path {}", path.display()))
    } else {
        Ok(())
    }
}

fn path_text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| format!("non-UTF-8 source-bundle path {}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
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

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("cannot serialize {}: {error}", path.display()))?;
    bytes.push(b'\n');
    write_bytes(path, &bytes)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let mut file =
        File::create(path).map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn command_output(mut command: Command, action: &str) -> Result<Output, String> {
    let output = command
        .output()
        .map_err(|error| format!("cannot {action}: {error}"))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "failed to {action}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{canonicalize_crate_archive, normalize_vendored_sources, read_json};
    use flate2::{Compression, GzBuilder};
    use serde_json::Value;
    use std::fs::{self, File};
    use std::path::Path;

    fn write_crate(path: &Path, entries: &[(&str, &[u8])], mtime: u64) {
        let output = File::create(path).unwrap();
        let encoder = GzBuilder::new()
            .mtime(mtime as u32)
            .write(output, Compression::fast());
        let mut archive = tar::Builder::new(encoder);
        for (name, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_uid(mtime);
            header.set_gid(mtime);
            header.set_mtime(mtime);
            header.set_cksum();
            archive.append_data(&mut header, name, *bytes).unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn canonical_crate_archive_ignores_input_order_and_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first.crate");
        let second = temp.path().join("second.crate");
        write_crate(
            &first,
            &[("example-1.0.0/b", b"two"), ("example-1.0.0/a", b"one")],
            7,
        );
        write_crate(
            &second,
            &[("example-1.0.0/a", b"one"), ("example-1.0.0/b", b"two")],
            99,
        );
        let canonical_first = temp.path().join("canonical-first.crate");
        let canonical_second = temp.path().join("canonical-second.crate");

        canonicalize_crate_archive(&first, &canonical_first).unwrap();
        canonicalize_crate_archive(&second, &canonical_second).unwrap();

        assert_eq!(
            fs::read(canonical_first).unwrap(),
            fs::read(canonical_second).unwrap()
        );
    }

    #[test]
    fn vendored_sources_drop_cache_specific_ignore_files() {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("dependency-1.0.0");
        fs::create_dir_all(package.join("nested")).unwrap();
        fs::write(package.join("lib.rs"), b"pub fn dependency() {}").unwrap();
        fs::write(package.join("nested/.gitignore"), b"generated\n").unwrap();
        fs::write(
            package.join(".cargo-checksum.json"),
            br#"{"files":{},"package":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
        )
        .unwrap();

        normalize_vendored_sources(temp.path()).unwrap();

        assert!(!package.join("nested/.gitignore").exists());
        let checksum: Value = read_json(&package.join(".cargo-checksum.json")).unwrap();
        assert_eq!(
            checksum["package"],
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert!(checksum["files"].get("lib.rs").is_some());
        assert!(checksum["files"].get("nested/.gitignore").is_none());
    }
}
