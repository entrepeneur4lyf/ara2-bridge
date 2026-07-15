//! Cross-language core ABI probe artifacts.

use crate::{bindings, provenance, Mode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

type DynError = Box<dyn std::error::Error>;

const SOURCE_REPOSITORY: &str = "https://github.com/Celemony/ARA_API";
const SOURCE_TAG: &str = "releases/2.3.0";
const SOURCE_COMMIT: &str = "65ec5c43b943a48cb5446f448a0492db6af8534b";
const GENERATOR: &str = "ara2-bridge xtask 0.2.0-alpha.1";

/// Canonical target ABI families required by the core probe gate.
pub const FAMILIES: &[&str] = &["x86_64", "aarch64", "i686"];

#[derive(Debug, Deserialize, Serialize)]
struct Envelope {
    schema: u32,
    source_repository: String,
    source_tag: String,
    normative_commit: String,
    generator: String,
    spdx_license: String,
    notice: String,
    target_triple: String,
    family: String,
    source_hashes: BTreeMap<String, String>,
    probe_hash: String,
    payload_hash: String,
    payload: Value,
}

#[derive(Debug)]
struct Inventory {
    structs: BTreeMap<String, Vec<String>>,
    constants: Vec<String>,
}

#[derive(Debug)]
struct Compiler {
    executable: String,
    prefix: Vec<String>,
}

fn message(text: impl Into<String>) -> DynError {
    std::io::Error::other(text.into()).into()
}

fn target_triple(family: &str) -> Result<&'static str, DynError> {
    match family {
        "x86_64" => Ok("x86_64-unknown-linux-gnu"),
        "aarch64" => Ok("aarch64-unknown-linux-gnu"),
        "i686" => Ok("i686-pc-windows-msvc"),
        _ => Err(message(format!("unknown core ABI family: {family}"))),
    }
}

fn raw_filename(family: &str) -> Result<&'static str, DynError> {
    match family {
        "x86_64" => Ok("x86_64.rs"),
        "aarch64" => Ok("aarch64.rs"),
        "i686" => Ok("i686.rs"),
        _ => Err(message(format!("unknown core ABI family: {family}"))),
    }
}

/// Generates or checks the Rust-side probe assertion table.
pub fn generate_support(root: &Path, mode: Mode) -> Result<(), DynError> {
    provenance::verify(root, root.join("sdk-provenance.toml"))?;
    let complete = complete_structs(root)?;
    let mut source = bindings::rust_banner();
    source.push_str(
        "// Rust-side comparisons for imported C/C++ core ABI payloads.\n\n\
         use ara2_bridge_sys::*;\n\n",
    );
    for family in FAMILIES {
        let raw = std::fs::read_to_string(
            root.join("ara2-bridge-sys/src/generated")
                .join(raw_filename(family)?),
        )?;
        let inventory = raw_inventory(&raw, &complete)?;
        let cfg = match *family {
            "x86_64" => "target_arch = \"x86_64\"",
            "aarch64" => "target_arch = \"aarch64\"",
            "i686" => "all(target_arch = \"x86\", target_pointer_width = \"32\")",
            _ => unreachable!(),
        };
        source.push_str(&format!(
            "#[cfg({cfg})]\n\
             pub fn assert_current(payload: &serde_json::Value) {{\n"
        ));
        for (name, fields) in &inventory.structs {
            source.push_str(&format!(
                "assert_eq!(payload[\"structs\"][{name:?}][\"size\"].as_u64().unwrap(), ::std::mem::size_of::<{name}>() as u64, \"sizeof {name}\");\n\
                 assert_eq!(payload[\"structs\"][{name:?}][\"alignment\"].as_u64().unwrap(), ::std::mem::align_of::<{name}>() as u64, \"alignof {name}\");\n"
            ));
            for field in fields {
                source.push_str(&format!(
                    "assert_eq!(payload[\"structs\"][{name:?}][\"fields\"][{field:?}].as_u64().unwrap(), ::std::mem::offset_of!({name}, {field}) as u64, \"offsetof {name}.{field}\");\n"
                ));
            }
        }
        for constant in &inventory.constants {
            source.push_str(&format!(
                "assert_eq!(payload[\"constants\"][{constant:?}].as_str().unwrap(), {constant}.to_string(), \"constant {constant}\");\n"
            ));
        }
        source.push_str(
            "assert_eq!(payload[\"cpp\"][\"kARAXMLName_CreateDistinctAudioModification\"].as_str().unwrap(), audio_file_chunks::kARAXMLName_CreateDistinctAudioModification);\n}\n\n",
        );
    }
    let source = bindings::format_rust(&source)?;
    bindings::apply(
        mode,
        &root.join("ara2-bridge-sys/tests/generated/core_abi_assertions.rs"),
        source.as_bytes(),
    )
}

