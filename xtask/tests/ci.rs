use std::fs;
use std::path::Path;
use std::process::Command;

const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

fn write_matrix(path: &Path) {
    fs::write(
        path,
        "# Matrix\n\n<!-- ci-matrix\n[[job]]\nworkflow = \"ci.yml\"\nid = \"quality\"\n-->\n",
    )
    .unwrap();
}

#[test]
fn ci_help_is_registered() {
    xtask::run(["ci".to_owned(), "--help".to_owned()]).unwrap();
}

#[test]
fn malformed_workflow_is_diagnosed() {
    let temp = tempfile::tempdir().unwrap();
    let workflows = temp.path().join("workflows");
    fs::create_dir(&workflows).unwrap();
    fs::write(workflows.join("ci.yml"), "jobs: [").unwrap();
    let matrix = temp.path().join("matrix.md");
    write_matrix(&matrix);

    let error = xtask::ci::validate_paths(&workflows, &matrix).unwrap_err();
    assert!(error.contains("invalid workflow YAML"), "{error}");
}

#[test]
fn missing_job_names_the_required_job() {
    let temp = tempfile::tempdir().unwrap();
    let workflows = temp.path().join("workflows");
    fs::create_dir(&workflows).unwrap();
    fs::write(
        workflows.join("ci.yml"),
        "jobs:\n  other:\n    runs-on: ubuntu-latest\n",
    )
    .unwrap();
    let matrix = temp.path().join("matrix.md");
    write_matrix(&matrix);

    let error = xtask::ci::validate_paths(&workflows, &matrix).unwrap_err();
    assert!(error.contains("missing required job quality"), "{error}");
}

#[test]
fn canonical_matrix_validates() {
    let temp = tempfile::tempdir().unwrap();
    let workflows = temp.path().join("workflows");
    fs::create_dir(&workflows).unwrap();
    fs::write(
        workflows.join("ci.yml"),
        "jobs:\n  quality:\n    runs-on: ubuntu-latest\n",
    )
    .unwrap();
    let matrix = temp.path().join("matrix.md");
    write_matrix(&matrix);

    xtask::ci::validate_paths(&workflows, &matrix).unwrap();
}

#[test]
fn checked_in_matrix_validates_and_preserves_phase_zero_jobs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let jobs = xtask::ci::list_jobs_paths(
        &root.join(".github/workflows"),
        &root.join("docs/conformance/ci-matrix.md"),
    )
    .unwrap();
    assert!(jobs.contains(&"ci.yml:quality".to_owned()));
    assert!(jobs.contains(&"ci.yml:phase0-core-probe".to_owned()));
    xtask::ci::validate_paths(
        &root.join(".github/workflows"),
        &root.join("docs/conformance/ci-matrix.md"),
    )
    .unwrap();
}

#[test]
fn checked_in_automation_has_no_release_workflow() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    assert!(
        !root.join(".github/workflows/release.yml").exists(),
        "release artifacts must be created only by the manual local procedure"
    );

    let jobs = xtask::ci::list_jobs_paths(
        &root.join(".github/workflows"),
        &root.join("docs/conformance/ci-matrix.md"),
    )
    .unwrap();
    assert_eq!(
        jobs.len(),
        13,
        "the canonical matrix contains validation jobs only"
    );
    assert!(jobs.iter().all(|job| !job.starts_with("release.yml:")));
}

#[test]
fn validator_rejects_a_ci_release_workflow() {
    let temp = tempfile::tempdir().unwrap();
    let workflows = temp.path().join("workflows");
    fs::create_dir(&workflows).unwrap();
    fs::write(
        workflows.join("ci.yml"),
        "jobs:\n  quality:\n    runs-on: ubuntu-latest\n",
    )
    .unwrap();
    fs::write(
        workflows.join("release.yml"),
        "jobs:\n  publish:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo publish\n",
    )
    .unwrap();
    let matrix = temp.path().join("matrix.md");
    write_matrix(&matrix);

    let error = xtask::ci::validate_paths(&workflows, &matrix).unwrap_err();
    assert!(error.contains("release workflow"), "{error}");
}

#[test]
fn validator_rejects_release_operations_in_unlisted_workflow_names() {
    let temp = tempfile::tempdir().unwrap();
    let workflows = temp.path().join("workflows");
    fs::create_dir(&workflows).unwrap();
    fs::write(
        workflows.join("ci.yml"),
        "jobs:\n  quality:\n    runs-on: ubuntu-latest\n",
    )
    .unwrap();
    fs::write(
        workflows.join("publish.yaml"),
        "jobs:\n  bundle:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo xtask release source-bundle --version 0.2.0-alpha.1 --output candidate.tar.zst\n",
    )
    .unwrap();
    let matrix = temp.path().join("matrix.md");
    write_matrix(&matrix);

    let error = xtask::ci::validate_paths(&workflows, &matrix).unwrap_err();
    assert!(error.contains("source-bundle"), "{error}");
}

