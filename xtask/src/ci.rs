//! Validation and evidence tooling for the canonical GitHub Actions matrix.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

const DEFAULT_WORKFLOW_DIR: &str = ".github/workflows";
const DEFAULT_MATRIX: &str = "docs/conformance/ci-matrix.md";

#[derive(Debug, Deserialize)]
struct CanonicalMatrix {
    #[serde(default)]
    enforce_policy: bool,
    job: Vec<CanonicalJob>,
}

#[derive(Debug, Deserialize)]
struct CanonicalJob {
    workflow: String,
    id: String,
    #[serde(default)]
    required: Vec<String>,
    #[serde(default = "default_one")]
    evidence_count: usize,
}

/// One schema-versioned CI gate result.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceFragment {
    schema: u32,
    repository: String,
    head_sha: String,
    workflow: String,
    workflow_run_id: String,
    job_id: String,
    target: String,
    toolchain: String,
    command: String,
    conclusion: String,
    input_hashes: BTreeMap<String, String>,
    output_hashes: BTreeMap<String, String>,
}

/// Runs the `cargo xtask ci` command.
pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), String> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("--help" | "-h") => {
            println!(
                "usage: cargo xtask ci <validate|list-jobs|validate-evidence|bundle-evidence>"
            );
            Ok(())
        }
        Some("validate") => {
            let (workflow_dir, matrix) = parse_paths(args)?;
            validate_paths(&workflow_dir, &matrix)
        }
        Some("list-jobs") => {
            let (workflow_dir, matrix) = parse_paths(args)?;
            for job in list_jobs_paths(&workflow_dir, &matrix)? {
                println!("{job}");
            }
            Ok(())
        }
        Some("validate-evidence") => {
            let path = required_argument(&mut args, "evidence path")?;
            reject_extra(args)?;
            validate_evidence_path(Path::new(&path)).map(|_| ())
        }
        Some("bundle-evidence") => {
            let mut input = None;
            let mut output = None;
            let mut head_sha = None;
            let mut matrix = None;
            let mut source_bundle = None;
            while let Some(argument) = args.next() {
                let value = required_argument(&mut args, &format!("value for {argument}"))?;
                match argument.as_str() {
                    "--input" => input = Some(PathBuf::from(value)),
                    "--output" => output = Some(PathBuf::from(value)),
                    "--head-sha" => head_sha = Some(value),
                    "--matrix" => matrix = Some(PathBuf::from(value)),
                    "--source-bundle" => source_bundle = Some(PathBuf::from(value)),
                    _ => return Err(format!("unknown bundle-evidence option: {argument}")),
                }
            }
            bundle_evidence_inner(
                &input.ok_or_else(|| "bundle-evidence requires --input".to_owned())?,
                &output.ok_or_else(|| "bundle-evidence requires --output".to_owned())?,
                &head_sha.ok_or_else(|| "bundle-evidence requires --head-sha".to_owned())?,
                matrix.as_deref(),
                source_bundle.as_deref(),
            )
        }
        Some(command) => Err(format!("unknown ci command: {command}")),
        None => Err("expected a ci command (try `ci --help`)".to_owned()),
    }
}

/// Validates all checked-in workflows against the canonical matrix document.
pub fn validate_paths(workflow_dir: &Path, matrix_path: &Path) -> Result<(), String> {
    let release_workflow = workflow_dir.join("release.yml");
    if release_workflow.exists() {
        return Err(format!(
            "{}: CI release workflows are forbidden; releases are manual",
            release_workflow.display()
        ));
    }
    validate_no_release_operations(workflow_dir)?;
    let matrix = read_matrix(matrix_path)?;
    let mut expected_by_workflow: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut parsed = BTreeMap::new();

    for expected in &matrix.job {
        let expected_jobs = expected_by_workflow.entry(&expected.workflow).or_default();
        if !expected_jobs.insert(&expected.id) {
            return Err(format!(
                "canonical matrix repeats {}:{}",
                expected.workflow, expected.id
            ));
        }
        let path = workflow_dir.join(&expected.workflow);
        if !parsed.contains_key(&expected.workflow) {
            parsed.insert(expected.workflow.clone(), read_workflow(&path)?);
        }
        let workflow = &parsed[&expected.workflow];
        let jobs = workflow_jobs(workflow, &path)?;
        let job_key = YamlValue::String(expected.id.clone());
        let job = jobs
            .get(&job_key)
            .ok_or_else(|| format!("{}: missing required job {}", path.display(), expected.id))?;
        let rendered = serde_yaml::to_string(job)
            .map_err(|error| format!("{}:{}: {error}", path.display(), expected.id))?;
        if rendered.contains("cargo publish") {
            return Err(format!(
                "{}:{}: cargo publish is forbidden in validation workflows; releases are manual",
                path.display(),
                expected.id
            ));
        }
        for token in &expected.required {
            if !rendered.contains(token) {
                return Err(format!(
                    "{}:{}: missing required token {token:?}",
                    path.display(),
                    expected.id
                ));
            }
        }
    }

    if !matrix.enforce_policy {
        return Ok(());
    }

    for (workflow_name, expected_jobs) in expected_by_workflow {
        let path = workflow_dir.join(workflow_name);
        let workflow = &parsed[workflow_name];
        let jobs = workflow_jobs(workflow, &path)?;
        let actual_jobs: BTreeSet<&str> = jobs
            .keys()
            .map(|key| {
                key.as_str()
                    .ok_or_else(|| format!("{}: job ID is not a string", path.display()))
            })
            .collect::<Result<_, _>>()?;
        if actual_jobs != expected_jobs {
            return Err(format!(
                "{}: job set differs; expected {expected_jobs:?}, found {actual_jobs:?}",
                path.display()
            ));
        }

        for (job_id, job) in jobs {
            let job_id = job_id.as_str().expect("job IDs checked above");
            validate_job_policy(&path, job_id, job)?;
        }
    }

    let schema_path = matrix_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("evidence-schema.json");
    validate_schema_document(&schema_path)
}