/// Compiles and runs the C/C++ probe for `family`, then writes a deterministic envelope.
pub fn emit(root: &Path, output: &Path, family: &str) -> Result<(), DynError> {
    provenance::verify(root, root.join("sdk-provenance.toml"))?;
    generate_support(root, Mode::Check)?;
    let triple = target_triple(family)?;
    let complete = complete_structs(root)?;
    let raw = std::fs::read_to_string(
        root.join("ara2-bridge-sys/src/generated")
            .join(raw_filename(family)?),
    )?;
    let inventory = raw_inventory(&raw, &complete)?;
    let table = render_c_table(&inventory);

    let work = root
        .join("target/core-probe")
        .join(format!("{family}-{}", std::process::id()));
    if work.exists() {
        std::fs::remove_dir_all(&work)?;
    }
    std::fs::create_dir_all(&work)?;
    let table_path = work.join("ara_probe_table.inc");
    std::fs::write(&table_path, &table)?;
    let executable = compile_probe(root, &work, family)?;
    let payload_bytes = run_probe(&executable, family)?;
    let payload: Value = serde_json::from_slice(&payload_bytes).map_err(|error| {
        message(format!(
            "probe for {family} did not emit valid JSON: {error}; output={}",
            String::from_utf8_lossy(&payload_bytes)
        ))
    })?;

    let source_hashes = source_hashes(root, &table)?;
    validate_payload(&payload, &inventory)?;
    let canonical_payload = serde_json::to_vec(&payload)?;
    let payload_hash = sha256(&canonical_payload);
    let probe_hash = combined_probe_hash(triple, &source_hashes);
    let envelope = Envelope {
        schema: 1,
        source_repository: SOURCE_REPOSITORY.to_owned(),
        source_tag: SOURCE_TAG.to_owned(),
        normative_commit: SOURCE_COMMIT.to_owned(),
        generator: GENERATOR.to_owned(),
        spdx_license: "Apache-2.0".to_owned(),
        notice: "DO NOT EDIT".to_owned(),
        target_triple: triple.to_owned(),
        family: family.to_owned(),
        source_hashes,
        probe_hash,
        payload_hash,
        payload,
    };
    validate_envelope(root, &envelope)?;
    write_envelope_archive(output, &envelope)?;
    std::fs::remove_dir_all(work)?;
    Ok(())
}

/// Imports exactly one validated envelope for every ABI family.
pub fn import_dir(root: &Path, directory: &Path) -> Result<(), DynError> {
    generate_support(root, Mode::Check)?;
    let mut envelopes = BTreeMap::new();
    for family in FAMILIES {
        let path = directory.join(format!("{family}.probe.tar.zst"));
        if !path.is_file() {
            return Err(message(format!(
                "missing probe envelope for {family}: {}",
                path.display()
            )));
        }
        let envelope = read_envelope_archive(&path)?;
        if envelope.family != *family {
            return Err(message(format!(
                "probe family mismatch in {}: found {}, expected {family}",
                path.display(),
                envelope.family
            )));
        }
        validate_envelope(root, &envelope)?;
        if envelopes.insert(*family, envelope).is_some() {
            return Err(message(format!("duplicate probe family: {family}")));
        }
    }

    let canonical = root.join("ara2-bridge-sys/tests/generated");
    std::fs::create_dir_all(&canonical)?;
    for (family, envelope) in envelopes {
        let path = canonical.join(format!("{family}-core-abi.json"));
        let temporary = path.with_extension(format!("json.tmp-{}", std::process::id()));
        let mut bytes = serde_json::to_vec_pretty(&envelope)?;
        bytes.push(b'\n');
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(temporary, path)?;
    }
    check_all(root)
}

/// Checks every canonical core ABI-family artifact and all embedded hashes.
pub fn check_all(root: &Path) -> Result<(), DynError> {
    generate_support(root, Mode::Check)?;
    let mut missing = Vec::new();
    for family in FAMILIES {
        let path = root.join(format!(
            "ara2-bridge-sys/tests/generated/{family}-core-abi.json"
        ));
        if !path.is_file() {
            missing.push(path.display().to_string());
            continue;
        }
        let envelope: Envelope = serde_json::from_slice(&std::fs::read(&path)?)?;
        if envelope.family != *family {
            return Err(message(format!(
                "canonical family mismatch in {}: {}",
                path.display(),
                envelope.family
            )));
        }
        validate_envelope(root, &envelope)?;
        let mut canonical = serde_json::to_vec_pretty(&envelope)?;
        canonical.push(b'\n');
        if canonical != std::fs::read(&path)? {
            return Err(message(format!(
                "non-canonical core ABI JSON: {}",
                path.display()
            )));
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("missing core ABI artifacts: {}", missing.join(", ")),
        )
        .into())
    }
}

