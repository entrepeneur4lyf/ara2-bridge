//! Documentation and manual-source validation.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAP_START: &str = "```toml manual-source-map";
const CHAPTER_COUNT: u8 = 12;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManualMap {
    schema: u32,
    chapter: Vec<Chapter>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Chapter {
    number: u8,
    title: String,
    normative_specs: Vec<String>,
    public_apis: Vec<String>,
    #[serde(default)]
    examples: Vec<String>,
    #[serde(default)]
    example_reason: Option<String>,
    conformance_commands: Vec<String>,
    testhost_args: Vec<String>,
    companion_binaries: Vec<String>,
    sdk_environment: Vec<String>,
    required_capabilities: Vec<String>,
    expected_skips: u32,
    fixture_hashes: Vec<String>,
    platform_steps: Vec<String>,
    gui_main_loop: Vec<String>,
    timeouts: Vec<String>,
    troubleshooting: Vec<String>,
}

/// Runs the `cargo xtask docs` command family.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    match args.next().as_deref() {
        Some("--help") => Ok(()),
        Some("verify-manual-map") => {
            if args.next().is_some() {
                return Err("docs verify-manual-map takes no arguments".to_owned());
            }
            let root = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("xtask is a workspace child");
            verify_manual_map_path(root, &root.join("docs/manual-source-map.md"))
        }
        Some("verify-public-docs") => {
            if args.next().is_some() {
                return Err("docs verify-public-docs takes no arguments".to_owned());
            }
            let root = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("xtask is a workspace child");
            verify_public_docs_path(root)
        }
        Some(command) => Err(format!("unknown docs command: {command}")),
        None => Err("docs requires --help, verify-manual-map, or verify-public-docs".to_owned()),
    }
}

/// Validates all publishable crate-root documentation contracts.
pub fn verify_public_docs_path(root: &Path) -> Result<(), String> {
    let generated = root.join("ara2-bridge-sys/src/generated");
    let mut generated_sources = Vec::new();
    collect_rust_sources(&generated, &mut generated_sources)?;
    let mut known_symbols = BTreeSet::new();
    for path in generated_sources {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        for token in
            source.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        {
            if token.starts_with("ARA") || token.starts_with("kARA") {
                known_symbols.insert(token.to_owned());
            }
        }
    }
    let known: Vec<_> = known_symbols.iter().map(String::as_str).collect();
    for crate_name in [
        "ara2-bridge-sys",
        "ara2-bridge-core",
        "ara2-bridge-plugin",
        "ara2-bridge-host",
        "ara2-bridge-companion",
        "ara2-bridge-testkit",
        "ara2-bridge",
    ] {
        let path = root.join(crate_name).join("src/lib.rs");
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        verify_crate_root_contract(crate_name, &source, &known)?;
        if crate_name != "ara2-bridge-sys" && !source.contains("#![deny(missing_docs)]") {
            return Err(format!(
                "{crate_name} must deny missing public-item documentation"
            ));
        }
        if !source.contains("#![deny(clippy::missing_safety_doc)]") {
            return Err(format!("{crate_name} must deny missing # Safety contracts"));
        }
    }
    Ok(())
}

/// Validates one crate-root documentation source against known upstream C symbols.
pub fn verify_crate_root_contract(
    crate_name: &str,
    source: &str,
    known_symbols: &[&str],
) -> Result<(), String> {
    let docs = source
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("//!"))
        .collect::<Vec<_>>()
        .join("\n");
    for section in [
        "# Role and boundaries",
        "# Lifecycle",
        "# Features and platforms",
        "# Compatibility and licensing",
        "# Example",
    ] {
        if !docs.contains(section) {
            return Err(format!("{crate_name} crate docs are missing `{section}`"));
        }
    }
    if !docs.contains("https://github.com/Celemony/ARA_API")
        && !docs.contains("https://github.com/Celemony/ARA_SDK")
    {
        return Err(format!("{crate_name} crate docs must link upstream ARA"));
    }
    let normalized = docs.split_whitespace().collect::<Vec<_>>().join(" ");
    if !normalized.contains("No direct C counterpart")
        && !normalized
            .to_ascii_lowercase()
            .contains("direct ara c counterpart")
    {
        return Err(format!(
            "{crate_name} crate docs need direct-C or `No direct C counterpart` classification"
        ));
    }

    let marker = "Direct ARA C counterpart: `";
    let mut rest = normalized.as_str();
    while let Some(start) = rest.find(marker) {
        let after = &rest[start + marker.len()..];
        let end = after
            .find('`')
            .ok_or_else(|| format!("{crate_name} has an unterminated direct-C classification"))?;
        let symbol = &after[..end];
        if !known_symbols.contains(&symbol) {
            return Err(format!(
                "{crate_name} names fabricated ARA C symbol `{symbol}`"
            ));
        }
        rest = &after[end + 1..];
    }
    Ok(())
}