fn validate_no_release_operations(workflow_dir: &Path) -> Result<(), String> {
    let mut paths = fs::read_dir(workflow_dir)
        .map_err(|error| format!("cannot read {}: {error}", workflow_dir.display()))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| format!("cannot read {} entry: {error}", workflow_dir.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    for path in paths {
        if !matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("yml" | "yaml")
        ) {
            continue;
        }
        let workflow = read_workflow(&path)?;
        let rendered = serde_yaml::to_string(&workflow)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        for forbidden in [
            "cargo publish",
            "cargo release",
            "release source-bundle",
            "git tag",
            "gh release",
            "cosign attest",
            "attest-build-provenance",
        ] {
            if rendered.contains(forbidden) {
                return Err(format!(
                    "{}: `{forbidden}` is forbidden in CI; releases are manual",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

/// Lists canonical workflow/job pairs in stable lexical order.
pub fn list_jobs_paths(workflow_dir: &Path, matrix_path: &Path) -> Result<Vec<String>, String> {
    let matrix = read_matrix(matrix_path)?;
    let mut seen = BTreeSet::new();
    for workflow in matrix
        .job
        .iter()
        .map(|job| workflow_dir.join(&job.workflow))
    {
        if seen.insert(workflow.clone()) {
            read_workflow(&workflow)?;
        }
    }
    let mut jobs: Vec<_> = matrix
        .job
        .into_iter()
        .map(|job| format!("{}:{}", job.workflow, job.id))
        .collect();
    jobs.sort();
    Ok(jobs)
}

/// Parses and validates one evidence fragment.
pub fn validate_evidence_path(path: &Path) -> Result<EvidenceFragment, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let fragment: EvidenceFragment = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: invalid evidence JSON: {error}", path.display()))?;
    validate_fragment(&fragment, path)?;
    Ok(fragment)
}

/// Creates a deterministic zstd-compressed tar containing same-SHA evidence.
pub fn bundle_evidence(input: &Path, output: &Path, head_sha: &str) -> Result<(), String> {
    bundle_evidence_inner(input, output, head_sha, None, None)
}

/// Returns the exact validation-evidence multiplicities from the canonical matrix.
pub fn expected_evidence_counts(matrix_path: &Path) -> Result<BTreeMap<String, usize>, String> {
    Ok(read_matrix(matrix_path)?
        .job
        .into_iter()
        .map(|job| (job.id, job.evidence_count))
        .collect())
}

fn bundle_evidence_inner(
    input: &Path,
    output: &Path,
    head_sha: &str,
    matrix_path: Option<&Path>,
    source_bundle: Option<&Path>,
) -> Result<(), String> {
    validate_sha(head_sha, "requested head SHA")?;
    let mut paths = Vec::new();
    collect_json(input, &mut paths)?;
    paths.sort();
    if paths.is_empty() {
        return Err(format!(
            "{} contains no evidence fragments",
            input.display()
        ));
    }

    let mut entries = Vec::new();
    let mut observed_jobs = BTreeMap::<String, usize>::new();
    for path in paths {
        let fragment = validate_evidence_path(&path)?;
        if fragment.head_sha != head_sha {
            return Err(format!(
                "{} is for {}, expected {head_sha}",
                path.display(),
                fragment.head_sha
            ));
        }
        if fragment.conclusion != "success" {
            return Err(format!(
                "{} records non-success conclusion {}",
                path.display(),
                fragment.conclusion
            ));
        }
        *observed_jobs.entry(fragment.job_id.clone()).or_default() += 1;
        let bytes = fs::read(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let name = format!(
            "{}-{}-{}.json",
            sanitize(&fragment.workflow),
            sanitize(&fragment.job_id),
            &hex_digest(&bytes)[..16]
        );
        entries.push((name, bytes));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    if let Some(matrix_path) = matrix_path {
        let expected_jobs = expected_evidence_counts(matrix_path)?;
        if observed_jobs != expected_jobs {
            return Err(format!(
                "evidence job set differs; expected {expected_jobs:?}, found {observed_jobs:?}"
            ));
        }
    }

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let file = File::create(output)
        .map_err(|error| format!("could not create {}: {error}", output.display()))?;
    let mut encoder = zstd::stream::write::Encoder::new(file, 19)
        .map_err(|error| format!("could not initialize zstd: {error}"))?;
    encoder
        .include_checksum(true)
        .map_err(|error| format!("could not configure zstd: {error}"))?;
    {
        let mut archive = tar::Builder::new(&mut encoder);
        for (name, bytes) in entries {
            append_tar_entry(&mut archive, &format!("evidence/{name}"), &bytes)?;
        }
        if let Some(source_bundle) = source_bundle {
            let name = source_bundle
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| "source bundle requires a UTF-8 file name".to_owned())?;
            let expected_name = format!("ara2-bridge-{}-source.tar.zst", crate::release::VERSION);
            if name != expected_name {
                return Err(format!(
                    "source bundle requires canonical source bundle filename {expected_name}"
                ));
            }
            crate::release::bundle::verify_for_commit(source_bundle, head_sha)?;
            let bytes = fs::read(source_bundle).map_err(|error| {
                format!(
                    "could not read source bundle {}: {error}",
                    source_bundle.display()
                )
            })?;
            let digest = format!("{}  {name}\n", hex_digest(&bytes));
            append_tar_entry(&mut archive, &format!("release/{name}"), &bytes)?;
            append_tar_entry(
                &mut archive,
                &format!("release/{name}.sha256"),
                digest.as_bytes(),
            )?;
        }
        archive
            .finish()
            .map_err(|error| format!("could not finish evidence tar: {error}"))?;
    }
    encoder
        .finish()
        .map_err(|error| format!("could not finish evidence archive: {error}"))?;
    Ok(())
}

fn validate_job_policy(path: &Path, job_id: &str, job: &YamlValue) -> Result<(), String> {
    let rendered = serde_yaml::to_string(job)
        .map_err(|error| format!("{}:{job_id}: {error}", path.display()))?;
    if !rendered.contains("ci/write-evidence.sh") {
        return Err(format!(
            "{}:{job_id}: gate does not emit schema-validated evidence",
            path.display()
        ));
    }
    if !rendered.contains("actions/upload-artifact@") {
        return Err(format!(
            "{}:{job_id}: evidence artifact is not uploaded",
            path.display()
        ));
    }
    let multi_toolchain = rendered.matches("dtolnay/rust-toolchain@").count() > 1;
    for line in rendered.lines() {
        if line.contains("bootstrap-reference-sdks.sh fetch") && !line.contains("--accept-license")
        {
            return Err(format!(
                "{}:{job_id}: SDK fetch lacks explicit --accept-license",
                path.display()
            ));
        }
        if line.contains("curl ") || line.contains("wget ") {
            return Err(format!(
                "{}:{job_id}: direct unpinned download is forbidden: {line}",
                path.display()
            ));
        }
        if multi_toolchain && line.contains("cargo install") && !line.contains("cargo +") {
            return Err(format!(
                "{}:{job_id}: job installs several toolchains, so cargo install must name one explicitly: {line}",
                path.display()
            ));
        }
    }
    validate_uses(path, job_id, job)
}

fn validate_uses(path: &Path, job_id: &str, value: &YamlValue) -> Result<(), String> {
    match value {
        YamlValue::Mapping(mapping) => {
            for (key, value) in mapping {
                if key.as_str() == Some("uses") {
                    let action = value.as_str().ok_or_else(|| {
                        format!("{}:{job_id}: uses value is not a string", path.display())
                    })?;
                    if !action.starts_with("./") {
                        let (_, revision) = action.rsplit_once('@').ok_or_else(|| {
                            format!("{}:{job_id}: unpinned action {action}", path.display())
                        })?;
                        if revision.len() != 40
                            || !revision.bytes().all(|byte| byte.is_ascii_hexdigit())
                        {
                            return Err(format!(
                                "{}:{job_id}: action must use an immutable 40-hex revision: {action}",
                                path.display()
                            ));
                        }
                    }
                }
                validate_uses(path, job_id, value)?;
            }
        }
        YamlValue::Sequence(sequence) => {
            for value in sequence {
                validate_uses(path, job_id, value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_schema_document(path: &Path) -> Result<(), String> {
    let document: JsonValue = serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("{}: invalid JSON schema: {error}", path.display()))?;
    let required = document
        .get("required")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| format!("{}: schema lacks required array", path.display()))?;
    for field in [
        "repository",
        "head_sha",
        "workflow",
        "workflow_run_id",
        "job_id",
        "target",
        "toolchain",
        "command",
        "conclusion",
        "input_hashes",
        "output_hashes",
    ] {
        if !required.iter().any(|value| value.as_str() == Some(field)) {
            return Err(format!(
                "{}: schema does not require {field}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn validate_fragment(fragment: &EvidenceFragment, path: &Path) -> Result<(), String> {
    if fragment.schema != 1 {
        return Err(format!("{}: unsupported evidence schema", path.display()));
    }
    for (name, value) in [
        ("repository", fragment.repository.as_str()),
        ("workflow", fragment.workflow.as_str()),
        ("workflow_run_id", fragment.workflow_run_id.as_str()),
        ("job_id", fragment.job_id.as_str()),
        ("target", fragment.target.as_str()),
        ("toolchain", fragment.toolchain.as_str()),
        ("command", fragment.command.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{}: {name} is empty", path.display()));
        }
    }
    validate_sha(&fragment.head_sha, "evidence head SHA")?;
    if !matches!(
        fragment.conclusion.as_str(),
        "success" | "failure" | "cancelled"
    ) {
        return Err(format!(
            "{}: invalid conclusion {}",
            path.display(),
            fragment.conclusion
        ));
    }
    for (name, hash) in fragment.input_hashes.iter().chain(&fragment.output_hashes) {
        if name.is_empty() || hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(format!("{}: invalid hash for {name:?}", path.display()));
        }
    }
    Ok(())
}

fn parse_paths(args: impl IntoIterator<Item = String>) -> Result<(PathBuf, PathBuf), String> {
    let mut workflow_dir = PathBuf::from(DEFAULT_WORKFLOW_DIR);
    let mut matrix = PathBuf::from(DEFAULT_MATRIX);
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--workflow-dir" => {
                workflow_dir = PathBuf::from(required_argument(&mut args, "workflow path")?);
            }
            "--matrix" => {
                matrix = PathBuf::from(required_argument(&mut args, "matrix path")?);
            }
            _ => return Err(format!("unknown ci option: {argument}")),
        }
    }
    Ok((workflow_dir, matrix))
}

fn read_matrix(path: &Path) -> Result<CanonicalMatrix, String> {
    let markdown = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let start_marker = "<!-- ci-matrix\n";
    let start = markdown
        .find(start_marker)
        .ok_or_else(|| format!("{}: missing ci-matrix block", path.display()))?
        + start_marker.len();
    let end = markdown[start..]
        .find("\n-->")
        .ok_or_else(|| format!("{}: unterminated ci-matrix block", path.display()))?
        + start;
    toml::from_str(&markdown[start..end])
        .map_err(|error| format!("{}: invalid ci-matrix block: {error}", path.display()))
}

fn read_workflow(path: &Path) -> Result<YamlValue, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    serde_yaml::from_str(&source)
        .map_err(|error| format!("{}: invalid workflow YAML: {error}", path.display()))
}

fn workflow_jobs<'a>(
    workflow: &'a YamlValue,
    path: &Path,
) -> Result<&'a serde_yaml::Mapping, String> {
    workflow
        .get("jobs")
        .and_then(YamlValue::as_mapping)
        .ok_or_else(|| format!("{}: missing jobs mapping", path.display()))
}

fn required_argument(
    args: &mut impl Iterator<Item = String>,
    description: &str,
) -> Result<String, String> {
    args.next().ok_or_else(|| format!("missing {description}"))
}

fn reject_extra(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    match args.next() {
        Some(argument) => Err(format!("unexpected argument: {argument}")),
        None => Ok(()),
    }
}

fn validate_sha(value: &str, description: &str) -> Result<(), String> {
    if value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!("{description} is not a 40-hex Git SHA: {value}"))
    }
}

fn collect_json(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("could not read {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("could not read {} entry: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            collect_json(&path, output)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            output.push(path);
        }
    }
    Ok(())
}

fn append_tar_entry<W: Write>(
    archive: &mut tar::Builder<W>,
    name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    archive
        .append_data(&mut header, name, bytes)
        .map_err(|error| format!("could not append {name}: {error}"))
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

const fn default_one() -> usize {
    1
}