fn complete_structs(root: &Path) -> Result<BTreeSet<String>, DynError> {
    let coverage: Value = serde_json::from_slice(&std::fs::read(
        root.join("ara2-bridge-sys/generated/symbol-coverage.json"),
    )?)?;
    Ok(coverage["records"]
        .as_array()
        .ok_or_else(|| message("symbol coverage records are missing"))?
        .iter()
        .filter(|record| record["classification"] == "core-abi" && record["kind"] == "struct")
        .filter_map(|record| record["symbol"].as_str().map(str::to_owned))
        .collect())
}

fn raw_inventory(raw: &str, complete: &BTreeSet<String>) -> Result<Inventory, DynError> {
    let mut all_structs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut constants = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("pub const kARA") {
            let suffix = rest.split(':').next().unwrap_or("");
            constants.push(format!("kARA{suffix}"));
        }
        if current.is_none() {
            if let Some(rest) = trimmed.strip_prefix("pub struct ") {
                let name = rest.split_whitespace().next().unwrap_or("");
                current = Some(name.to_owned());
                all_structs.entry(name.to_owned()).or_default();
            }
            continue;
        }
        if trimmed == "}" {
            current = None;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("pub ") {
            if let Some((field, _)) = rest.split_once(':') {
                if !field.starts_with('_') {
                    all_structs
                        .get_mut(current.as_ref().unwrap())
                        .unwrap()
                        .push(field.to_owned());
                }
            }
        }
    }
    constants.sort();
    constants.dedup();
    let structs: BTreeMap<String, Vec<String>> = all_structs
        .into_iter()
        .filter(|(name, _)| complete.contains(name))
        .collect();
    if structs.is_empty() || constants.is_empty() {
        return Err(message("raw probe inventory is unexpectedly empty"));
    }
    Ok(Inventory { structs, constants })
}

fn render_c_table(inventory: &Inventory) -> String {
    let mut output = String::from(
        "/* DO NOT EDIT: generated core ABI probe table. */\n\
         static void ara_probe_emit_structs(void)\n{\n    int first_struct = 1;\n",
    );
    for (name, fields) in &inventory.structs {
        output.push_str(&format!(
            "    printf(\"%s\\\"{name}\\\":{{\\\"size\\\":%zu,\\\"alignment\\\":%zu,\\\"fields\\\":{{\", first_struct ? \"\" : \",\", sizeof({name}), _Alignof({name}));\n\
             first_struct = 0;\n\
             {{ int first_field = 1;\n"
        ));
        for field in fields {
            output.push_str(&format!(
                "        printf(\"%s\\\"{field}\\\":%zu\", first_field ? \"\" : \",\", offsetof({name}, {field})); first_field = 0;\n"
            ));
        }
        output.push_str("    } fputs(\"}}\", stdout);\n");
    }
    output.push_str("}\n\nstatic void ara_probe_emit_constants(void)\n{\n    int first = 1;\n");
    for constant in &inventory.constants {
        output.push_str(&format!(
            "    printf(\"%s\\\"{constant}\\\":\\\"%.21Lg\\\"\", first ? \"\" : \",\", (long double)({constant})); first = 0;\n"
        ));
    }
    output.push_str("}\n");
    output
}

fn compiler(family: &str, cxx: bool) -> Compiler {
    let language = if cxx { "CXX" } else { "CC" };
    let specific = format!("ARA2_BRIDGE_{language}_{}", family.to_ascii_uppercase());
    if let Ok(executable) = std::env::var(&specific) {
        return Compiler {
            executable,
            prefix: Vec::new(),
        };
    }
    if family_is_native(family) {
        return Compiler {
            executable: std::env::var(language)
                .unwrap_or_else(|_| if cxx { "c++" } else { "cc" }.to_owned()),
            prefix: Vec::new(),
        };
    }
    let zig = if Path::new("/snap/bin/zig").is_file() {
        "/snap/bin/zig"
    } else {
        "zig"
    };
    let target = match family {
        "aarch64" => "aarch64-linux-musl",
        "i686" => "x86-windows-gnu",
        _ => "x86_64-linux-gnu",
    };
    Compiler {
        executable: zig.to_owned(),
        prefix: vec![
            if cxx { "c++" } else { "cc" }.to_owned(),
            "-target".to_owned(),
            target.to_owned(),
        ],
    }
}