fn collect_rust_sources(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("cannot read {} entry: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            collect_rust_sources(&path, output)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            output.push(path);
        }
    }
    output.sort();
    Ok(())
}

/// Validates one manual source map against files rooted at `root`.
pub fn verify_manual_map_path(root: &Path, map_path: &Path) -> Result<(), String> {
    let markdown = fs::read_to_string(map_path)
        .map_err(|error| format!("cannot read {}: {error}", map_path.display()))?;
    let encoded = embedded_toml(&markdown)?;
    let map: ManualMap = toml::from_str(encoded)
        .map_err(|error| format!("invalid manual source map TOML: {error}"))?;
    if map.schema != 1 {
        return Err(format!(
            "unsupported manual source map schema {}",
            map.schema
        ));
    }

    let mut numbers = BTreeSet::new();
    for chapter in &map.chapter {
        if !(1..=CHAPTER_COUNT).contains(&chapter.number) {
            return Err(format!("invalid chapter number {}", chapter.number));
        }
        if !numbers.insert(chapter.number) {
            return Err(format!("duplicate chapter {}", chapter.number));
        }
        validate_chapter(root, chapter)?;
    }
    for number in 1..=CHAPTER_COUNT {
        if !numbers.contains(&number) {
            return Err(format!("missing chapter {number}"));
        }
    }
    Ok(())
}

fn embedded_toml(markdown: &str) -> Result<&str, String> {
    let start = markdown
        .find(MAP_START)
        .ok_or_else(|| format!("missing `{MAP_START}` block"))?;
    let after_marker = &markdown[start + MAP_START.len()..];
    let body = after_marker
        .strip_prefix('\n')
        .or_else(|| after_marker.strip_prefix("\r\n"))
        .ok_or_else(|| "manual source map marker must end the line".to_owned())?;
    let end = body
        .find("\n```")
        .ok_or_else(|| "unterminated manual source map block".to_owned())?;
    Ok(&body[..end])
}

fn validate_chapter(root: &Path, chapter: &Chapter) -> Result<(), String> {
    let label = format!("chapter {}", chapter.number);
    require_text(&chapter.title, &label, "title")?;
    require_nonempty(&chapter.normative_specs, &label, "normative_specs")?;
    require_nonempty(&chapter.public_apis, &label, "public_apis")?;
    require_nonempty(
        &chapter.conformance_commands,
        &label,
        "conformance_commands",
    )?;
    require_nonempty(&chapter.testhost_args, &label, "testhost_args")?;
    require_nonempty(&chapter.companion_binaries, &label, "companion_binaries")?;
    require_nonempty(&chapter.sdk_environment, &label, "sdk_environment")?;
    require_nonempty(
        &chapter.required_capabilities,
        &label,
        "required_capabilities",
    )?;
    require_nonempty(&chapter.fixture_hashes, &label, "fixture_hashes")?;
    require_nonempty(&chapter.platform_steps, &label, "platform_steps")?;
    require_nonempty(&chapter.gui_main_loop, &label, "gui_main_loop")?;
    require_nonempty(&chapter.timeouts, &label, "timeouts")?;
    require_nonempty(&chapter.troubleshooting, &label, "troubleshooting")?;

    match (
        chapter.examples.is_empty(),
        chapter.example_reason.as_deref(),
    ) {
        (true, None | Some("")) => {
            return Err(format!("{label} requires an example or example_reason"))
        }
        (false, Some(_)) => {
            return Err(format!(
                "{label} cannot set example_reason when examples are present"
            ))
        }
        _ => {}
    }

    for spec in &chapter.normative_specs {
        require_file(root, spec, &label, "normative spec")?;
    }
    for example in &chapter.examples {
        require_file(root, example, &label, "example")?;
        if !example.ends_with(".rs") {
            return Err(format!("{label} example must be a Rust source: {example}"));
        }
    }
    for api in &chapter.public_apis {
        if !api.contains("::") || api.chars().any(char::is_whitespace) {
            return Err(format!("{label} invalid public API reference: {api}"));
        }
    }
    for command in &chapter.conformance_commands {
        validate_command(root, &label, command)?;
    }
    for fixture in &chapter.fixture_hashes {
        validate_fixture_hash(root, &label, fixture)?;
    }
    for reference in &chapter.troubleshooting {
        let (path, anchor) = reference.split_once('#').ok_or_else(|| {
            format!("{label} troubleshooting reference needs an anchor: {reference}")
        })?;
        require_file(root, path, &label, "troubleshooting reference")?;
        validate_markdown_anchor(root, &label, path, anchor)?;
    }

    let _ = chapter.expected_skips;
    Ok(())
}