#[test]
fn validator_rejects_cargo_publish_in_validation_workflows() {
    let temp = tempfile::tempdir().unwrap();
    let workflows = temp.path().join("workflows");
    fs::create_dir(&workflows).unwrap();
    fs::write(
        workflows.join("ci.yml"),
        "jobs:\n  quality:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo publish\n",
    )
    .unwrap();
    let matrix = temp.path().join("matrix.md");
    write_matrix(&matrix);

    let error = xtask::ci::validate_paths(&workflows, &matrix).unwrap_err();
    assert!(error.contains("cargo publish"), "{error}");
}

#[test]
fn evidence_bundle_is_deterministic_and_rejects_mixed_commits() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    write_fragment(&input.join("one.json"), HEAD, "quality");
    write_fragment(&input.join("two.json"), HEAD, "miri");
    let first = temp.path().join("first.tar.zst");
    let second = temp.path().join("second.tar.zst");
    xtask::ci::bundle_evidence(&input, &first, HEAD).unwrap();
    xtask::ci::bundle_evidence(&input, &second, HEAD).unwrap();
    assert_eq!(fs::read(first).unwrap(), fs::read(&second).unwrap());

    write_fragment(
        &input.join("mixed.json"),
        "fedcba9876543210fedcba9876543210fedcba98",
        "other",
    );
    let error = xtask::ci::bundle_evidence(&input, &second, HEAD).unwrap_err();
    assert!(error.contains("expected 0123456789abcdef"), "{error}");
}

#[test]
fn release_evidence_bundle_rejects_an_invalid_source_bundle() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    write_fragment(&input.join("one.json"), HEAD, "quality");
    let source = temp.path().join("ara2-bridge-0.2.0-alpha.1-source.tar.zst");
    fs::write(&source, b"source bundle bytes").unwrap();
    let output = temp.path().join("evidence.tar.zst");
    let error = xtask::ci::run([
        "bundle-evidence".to_owned(),
        "--input".to_owned(),
        input.display().to_string(),
        "--output".to_owned(),
        output.display().to_string(),
        "--head-sha".to_owned(),
        HEAD.to_owned(),
        "--source-bundle".to_owned(),
        source.display().to_string(),
    ])
    .unwrap_err();

    assert!(error.contains("invalid source archive"), "{error}");
}

#[test]
fn release_evidence_bundle_requires_the_canonical_source_bundle_name() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    write_fragment(&input.join("one.json"), HEAD, "quality");
    let source = temp.path().join("source.tar.zst");
    fs::write(&source, b"source bundle bytes").unwrap();
    let output = temp.path().join("evidence.tar.zst");

    let error = xtask::ci::run([
        "bundle-evidence".to_owned(),
        "--input".to_owned(),
        input.display().to_string(),
        "--output".to_owned(),
        output.display().to_string(),
        "--head-sha".to_owned(),
        HEAD.to_owned(),
        "--source-bundle".to_owned(),
        source.display().to_string(),
    ])
    .unwrap_err();

    assert!(
        error.contains("canonical source bundle filename"),
        "{error}"
    );
}

#[test]
fn release_evidence_bundle_accepts_only_a_verified_matching_candidate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let head = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(head.status.success());
    let head = String::from_utf8(head.stdout).unwrap().trim().to_owned();
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    write_fragment(&input.join("one.json"), &head, "quality");
    let source = temp.path().join("ara2-bridge-0.2.0-alpha.1-source.tar.zst");
    xtask::release::create_source_bundle(root, &source, true).unwrap();
    let output = temp.path().join("evidence.tar.zst");

    xtask::ci::run([
        "bundle-evidence".to_owned(),
        "--input".to_owned(),
        input.display().to_string(),
        "--output".to_owned(),
        output.display().to_string(),
        "--head-sha".to_owned(),
        head,
        "--source-bundle".to_owned(),
        source.display().to_string(),
    ])
    .unwrap();
    assert!(output.is_file());

    write_fragment(&input.join("one.json"), HEAD, "quality");
    let error = xtask::ci::run([
        "bundle-evidence".to_owned(),
        "--input".to_owned(),
        input.display().to_string(),
        "--output".to_owned(),
        output.display().to_string(),
        "--head-sha".to_owned(),
        HEAD.to_owned(),
        "--source-bundle".to_owned(),
        source.display().to_string(),
    ])
    .unwrap_err();
    assert!(error.contains("candidate commit mismatch"), "{error}");
}

fn write_fragment(path: &Path, head_sha: &str, job_id: &str) {
    let fragment = serde_json::json!({
        "schema": 1,
        "repository": "owner/repository",
        "head_sha": head_sha,
        "workflow": "CI",
        "workflow_run_id": "42",
        "job_id": job_id,
        "target": "x86_64-unknown-linux-gnu",
        "toolchain": "stable",
        "command": "cargo test",
        "conclusion": "success",
        "input_hashes": {"Cargo.lock": "0".repeat(64)},
        "output_hashes": {}
    });
    fs::write(path, serde_json::to_vec_pretty(&fragment).unwrap()).unwrap();
}