fn family_is_native(family: &str) -> bool {
    matches!(
        (family, std::env::consts::ARCH, std::env::consts::OS),
        ("x86_64", "x86_64", "linux")
            | ("aarch64", "aarch64", "linux")
            | ("i686", "x86", "windows")
    )
}

fn compile_probe(root: &Path, work: &Path, family: &str) -> Result<PathBuf, DynError> {
    let api = root.join("reference/ARA_SDK/ARA_API");
    let probe = root.join("ara2-bridge-sys/tests/probe");
    let c_object = work.join("ara_layout.o");
    let cpp_object = work.join("ara_core.o");
    let executable = work.join(if family == "i686" {
        "ara_core_probe.exe"
    } else {
        "ara_core_probe"
    });

    let cc = compiler(family, false);
    let mut command = Command::new(&cc.executable);
    command
        .args(&cc.prefix)
        .arg("-std=c11")
        .arg("-I")
        .arg(&api)
        .arg("-I")
        .arg(work)
        .arg("-c")
        .arg(probe.join("ara_layout.c"))
        .arg("-o")
        .arg(&c_object);
    run_checked(command, "compile C core ABI probe")?;

    let cxx = compiler(family, true);
    let mut command = Command::new(&cxx.executable);
    command
        .args(&cxx.prefix)
        .arg("-std=c++17")
        .arg("-I")
        .arg(&api)
        .arg("-c")
        .arg(probe.join("ara_core.cpp"))
        .arg("-o")
        .arg(&cpp_object);
    run_checked(command, "compile C++ core ABI probe")?;

    let mut command = Command::new(&cxx.executable);
    command
        .args(&cxx.prefix)
        .arg(&c_object)
        .arg(&cpp_object)
        .arg("-o")
        .arg(&executable);
    if family == "aarch64" && !family_is_native(family) {
        command.arg("-static");
    }
    run_checked(command, "link core ABI probe")?;
    Ok(executable)
}

