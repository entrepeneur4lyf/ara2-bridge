use std::fs;
use std::path::Path;

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
