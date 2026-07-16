//! Verification of immutable upstream SDK inputs.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

type DynError = Box<dyn std::error::Error>;

#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
    schema: u32,
    sdk_commit: String,
    ara_api_commit: String,
    ara_library_commit: String,
    ara_examples_commit: String,
    submodule: Vec<Submodule>,
    file: Vec<FileEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Submodule {
    path: String,
    commit: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct FileEntry {
    path: String,
    role: String,
    sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ComponentManifest {
    schema: u32,
    component: String,
    repository: String,
    tag: String,
    commit: String,
    tree: String,
    license: String,
    file: Vec<FileEntry>,
}

fn message(text: impl Into<String>) -> DynError {
    std::io::Error::other(text.into()).into()
}

fn read_manifest(path: &Path) -> Result<Manifest, DynError> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        std::io::Error::new(error.kind(), format!("{}: {error}", path.display()))
    })?;
    toml::from_str(&text).map_err(|error| message(format!("{}: {error}", path.display())))
}

fn git_output(checkout: &Path, arguments: &[&str]) -> Result<String, DynError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(arguments)
        .output()
        .map_err(|error| {
            message(format!(
                "could not run git for {}: {error}",
                checkout.display()
            ))
        })?;
    if !output.status.success() {
        return Err(message(format!(
            "git -C {} {} failed: {}",
            checkout.display(),
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn verify_git(root: &Path, manifest: &Manifest) -> Result<(), DynError> {
    if manifest.schema != 1 {
        return Err(message(format!(
            "unsupported sdk-provenance schema {}",
            manifest.schema
        )));
    }

    let sdk = root.join(".third-party/ARA_SDK");
    let dirty = git_output(&sdk, &["status", "--porcelain", "--ignore-submodules=none"])?;
    if !dirty.is_empty() {
        return Err(message(format!(
            "{} is dirty; provenance requires immutable input",
            sdk.display()
        )));
    }
    verify_head(&sdk, &manifest.sdk_commit)?;

    for submodule in &manifest.submodule {
        verify_head(&sdk.join(&submodule.path), &submodule.commit)?;
    }

    let expected_named = [
        ("ARA_API", manifest.ara_api_commit.as_str()),
        ("ARA_Library", manifest.ara_library_commit.as_str()),
        ("ARA_Examples", manifest.ara_examples_commit.as_str()),
    ];
    for (path, expected) in expected_named {
        let listed = manifest
            .submodule
            .iter()
            .find(|submodule| submodule.path == path)
            .ok_or_else(|| message(format!("manifest is missing submodule {path}")))?;
        if listed.commit != expected {
            return Err(message(format!(
                "named commit for {path} does not match its submodule record"
            )));
        }
    }
    Ok(())
}

fn verify_head(checkout: &Path, expected: &str) -> Result<(), DynError> {
    let actual = git_output(checkout, &["rev-parse", "HEAD"])?;
    if actual != expected {
        return Err(message(format!(
            "{} HEAD is {actual}; expected {expected}",
            checkout.display()
        )));
    }
    Ok(())
}

fn file_hash(path: &Path) -> Result<String, DynError> {
    let bytes =
        std::fs::read(path).map_err(|error| message(format!("{}: {error}", path.display())))?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn validate_file_paths(root: &Path, manifest: &Manifest) -> Result<(), DynError> {
    let mut seen = std::collections::BTreeSet::new();
    for entry in &manifest.file {
        let relative = Path::new(&entry.path);
        if relative.is_absolute() || entry.path.split('/').any(|part| part == "..") {
            return Err(message(format!(
                "provenance path must remain inside the repository: {}",
                entry.path
            )));
        }
        if !seen.insert(entry.path.as_str()) {
            return Err(message(format!(
                "duplicate provenance file: {}",
                entry.path
            )));
        }
        if !root.join(relative).is_file() {
            return Err(message(format!(
                "provenance input is not a file: {}",
                entry.path
            )));
        }
    }
    Ok(())
}

/// Verifies the SDK checkout rooted at `root` against `manifest_path`.
pub fn verify(root: &Path, manifest_path: impl AsRef<Path>) -> Result<(), DynError> {
    let manifest = read_manifest(manifest_path.as_ref())?;
    verify_git(root, &manifest)?;
    validate_file_paths(root, &manifest)?;
    for entry in &manifest.file {
        let actual = file_hash(&root.join(&entry.path))?;
        if actual != entry.sha256 {
            return Err(message(format!(
                "SHA-256 mismatch for {}: found {actual}, expected {}",
                entry.path, entry.sha256
            )));
        }
    }
    Ok(())
}

/// Recomputes file hashes after verifying the pinned, clean Git identities.
pub fn refresh(root: &Path, manifest_path: impl AsRef<Path>) -> Result<(), DynError> {
    let manifest_path = manifest_path.as_ref();
    let mut manifest = read_manifest(manifest_path)?;
    verify_git(root, &manifest)?;
    validate_file_paths(root, &manifest)?;
    for entry in &mut manifest.file {
        entry.sha256 = file_hash(&root.join(&entry.path))?;
    }

    let rendered = toml::to_string_pretty(&manifest)?;
    let temporary = temporary_path(manifest_path);
    std::fs::write(&temporary, rendered)
        .map_err(|error| message(format!("{}: {error}", temporary.display())))?;
    std::fs::rename(&temporary, manifest_path)
        .map_err(|error| message(format!("{}: {error}", manifest_path.display())))?;
    verify(root, manifest_path)
}

struct ComponentContract {
    repository: &'static str,
    tag: &'static str,
    commit: &'static str,
    accepted_licenses: &'static [&'static str],
    checkout: PathBuf,
}

fn component_contract(root: &Path, component: &str) -> Result<ComponentContract, DynError> {
    match component {
        "clap" => Ok(ComponentContract {
            repository: "https://github.com/free-audio/clap.git",
            tag: "1.1.9",
            commit: "094bb76c85366a13cc6c49292226d8608d6ae50c",
            accepted_licenses: &["MIT"],
            checkout: root.join(".third-party/clap"),
        }),
        "vst3" => Ok(ComponentContract {
            repository: "https://github.com/steinbergmedia/vst3sdk.git",
            tag: "v3.7.11_build_10",
            commit: "7d92338ae922db2d559ac458824a4df40f37e82e",
            accepted_licenses: &["GPL-3.0-only", "LicenseRef-Steinberg-VST3"],
            checkout: root.join(".third-party/vst3sdk"),
        }),
        "audio-unit" | "audio-unit-v2" => Ok(ComponentContract {
            repository: "https://github.com/apple/AudioUnitSDK.git",
            tag: "AudioUnitSDK-1.0.0",
            commit: "53ea94e5efebf864b70afb673bdd60c977818ec7",
            accepted_licenses: &["Apache-2.0"],
            checkout: root.join(".third-party/AudioUnitSDK"),
        }),
        _ => Err(message(format!(
            "component provenance is not implemented for {component}"
        ))),
    }
}

fn component_manifest_path(root: &Path, component: &str) -> PathBuf {
    root.join("ara2-bridge-companion/provenance")
        .join(format!("{component}.toml"))
}

fn read_component_manifest(path: &Path) -> Result<ComponentManifest, DynError> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        std::io::Error::new(error.kind(), format!("{}: {error}", path.display()))
    })?;
    toml::from_str(&text).map_err(|error| message(format!("{}: {error}", path.display())))
}

/// Verifies one companion SDK provenance manifest and every consumed input hash.
pub fn verify_component(root: &Path, component: &str) -> Result<(), DynError> {
    let contract = component_contract(root, component)?;
    let path = component_manifest_path(root, component);
    let manifest = read_component_manifest(&path)?;
    let actual_tree = git_output(&contract.checkout, &["rev-parse", "HEAD^{tree}"])?;
    if manifest.schema != 1
        || manifest.component != component
        || manifest.repository != contract.repository
        || manifest.tag != contract.tag
        || manifest.commit != contract.commit
        || manifest.tree != actual_tree
        || !contract
            .accepted_licenses
            .contains(&manifest.license.as_str())
    {
        return Err(message(format!(
            "{} metadata does not match the locked SDK contract",
            path.display()
        )));
    }
    let dirty = git_output(&contract.checkout, &["status", "--porcelain"])?;
    if !dirty.is_empty()
        || git_output(&contract.checkout, &["rev-parse", "HEAD"])? != contract.commit
    {
        return Err(message(format!(
            "{} is dirty or at the wrong commit",
            contract.checkout.display()
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    for file in &manifest.file {
        if !seen.insert(file.path.as_str()) {
            return Err(message(format!("duplicate provenance file: {}", file.path)));
        }
        let path = root.join(&file.path);
        if !path.is_file() || file_hash(&path)? != file.sha256 {
            return Err(message(format!(
                "companion provenance hash mismatch: {}",
                file.path
            )));
        }
    }
    if manifest.file.is_empty() {
        return Err(message("companion provenance manifest has no inputs"));
    }
    Ok(())
}

fn clap_dependencies(root: &Path) -> Result<Vec<PathBuf>, DynError> {
    let output = Command::new("cc")
        .current_dir(root)
        .args([
            "-std=c11",
            "-MM",
            "-I.third-party/clap/include",
            "-I.third-party/ARA_SDK/ARA_API",
            "ara2-bridge-testkit/native/clap_probe.c",
        ])
        .output()?;
    if !output.status.success() {
        return Err(message(format!(
            "could not enumerate CLAP headers: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = String::from_utf8(output.stdout)?.replace("\\\n", " ");
    let (_, dependencies) = text
        .split_once(':')
        .ok_or_else(|| message("unexpected C dependency output"))?;
    let canonical_root = root.canonicalize()?;
    let mut files = std::collections::BTreeSet::new();
    for dependency in dependencies.split_whitespace() {
        let canonical = root.join(dependency).canonicalize()?;
        if canonical.starts_with(&canonical_root) {
            files.insert(canonical.strip_prefix(&canonical_root)?.to_owned());
        }
    }
    files.insert(PathBuf::from("ara2-bridge-companion/src/clap/sys.rs"));
    files.insert(PathBuf::from("ara2-bridge-companion/build.rs"));
    files.insert(PathBuf::from(
        "ara2-bridge-testkit/native/clap_layout_probe.c",
    ));
    Ok(files.into_iter().collect())
}

fn local_include_closure(
    root: &Path,
    directory: &Path,
    initial: impl IntoIterator<Item = PathBuf>,
) -> Result<Vec<PathBuf>, DynError> {
    let mut pending = initial.into_iter().collect::<Vec<_>>();
    let mut files = std::collections::BTreeSet::new();
    while let Some(relative) = pending.pop() {
        if !files.insert(relative.clone()) {
            continue;
        }
        let text = std::fs::read_to_string(root.join(&relative))?;
        for line in text.lines() {
            let Some(include) = line.trim().strip_prefix("#include") else {
                continue;
            };
            let include = include.trim();
            let name = include
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .or_else(|| {
                    include
                        .strip_prefix("<AudioUnitSDK/")
                        .and_then(|value| value.strip_suffix('>'))
                });
            let Some(name) = name else { continue };
            let candidate = directory.join(name);
            if root.join(&candidate).is_file() {
                pending.push(candidate);
            }
        }
    }
    Ok(files.into_iter().collect())
}

fn audio_unit_dependencies(root: &Path) -> Result<Vec<PathBuf>, DynError> {
    let source = Path::new(".third-party/AudioUnitSDK/Source");
    let mut files = local_include_closure(root, source, [source.join("AUBase.h")])?;
    files.extend([
        PathBuf::from(".third-party/ARA_SDK/ARA_API/ARAInterface.h"),
        PathBuf::from(".third-party/ARA_SDK/ARA_API/ARAAudioUnit.h"),
        PathBuf::from("ara2-bridge-companion/build.rs"),
        PathBuf::from("ara2-bridge-companion/native/audio_unit/ara_au_shim.h"),
        PathBuf::from("ara2-bridge-companion/native/audio_unit/ara_au_shim.mm"),
        PathBuf::from("ara2-bridge-companion/src/audio_unit/ffi.rs"),
        PathBuf::from("ara2-bridge-companion/probes/audio-unit-symbols.json"),
        PathBuf::from("ara2-bridge-testkit/native/audio_unit_probe.mm"),
        PathBuf::from("ara2-bridge-testkit/tests/audio_unit_interop.rs"),
    ]);
    files.sort();
    files.dedup();
    Ok(files)
}

fn vst3_dependencies(root: &Path) -> Result<Vec<PathBuf>, DynError> {
    let compiler = std::env::var_os("CXX").unwrap_or_else(|| "c++".into());
    let output = Command::new(compiler)
        .current_dir(root)
        .args([
            "-std=c++17",
            "-MM",
            "-I.third-party/vst3sdk",
            "-I.third-party/ARA_SDK/ARA_API",
            "-Iara2-bridge-companion/native/vst3",
            "ara2-bridge-companion/native/vst3/ara_vst3_shim.cpp",
        ])
        .output()?;
    if !output.status.success() {
        return Err(message(format!(
            "could not enumerate VST3 shim headers: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = String::from_utf8(output.stdout)?.replace("\\\n", " ");
    let (_, dependencies) = text
        .split_once(':')
        .ok_or_else(|| message("unexpected C++ dependency output"))?;
    let canonical_root = root.canonicalize()?;
    let mut files = std::collections::BTreeSet::new();
    for dependency in dependencies.split_whitespace() {
        let canonical = root.join(dependency).canonicalize()?;
        if canonical.starts_with(&canonical_root) {
            files.insert(canonical.strip_prefix(&canonical_root)?.to_owned());
        }
    }
    files.extend([
        PathBuf::from("ara2-bridge-companion/build.rs"),
        PathBuf::from("ara2-bridge-companion/src/vst3/ffi.rs"),
        PathBuf::from("ara2-bridge-companion/probes/vst3-symbols.json"),
        PathBuf::from("ara2-bridge-testkit/native/vst3_probe.cpp"),
        PathBuf::from("ara2-bridge-testkit/tests/vst3_abi.rs"),
    ]);
    Ok(files.into_iter().collect())
}

/// Rebuilds one companion provenance manifest from compiler-observed inputs.
pub fn refresh_component(root: &Path, component: &str) -> Result<(), DynError> {
    let contract = component_contract(root, component)?;
    if git_output(&contract.checkout, &["rev-parse", "HEAD"])? != contract.commit
        || !git_output(&contract.checkout, &["status", "--porcelain"])?.is_empty()
    {
        return Err(message(format!(
            "{} must be clean at locked commit {}",
            contract.checkout.display(),
            contract.commit
        )));
    }
    let files = match component {
        "clap" => clap_dependencies(root)?,
        "vst3" => vst3_dependencies(root)?,
        "audio-unit" | "audio-unit-v2" => audio_unit_dependencies(root)?,
        _ => return Err(message(format!("unknown companion component: {component}"))),
    };
    let license = if component == "vst3" {
        let selected = std::env::var("ARA_VST3_LICENSE_POLICY").map_err(|_| {
            message(
                "refreshing VST3 provenance requires ARA_VST3_LICENSE_POLICY=GPL-3.0-only or LicenseRef-Steinberg-VST3",
            )
        })?;
        if !contract.accepted_licenses.contains(&selected.as_str()) {
            return Err(message("invalid ARA_VST3_LICENSE_POLICY"));
        }
        selected
    } else {
        contract.accepted_licenses[0].to_owned()
    };
    let manifest = ComponentManifest {
        schema: 1,
        component: component.to_owned(),
        repository: contract.repository.to_owned(),
        tag: contract.tag.to_owned(),
        commit: contract.commit.to_owned(),
        tree: git_output(&contract.checkout, &["rev-parse", "HEAD^{tree}"])?,
        license,
        file: files
            .into_iter()
            .map(|path| {
                let role = if path.starts_with(".third-party") {
                    "transitive-header"
                } else if path.ends_with("ARACLAP.h") || path.ends_with("ARAInterface.h") {
                    "ara-header"
                } else if path.ends_with("clap_probe.c") {
                    "abi-probe"
                } else {
                    "generated-declaration"
                };
                Ok(FileEntry {
                    path: path.to_string_lossy().replace('\\', "/"),
                    role: role.to_owned(),
                    sha256: file_hash(&root.join(&path))?,
                })
            })
            .collect::<Result<Vec<_>, DynError>>()?,
    };
    let path = component_manifest_path(root, component);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let rendered = toml::to_string_pretty(&manifest)?;
    let temporary = temporary_path(&path);
    std::fs::write(&temporary, rendered)?;
    std::fs::rename(temporary, &path)?;
    verify_component(root, component)
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".tmp-{}", std::process::id()));
    PathBuf::from(name)
}