fn run_checked(mut command: Command, description: &str) -> Result<(), DynError> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(message(format!(
            "{description} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

fn run_probe(executable: &Path, family: &str) -> Result<Vec<u8>, DynError> {
    let mut command = if family_is_native(family) {
        Command::new(executable)
    } else {
        let variable = format!(
            "ARA2_BRIDGE_CORE_PROBE_RUNNER_{}",
            family.to_ascii_uppercase()
        );
        let runner = std::env::var(&variable).unwrap_or_else(|_| match family {
            "aarch64" => "qemu-aarch64".to_owned(),
            "i686" => "wine".to_owned(),
            _ => String::new(),
        });
        let mut parts = runner.split_whitespace();
        let program = parts.next().ok_or_else(|| {
            message(format!(
                "no runner configured; set {variable} for family {family}"
            ))
        })?;
        let mut command = Command::new(program);
        command.args(parts).arg(executable);
        command
    };
    if family == "i686" {
        command.env("WINEDEBUG", "-all");
    }
    let output = command.output().map_err(|error| {
        message(format!(
            "could not execute {family} probe (configure ARA2_BRIDGE_CORE_PROBE_RUNNER_{}): {error}",
            family.to_ascii_uppercase()
        ))
    })?;
    if !output.status.success() {
        return Err(message(format!(
            "{family} probe failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

fn source_hashes(root: &Path, table: &str) -> Result<BTreeMap<String, String>, DynError> {
    let probe = root.join("ara2-bridge-sys/tests/probe");
    Ok(BTreeMap::from([
        (
            "ara_layout.c".to_owned(),
            sha256(&std::fs::read(probe.join("ara_layout.c"))?),
        ),
        (
            "ara_core.cpp".to_owned(),
            sha256(&std::fs::read(probe.join("ara_core.cpp"))?),
        ),
        ("ara_probe_table.inc".to_owned(), sha256(table.as_bytes())),
    ]))
}

fn combined_probe_hash(triple: &str, hashes: &BTreeMap<String, String>) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(triple.as_bytes());
    for (name, hash) in hashes {
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(hash.as_bytes());
    }
    sha256(&bytes)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_payload(payload: &Value, inventory: &Inventory) -> Result<(), DynError> {
    let structs = payload["structs"]
        .as_object()
        .ok_or_else(|| message("probe payload has no structs object"))?;
    let actual_structs: BTreeSet<_> = structs.keys().map(String::as_str).collect();
    let expected_structs: BTreeSet<_> = inventory.structs.keys().map(String::as_str).collect();
    if actual_structs != expected_structs {
        return Err(message("probe payload struct inventory mismatch"));
    }
    for (name, expected_fields) in &inventory.structs {
        let record = &structs[name];
        if record["size"].as_u64().is_none() || record["alignment"].as_u64().is_none() {
            return Err(message(format!(
                "probe has invalid layout values for {name}"
            )));
        }
        let fields = record["fields"]
            .as_object()
            .ok_or_else(|| message(format!("probe has no fields for {name}")))?;
        let actual: BTreeSet<_> = fields.keys().map(String::as_str).collect();
        let expected: BTreeSet<_> = expected_fields.iter().map(String::as_str).collect();
        if actual != expected || fields.values().any(|value| value.as_u64().is_none()) {
            return Err(message(format!(
                "probe field inventory mismatch for {name}"
            )));
        }
    }
    let constants = payload["constants"]
        .as_object()
        .ok_or_else(|| message("probe payload has no constants object"))?;
    let actual: BTreeSet<_> = constants.keys().map(String::as_str).collect();
    let expected: BTreeSet<_> = inventory.constants.iter().map(String::as_str).collect();
    if actual != expected || constants.values().any(|value| value.as_str().is_none()) {
        return Err(message("probe constant inventory mismatch"));
    }
    if payload["cpp"]["kARAXMLName_CreateDistinctAudioModification"]
        != "createDistinctAudioModification"
    {
        return Err(message("C++ audio-file chunk constant mismatch"));
    }
    Ok(())
}

fn validate_envelope(root: &Path, envelope: &Envelope) -> Result<(), DynError> {
    if envelope.schema != 1
        || envelope.source_repository != SOURCE_REPOSITORY
        || envelope.source_tag != SOURCE_TAG
        || envelope.normative_commit != SOURCE_COMMIT
        || envelope.generator != GENERATOR
        || envelope.spdx_license != "Apache-2.0"
        || envelope.notice != "DO NOT EDIT"
    {
        return Err(message("core ABI envelope provenance metadata mismatch"));
    }
    if envelope.target_triple != target_triple(&envelope.family)? {
        return Err(message(format!(
            "target/family mismatch: {}/{}",
            envelope.target_triple, envelope.family
        )));
    }
    let complete = complete_structs(root)?;
    let raw = std::fs::read_to_string(
        root.join("ara2-bridge-sys/src/generated")
            .join(raw_filename(&envelope.family)?),
    )?;
    let inventory = raw_inventory(&raw, &complete)?;
    let table = render_c_table(&inventory);
    let expected_sources = source_hashes(root, &table)?;
    if envelope.source_hashes != expected_sources {
        return Err(message(format!(
            "source hash mismatch for {} core ABI probe",
            envelope.family
        )));
    }
    if envelope.probe_hash != combined_probe_hash(&envelope.target_triple, &expected_sources) {
        return Err(message(format!(
            "probe hash mismatch for {}",
            envelope.family
        )));
    }
    if envelope.payload_hash != sha256(&serde_json::to_vec(&envelope.payload)?) {
        return Err(message(format!(
            "payload hash mismatch for {}",
            envelope.family
        )));
    }
    validate_payload(&envelope.payload, &inventory)
}

fn write_envelope_archive(path: &Path, envelope: &Envelope) -> Result<(), DynError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tar.zst.tmp-{}", std::process::id()));
    let file = File::create(&temporary)?;
    let encoder = zstd::Encoder::new(file, 19)?.auto_finish();
    let mut archive = tar::Builder::new(encoder);
    let bytes = serde_json::to_vec_pretty(envelope)?;
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    archive.append_data(&mut header, "core-abi-envelope.json", Cursor::new(bytes))?;
    archive.finish()?;
    drop(archive);
    std::fs::rename(temporary, path)?;
    Ok(())
}

fn read_envelope_archive(path: &Path) -> Result<Envelope, DynError> {
    let decoder = zstd::Decoder::new(File::open(path)?)?;
    let mut archive = tar::Archive::new(decoder);
    let mut entries = archive.entries()?;
    let mut entry = entries
        .next()
        .ok_or_else(|| message(format!("empty probe envelope: {}", path.display())))??;
    if entry.path()?.as_ref() != Path::new("core-abi-envelope.json")
        || !entry.header().entry_type().is_file()
    {
        return Err(message(format!(
            "unexpected probe envelope entry in {}",
            path.display()
        )));
    }
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    drop(entry);
    if entries.next().is_some() {
        return Err(message(format!(
            "probe envelope contains extra entries: {}",
            path.display()
        )));
    }
    Ok(serde_json::from_slice(&bytes)?)
}