fn validate_fixture_hash(root: &Path, label: &str, fixture: &str) -> Result<(), String> {
    if let Some(reason) = fixture.strip_prefix("not-applicable:") {
        return require_text(reason, label, "fixture_hashes not-applicable reason");
    }
    let (path, expected) = fixture
        .split_once('@')
        .ok_or_else(|| format!("{label} fixture hash must use path@sha256: {fixture}"))?;
    require_file(root, path, label, "fixture")?;
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{label} invalid fixture SHA-256: {fixture}"));
    }
    let bytes = fs::read(root.join(path))
        .map_err(|error| format!("cannot read fixture {path}: {error}"))?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(format!(
            "{label} fixture hash mismatch for {path}: expected {expected}, found {actual}"
        ));
    }
    Ok(())
}

fn validate_markdown_anchor(
    root: &Path,
    label: &str,
    path: &str,
    expected: &str,
) -> Result<(), String> {
    let source = fs::read_to_string(root.join(path))
        .map_err(|error| format!("cannot read troubleshooting reference {path}: {error}"))?;
    let present = source.lines().filter_map(|line| {
        let heading = line
            .trim_start()
            .strip_prefix('#')?
            .trim_start_matches('#')
            .trim();
        (!heading.is_empty()).then(|| markdown_anchor(heading))
    });
    if present.into_iter().any(|anchor| anchor == expected) {
        Ok(())
    } else {
        Err(format!(
            "{label} missing troubleshooting anchor #{expected} in {path}"
        ))
    }
}

fn markdown_anchor(heading: &str) -> String {
    let mut anchor = String::new();
    for character in heading.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            anchor.push(character);
        } else if character.is_whitespace() && !anchor.ends_with('-') {
            anchor.push('-');
        }
    }
    anchor.trim_matches('-').to_owned()
}

fn require_nonempty(values: &[String], label: &str, field: &str) -> Result<(), String> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        Err(format!("{label} requires non-empty {field}"))
    } else {
        Ok(())
    }
}

fn require_text(value: &str, label: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} requires non-empty {field}"))
    } else {
        Ok(())
    }
}

fn require_file(root: &Path, relative: &str, label: &str, kind: &str) -> Result<(), String> {
    let path = safe_relative_path(relative)
        .ok_or_else(|| format!("{label} invalid {kind} path: {relative}"))?;
    if root.join(&path).is_file() {
        Ok(())
    } else {
        Err(format!("{label} missing {kind}: {relative}"))
    }
}

fn safe_relative_path(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return None;
    }
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return None;
    }
    Some(path.to_owned())
}

fn validate_command(root: &Path, label: &str, command: &str) -> Result<(), String> {
    let words: Vec<_> = command.split_whitespace().collect();
    let executable = words
        .iter()
        .copied()
        .find(|word| !word.contains('='))
        .unwrap_or_default();
    match executable {
        "cargo" => validate_cargo_target(root, label, &words, command),
        "go" => Ok(()),
        path if path.starts_with("ci/") || path.starts_with("scripts/") => {
            require_file(root, path, label, "command executable")
        }
        _ => Err(format!("{label} invalid conformance command: {command}")),
    }
}

fn validate_cargo_target(
    root: &Path,
    label: &str,
    words: &[&str],
    command: &str,
) -> Result<(), String> {
    let package = argument_after(words, "-p").or_else(|| argument_after(words, "--package"));
    if let Some(target) = argument_after(words, "--test") {
        let package = package.ok_or_else(|| {
            format!("{label} cargo --test command must name -p/--package: {command}")
        })?;
        let path = root
            .join(package)
            .join("tests")
            .join(format!("{target}.rs"));
        if !path.is_file() {
            return Err(format!(
                "{label} missing cargo test target {package}/tests/{target}.rs"
            ));
        }
    }
    if let Some(target) = argument_after(words, "--example") {
        let package = package.ok_or_else(|| {
            format!("{label} cargo --example command must name -p/--package: {command}")
        })?;
        let path = root
            .join(package)
            .join("examples")
            .join(format!("{target}.rs"));
        if !path.is_file() {
            return Err(format!(
                "{label} missing cargo example target {package}/examples/{target}.rs"
            ));
        }
    }
    Ok(())
}

fn argument_after<'a>(words: &'a [&str], flag: &str) -> Option<&'a str> {
    words
        .iter()
        .position(|word| *word == flag)
        .and_then(|index| words.get(index + 1))
        .copied()
}
