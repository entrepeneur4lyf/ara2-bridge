//! Runner-local companion ABI probe envelope generation and verification.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

type DynError = Box<dyn std::error::Error>;

const SCHEMA: u32 = 1;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Envelope {
    schema: u32,
    component: String,
    target: String,
    probe_method: String,
    source_commit: String,
    source_tree: String,
    source_hash: String,
    probe_hash: String,
    payload_hash: String,
    payload: Value,
}

fn error(message: impl Into<String>) -> DynError {
    std::io::Error::other(message.into()).into()
}

fn hash_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn git(checkout: &Path, args: &[&str]) -> Result<String, DynError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(error(format!(
            "git -C {} {} failed: {}",
            checkout.display(),
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn host_target() -> Result<String, DynError> {
    let environment = if cfg!(target_env = "msvc") {
        "msvc"
    } else if cfg!(target_env = "gnu") {
        "gnu"
    } else {
        ""
    };
    target_triple(std::env::consts::OS, std::env::consts::ARCH, environment).map(str::to_owned)
}

fn target_triple(
    operating_system: &str,
    architecture: &str,
    environment: &str,
) -> Result<&'static str, DynError> {
    match (operating_system, architecture, environment) {
        ("linux", "x86_64", "gnu") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64", "gnu") => Ok("aarch64-unknown-linux-gnu"),
        ("windows", "x86_64", "msvc") => Ok("x86_64-pc-windows-msvc"),
        ("windows", "x86", "msvc") => Ok("i686-pc-windows-msvc"),
        ("macos", "x86_64", "") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64", "") => Ok("aarch64-apple-darwin"),
        _ => Err(error(format!(
            "unsupported companion probe runner {architecture}-{operating_system}-{environment}"
        ))),
    }
}

fn component_checkout(root: &Path, component: &str) -> Result<PathBuf, DynError> {
    match component {
        "clap" => Ok(root.join(".third-party/clap")),
        "vst3" => Ok(root.join(".third-party/vst3sdk")),
        "audio-unit-v2" => Ok(root.join(".third-party/AudioUnitSDK")),
        _ => Err(error(format!("unknown companion component: {component}"))),
    }
}

fn source_hash(root: &Path, component: &str) -> Result<String, DynError> {
    let files = match component {
        "clap" => vec![
            root.join(".third-party/ARA_SDK/ARA_API/ARACLAP.h"),
            root.join("ara2-bridge-companion/src/clap/sys.rs"),
        ],
        "vst3" => vec![
            root.join(".third-party/ARA_SDK/ARA_API/ARAVST3.h"),
            root.join("ara2-bridge-companion/native/vst3/ara_vst3_shim.hpp"),
            root.join("ara2-bridge-companion/native/vst3/ara_vst3_shim.cpp"),
            root.join("ara2-bridge-companion/src/vst3/ffi.rs"),
        ],
        "audio-unit-v2" => vec![
            root.join(".third-party/ARA_SDK/ARA_API/ARAAudioUnit.h"),
            root.join("ara2-bridge-companion/native/audio_unit/ara_au_shim.h"),
            root.join("ara2-bridge-companion/native/audio_unit/ara_au_shim.mm"),
            root.join("ara2-bridge-companion/src/audio_unit/ffi.rs"),
        ],
        _ => return Err(error(format!("unknown companion component: {component}"))),
    };
    hash_source_files(root, &files)
}

fn hash_source_files(root: &Path, files: &[PathBuf]) -> Result<String, DynError> {
    let mut hasher = Sha256::new();
    hasher.update(b"ara2-bridge-companion-probe-source-v1\0");
    for file in files {
        let relative = file.strip_prefix(root).map_err(|_| {
            error(format!(
                "probe input is outside the repository: {}",
                file.display()
            ))
        })?;
        let mut normalized_path = Vec::new();
        for (index, component) in relative.components().enumerate() {
            if index != 0 {
                normalized_path.push(b'/');
            }
            normalized_path.extend_from_slice(component.as_os_str().to_string_lossy().as_bytes());
        }
        let contents = std::fs::read(file)?;
        hasher.update((normalized_path.len() as u64).to_be_bytes());
        hasher.update(normalized_path);
        hasher.update((contents.len() as u64).to_be_bytes());
        hasher.update(contents);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn clap_payload(root: &Path) -> Result<Value, DynError> {
    let temporary = std::env::temp_dir().join(format!(
        "ara2-bridge-clap-probe-{}{}",
        std::process::id(),
        std::env::consts::EXE_SUFFIX
    ));
    let compiler = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
    let output = Command::new(compiler)
        .arg("-std=c11")
        .arg("-DARA2_CLAP_PROBE_MAIN")
        .arg(format!(
            "-I{}",
            root.join(".third-party/clap/include").display()
        ))
        .arg(format!(
            "-I{}",
            root.join(".third-party/ARA_SDK/ARA_API").display()
        ))
        .arg(root.join("ara2-bridge-testkit/native/clap_probe.c"))
        .arg("-o")
        .arg(&temporary)
        .output()?;
    if !output.status.success() {
        return Err(error(format!(
            "CLAP probe compilation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let output = Command::new(&temporary).output()?;
    let _ = std::fs::remove_file(&temporary);
    if !output.status.success() {
        return Err(error("CLAP probe executable failed"));
    }
    serde_json::from_slice(&output.stdout).map_err(Into::into)
}

fn clang_resource_dir() -> Result<String, DynError> {
    let output = Command::new("clang").arg("-print-resource-dir").output()?;
    if !output.status.success() {
        return Err(error("clang -print-resource-dir failed"));
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn parse_layout_summary(value: &str) -> Result<(usize, usize), DynError> {
    let value = value
        .trim()
        .strip_prefix("[sizeof=")
        .ok_or_else(|| error(format!("invalid clang layout summary: {value}")))?
        .trim_end_matches(']');
    let (size, align) = value
        .split_once(", align=")
        .ok_or_else(|| error(format!("invalid clang layout summary: {value}")))?;
    Ok((size.parse()?, align.parse()?))
}

fn cross_clap_payload(root: &Path, target: &str) -> Result<Value, DynError> {
    let output = Command::new("clang")
        .current_dir(root)
        .arg(format!("--target={target}"))
        .args(["-ffreestanding", "-nostdinc", "-isystem"])
        .arg(Path::new(&clang_resource_dir()?).join("include"))
        .args([
            "-std=c11",
            "-Xclang",
            "-fdump-record-layouts-complete",
            "-fsyntax-only",
            "-I.third-party/clap/include",
            "-I.third-party/ARA_SDK/ARA_API",
            "ara2-bridge-testkit/native/clap_layout_probe.c",
        ])
        .output()?;
    if !output.status.success() {
        return Err(error(format!(
            "cross-target CLAP layout probe failed for {target}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let requested = [
        "clap_ara_factory",
        "clap_ara_plugin_extension",
        "clap_plugin_entry",
        "clap_plugin",
        "clap_plugin_factory",
    ];
    let mut layouts = serde_json::Map::new();
    let mut current: Option<&str> = None;
    let mut offsets = Vec::new();
    for line in text.lines() {
        let after_bar = line.split_once('|').map(|(_, value)| value.trim());
        if let Some(record) = after_bar.and_then(|value| value.strip_prefix("struct ")) {
            let requested_record = requested
                .iter()
                .copied()
                .find(|candidate| record == *candidate);
            if requested_record.is_some() {
                current = requested_record;
                offsets.clear();
                continue;
            }
        }
        let Some(record) = current else { continue };
        let Some((prefix, value)) = line.split_once('|') else {
            continue;
        };
        if value.trim().starts_with("[sizeof=") {
            let (size, align) = parse_layout_summary(value)?;
            let mut layout = serde_json::Map::new();
            layout.insert("size".to_owned(), Value::from(size));
            if record.starts_with("clap_ara_") {
                layout.insert("align".to_owned(), Value::from(align));
                layout.insert(
                    "offsets".to_owned(),
                    Value::Array(offsets.iter().copied().map(Value::from).collect()),
                );
            }
            layouts.insert(record.to_owned(), Value::Object(layout));
            current = None;
        } else if record.starts_with("clap_ara_") {
            offsets.push(prefix.trim().parse::<usize>()?);
        }
    }
    for record in requested {
        if !layouts.contains_key(record) {
            return Err(error(format!(
                "clang did not emit the {record} layout for {target}"
            )));
        }
    }
    Ok(Value::Object(layouts))
}

fn probe_hash(root: &Path, component: &str) -> Result<String, DynError> {
    let paths: &[&str] = match component {
        "clap" => &[
            "ara2-bridge-testkit/native/clap_probe.c",
            "ara2-bridge-testkit/native/clap_layout_probe.c",
        ],
        "vst3" => &["ara2-bridge-testkit/native/vst3_probe.cpp"],
        "audio-unit-v2" => &["ara2-bridge-testkit/native/audio_unit_probe.mm"],
        _ => return Err(error(format!("unknown companion component: {component}"))),
    };
    let mut hasher = Sha256::new();
    for path in paths {
        hasher.update(path.as_bytes());
        hasher.update(std::fs::read(root.join(path))?);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn compile_and_run(
    root: &Path,
    compiler: std::ffi::OsString,
    arguments: impl IntoIterator<Item = std::ffi::OsString>,
    stem: &str,
) -> Result<Value, DynError> {
    let executable = std::env::temp_dir().join(format!(
        "ara2-bridge-{stem}-{}{}",
        std::process::id(),
        std::env::consts::EXE_SUFFIX
    ));
    let output = Command::new(compiler)
        .current_dir(root)
        .args(arguments)
        .arg("-o")
        .arg(&executable)
        .output()?;
    if !output.status.success() {
        return Err(error(format!(
            "{stem} compilation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let output = Command::new(&executable).output()?;
    let _ = std::fs::remove_file(&executable);
    if !output.status.success() {
        return Err(error(format!("{stem} executable failed")));
    }
    serde_json::from_slice(&output.stdout).map_err(Into::into)
}

fn vst3_language_flags(target: &str) -> &'static [&'static str] {
    if target.ends_with("-msvc") {
        &["/std:c++17", "/EHsc"]
    } else {
        &["-std=c++17"]
    }
}

fn vst3_payload(root: &Path, target: &str) -> Result<Value, DynError> {
    let compiler = std::env::var_os("CXX").unwrap_or_else(|| "c++".into());
    let arguments = vst3_language_flags(target)
        .iter()
        .copied()
        .chain([
            "-I.third-party/vst3sdk",
            "-I.third-party/ARA_SDK/ARA_API",
            "-Iara2-bridge-companion/native/vst3",
            "ara2-bridge-companion/native/vst3/ara_vst3_shim.cpp",
            "ara2-bridge-testkit/native/vst3_probe.cpp",
        ])
        .map(Into::into);
    compile_and_run(root, compiler, arguments, "vst3-probe")
}

fn audio_unit_payload(root: &Path) -> Result<Value, DynError> {
    let compiler = std::env::var_os("CXX").unwrap_or_else(|| "clang++".into());
    compile_and_run(
        root,
        compiler,
        [
            "-std=c++17",
            "-x",
            "objective-c++",
            "-I.third-party/AudioUnitSDK/Source",
            "-I.third-party/ARA_SDK/ARA_API",
            "-Iara2-bridge-companion/native/audio_unit",
            "ara2-bridge-testkit/native/audio_unit_probe.mm",
            "-framework",
            "AudioToolbox",
        ]
        .into_iter()
        .map(Into::into),
        "audio-unit-probe",
    )
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), DynError> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(format!(".tmp-{}", std::process::id()));
    let temporary = PathBuf::from(temporary);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

/// Emits one immutable runner-local probe envelope.
pub fn emit(root: &Path, component: &str, output: &Path, target: &str) -> Result<(), DynError> {
    let actual_target = host_target()?;
    let checkout = component_checkout(root, component)?;
    if !checkout.is_dir() {
        return Err(error(format!(
            "missing locked {component} checkout at {}",
            checkout.display()
        )));
    }
    let (probe_method, payload) = match (component, target == actual_target) {
        ("clap", true) => ("native-execution", clap_payload(root)?),
        ("clap", false) => ("clang-target-layout", cross_clap_payload(root, target)?),
        ("vst3", true) => ("native-execution", vst3_payload(root, target)?),
        ("audio-unit-v2", true) => ("native-execution", audio_unit_payload(root)?),
        ("vst3" | "audio-unit-v2", false) => {
            return Err(error(format!(
                "{component} runtime probe must execute on its matching target runner"
            )))
        }
        _ => return Err(error(format!("unknown companion component: {component}"))),
    };
    let payload_bytes = serde_json::to_vec(&payload)?;
    let envelope = Envelope {
        schema: SCHEMA,
        component: component.to_owned(),
        target: target.to_owned(),
        probe_method: probe_method.to_owned(),
        source_commit: git(&checkout, &["rev-parse", "HEAD"])?,
        source_tree: git(&checkout, &["rev-parse", "HEAD^{tree}"])?,
        source_hash: source_hash(root, component)?,
        probe_hash: probe_hash(root, component)?,
        payload_hash: hash_bytes(&payload_bytes),
        payload,
    };
    let json = serde_json::to_vec_pretty(&envelope)?;
    if output.extension().and_then(|extension| extension.to_str()) == Some("zst") {
        let mut archive = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(json.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_cksum();
        archive.append_data(&mut header, "envelope.json", json.as_slice())?;
        let tar = archive.into_inner()?;
        atomic_write(output, &zstd::stream::encode_all(tar.as_slice(), 19)?)
    } else {
        atomic_write(output, &json)
    }
}

fn read_envelope(path: &Path) -> Result<Envelope, DynError> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        let decoded = zstd::stream::decode_all(bytes.as_slice())?;
        let mut archive = tar::Archive::new(decoded.as_slice());
        for entry in archive.entries()? {
            let mut entry = entry?;
            if entry.path()?.as_ref() == Path::new("envelope.json") {
                let mut json = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut json)?;
                return serde_json::from_slice(&json).map_err(Into::into);
            }
        }
        Err(error("probe envelope archive has no envelope.json"))
    } else {
        serde_json::from_slice(&bytes).map_err(Into::into)
    }
}

/// Validates envelope metadata, source freshness, and payload integrity.
pub fn validate_envelope(root: &Path, component: &str, path: &Path) -> Result<(), DynError> {
    let envelope = read_envelope(path)?;
    if envelope.schema != SCHEMA || envelope.component != component {
        return Err(error("companion probe envelope metadata mismatch"));
    }
    let checkout = component_checkout(root, component)?;
    if envelope.source_commit != git(&checkout, &["rev-parse", "HEAD"])?
        || envelope.source_tree != git(&checkout, &["rev-parse", "HEAD^{tree}"])?
        || envelope.source_hash != source_hash(root, component)?
        || envelope.probe_hash != probe_hash(root, component)?
        || envelope.payload_hash != hash_bytes(&serde_json::to_vec(&envelope.payload)?)
    {
        return Err(error("companion probe envelope is stale or corrupted"));
    }
    if !matches!(
        envelope.probe_method.as_str(),
        "native-execution" | "clang-target-layout"
    ) {
        return Err(error("unknown companion probe evidence method"));
    }
    Ok(())
}

fn canonical_name(component: &str, target: &str) -> Result<&'static str, DynError> {
    match (component, target) {
        ("clap", "x86_64-unknown-linux-gnu") => Ok("clap-x86_64.json"),
        ("clap", "aarch64-unknown-linux-gnu") => Ok("clap-aarch64.json"),
        ("clap", "i686-pc-windows-msvc") => Ok("clap-i686.json"),
        ("vst3", "x86_64-unknown-linux-gnu") => Ok("vst3-linux-x86_64.json"),
        ("vst3", "aarch64-unknown-linux-gnu") => Ok("vst3-linux-aarch64.json"),
        ("vst3", "x86_64-pc-windows-msvc") => Ok("vst3-windows-x86_64.json"),
        ("vst3", "x86_64-apple-darwin") => Ok("vst3-macos-x86_64.json"),
        ("vst3", "aarch64-apple-darwin") => Ok("vst3-macos-aarch64.json"),
        ("audio-unit-v2", "x86_64-apple-darwin") => Ok("audio-unit-macos-x86_64.json"),
        ("audio-unit-v2", "aarch64-apple-darwin") => Ok("audio-unit-macos-aarch64.json"),
        _ => Err(error(format!(
            "unsupported {component} companion probe target {target}"
        ))),
    }
}

/// Re-runs one probe on its declared runner and compares it with the canonical envelope.
pub fn check_target(root: &Path, component: &str, target: &str) -> Result<(), DynError> {
    let canonical = root
        .join("ara2-bridge-companion/probes")
        .join(canonical_name(component, target)?);
    validate_envelope(root, component, &canonical)?;
    let temporary = tempfile::tempdir()?;
    let emitted = temporary.path().join("runner-envelope.json");
    emit(root, component, &emitted, target)?;
    validate_envelope(root, component, &emitted)?;
    if read_envelope(&emitted)? != read_envelope(&canonical)? {
        return Err(error(format!(
            "fresh {component} probe for {target} differs from canonical {}",
            canonical.display()
        )));
    }
    Ok(())
}

/// Imports a directory of validated, uniquely targeted probe envelopes atomically.
pub fn import_dir(root: &Path, component: &str, directory: &Path) -> Result<(), DynError> {
    component_checkout(root, component)?;
    let mut targets = BTreeSet::new();
    let mut imports = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let envelope = match read_envelope(&path) {
            Ok(envelope) if envelope.component == component => envelope,
            _ => continue,
        };
        validate_envelope(root, component, &path)?;
        if !targets.insert(envelope.target.clone()) {
            return Err(error(format!(
                "duplicate {component} probe target {}",
                envelope.target
            )));
        }
        let name = canonical_name(component, &envelope.target)?;
        imports.push((envelope, name));
    }
    if imports.is_empty() {
        return Err(error(format!(
            "no {component} probe envelopes found in {}",
            directory.display()
        )));
    }
    let destination = root.join("ara2-bridge-companion/probes");
    std::fs::create_dir_all(&destination)?;
    for (envelope, name) in imports {
        atomic_write(
            &destination.join(name),
            &serde_json::to_vec_pretty(&envelope)?,
        )?;
    }
    Ok(())
}

/// Validates every canonical result required for a companion component.
pub fn check_all(root: &Path, component: &str) -> Result<(), DynError> {
    let names: &[&str] = match component {
        "clap" => &["clap-x86_64.json", "clap-aarch64.json", "clap-i686.json"],
        "vst3" => &[
            "vst3-linux-x86_64.json",
            "vst3-linux-aarch64.json",
            "vst3-windows-x86_64.json",
            "vst3-macos-x86_64.json",
            "vst3-macos-aarch64.json",
        ],
        "audio-unit-v2" => &[
            "audio-unit-macos-x86_64.json",
            "audio-unit-macos-aarch64.json",
        ],
        _ => return Err(error(format!("unknown companion component: {component}"))),
    };
    for name in names {
        let path = root.join("ara2-bridge-companion/probes").join(name);
        if !path.is_file() {
            return Err(error(format!(
                "missing canonical probe result {}",
                path.display()
            )));
        }
        validate_envelope(root, component, &path)?;
    }
    validate_symbol_manifest(root, component)?;
    Ok(())
}

fn validate_symbol_manifest(root: &Path, component: &str) -> Result<(), DynError> {
    let (header, manifest_name) = match component {
        "clap" => ("ARACLAP.h", "clap-symbols.json"),
        "vst3" => ("ARAVST3.h", "vst3-symbols.json"),
        "audio-unit-v2" => ("ARAAudioUnit.h", "audio-unit-symbols.json"),
        _ => return Err(error(format!("unknown companion component: {component}"))),
    };
    let coverage: Value = serde_json::from_slice(&std::fs::read(
        root.join("ara2-bridge-sys/generated/symbol-coverage.json"),
    )?)?;
    let expected = coverage["records"]
        .as_array()
        .ok_or_else(|| error("core symbol coverage has no records"))?
        .iter()
        .filter(|record| record["header"] == header)
        .filter_map(|record| record["symbol"].as_str())
        .collect::<BTreeSet<_>>();
    let manifest: Value = serde_json::from_slice(&std::fs::read(
        root.join("ara2-bridge-companion/probes")
            .join(manifest_name),
    )?)?;
    let actual = manifest["records"]
        .as_array()
        .ok_or_else(|| error("companion symbol manifest has no records"))?
        .iter()
        .filter_map(|record| record["symbol"].as_str())
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(error(format!(
            "{component} symbol manifest does not close the core deferred declarations"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod source_hash_tests {
    use super::{hash_source_files, target_triple, vst3_language_flags};
    use std::fs;

    #[test]
    fn source_hash_is_independent_of_checkout_location() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for root in [first.path(), second.path()] {
            fs::create_dir(root.join("nested")).unwrap();
            fs::write(root.join("nested/input.h"), b"same bytes").unwrap();
        }

        let first_hash =
            hash_source_files(first.path(), &[first.path().join("nested/input.h")]).unwrap();
        let second_hash =
            hash_source_files(second.path(), &[second.path().join("nested/input.h")]).unwrap();

        assert_eq!(first_hash, second_hash);
    }

    #[test]
    fn compile_target_mapping_covers_every_runtime_probe_runner() {
        assert_eq!(
            target_triple("linux", "x86_64", "gnu").unwrap(),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            target_triple("linux", "aarch64", "gnu").unwrap(),
            "aarch64-unknown-linux-gnu"
        );
        assert_eq!(
            target_triple("windows", "x86_64", "msvc").unwrap(),
            "x86_64-pc-windows-msvc"
        );
        assert_eq!(
            target_triple("windows", "x86", "msvc").unwrap(),
            "i686-pc-windows-msvc"
        );
        assert_eq!(
            target_triple("macos", "x86_64", "").unwrap(),
            "x86_64-apple-darwin"
        );
        assert_eq!(
            target_triple("macos", "aarch64", "").unwrap(),
            "aarch64-apple-darwin"
        );
        assert!(target_triple("linux", "riscv64", "gnu").is_err());
    }

    #[test]
    fn vst3_probe_uses_the_target_compiler_dialect() {
        assert_eq!(
            vst3_language_flags("x86_64-pc-windows-msvc"),
            ["/std:c++17", "/EHsc"]
        );
        assert_eq!(
            vst3_language_flags("x86_64-unknown-linux-gnu"),
            ["-std=c++17"]
        );
        assert_eq!(vst3_language_flags("aarch64-apple-darwin"), ["-std=c++17"]);
    }

    #[test]
    fn source_hash_frames_paths_and_file_contents() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        fs::write(first.path().join("ab"), b"c").unwrap();
        fs::write(second.path().join("a"), b"bc").unwrap();

        let first_hash = hash_source_files(first.path(), &[first.path().join("ab")]).unwrap();
        let second_hash = hash_source_files(second.path(), &[second.path().join("a")]).unwrap();

        assert_ne!(first_hash, second_hash);
    }
}
