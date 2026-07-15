use super::{bundle, VERSION};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Deserialize)]
struct EvidenceFragment {
    schema: u32,
    repository: String,
    head_sha: String,
    workflow: String,
    job_id: String,
    target: String,
    conclusion: String,
}

pub(super) fn verify_archive(
    archive_path: &Path,
    commit: &str,
    matrix_path: &Path,
) -> Result<(), String> {
    let parent = archive_path
        .parent()
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| Path::new("."));
    let temp = tempfile::Builder::new()
        .prefix("ara2-evidence-verify-")
        .tempdir_in(parent)
        .or_else(|_| {
            tempfile::Builder::new()
                .prefix("ara2-evidence-verify-")
                .tempdir()
        })
        .map_err(|error| format!("cannot create evidence verification directory: {error}"))?;
    extract(archive_path, temp.path())?;

    let evidence = temp.path().join("evidence");
    let mut fragments = Vec::new();
    if evidence.is_dir() {
        let mut entries = fs::read_dir(&evidence)
            .map_err(|error| format!("cannot read {}: {error}", evidence.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("cannot enumerate evidence fragments: {error}"))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") || !path.is_file()
            {
                return Err(format!("unexpected evidence entry {}", path.display()));
            }
            let bytes = fs::read(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            let fragment: EvidenceFragment = serde_json::from_slice(&bytes)
                .map_err(|error| format!("invalid {}: {error}", path.display()))?;
            fragments.push(fragment);
        }
    }
    if fragments.len() != 40 {
        return Err(format!(
            "release requires exactly 40 evidence fragments, found {}",
            fragments.len()
        ));
    }
    let mut identities = BTreeSet::new();
    let mut observed = BTreeMap::<String, usize>::new();
    for fragment in fragments {
        if fragment.schema != 1
            || fragment.repository != "entrepeneur4lyf/ara2-bridge"
            || fragment.head_sha != commit
            || fragment.conclusion != "success"
        {
            return Err(format!(
                "evidence fragment identity or conclusion mismatch for {}/{}",
                fragment.workflow, fragment.job_id
            ));
        }
        let job_id = fragment.job_id.clone();
        if !identities.insert((fragment.workflow, fragment.job_id, fragment.target)) {
            return Err("duplicate evidence fragment identity".to_owned());
        }
        *observed.entry(job_id).or_default() += 1;
    }
    let expected = crate::ci::expected_evidence_counts(matrix_path)?;
    if observed != expected {
        return Err(format!(
            "evidence job set differs from canonical matrix; expected {expected:?}, found {observed:?}"
        ));
    }

    let source = temp
        .path()
        .join("release")
        .join(format!("ara2-bridge-{VERSION}-source.tar.zst"));
    if !source.is_file() {
        return Err(format!("evidence archive is missing {}", source.display()));
    }
    let filename = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "source bundle has a non-UTF-8 file name".to_owned())?;
    let digest_path = source.with_file_name(format!("{filename}.sha256"));
    let expected_digest = format!("{}  {filename}\n", sha256(&source)?);
    let actual_digest = fs::read_to_string(&digest_path)
        .map_err(|error| format!("cannot read {}: {error}", digest_path.display()))?;
    if actual_digest != expected_digest {
        return Err("embedded source-bundle digest mismatch".to_owned());
    }
    bundle::verify_for_commit(&source, commit)
}

fn extract(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|error| format!("cannot open {}: {error}", archive_path.display()))?;
    let decoder = zstd::stream::read::Decoder::new(file)
        .map_err(|error| format!("cannot decode {}: {error}", archive_path.display()))?;
    let mut archive = tar::Archive::new(decoder);
    let mut seen = BTreeSet::new();
    for entry in archive
        .entries()
        .map_err(|error| format!("cannot read evidence archive: {error}"))?
    {
        let mut entry = entry.map_err(|error| format!("invalid evidence entry: {error}"))?;
        if !entry.header().entry_type().is_file() {
            return Err("evidence archive contains a non-file entry".to_owned());
        }
        let path = entry
            .path()
            .map_err(|error| format!("invalid evidence path: {error}"))?
            .into_owned();
        validate_path(&path)?;
        if !seen.insert(path.clone()) {
            return Err(format!("duplicate evidence path {}", path.display()));
        }
        entry
            .unpack_in(destination)
            .map_err(|error| format!("cannot extract {}: {error}", path.display()))?;
    }
    let allowed_source = PathBuf::from(format!("release/ara2-bridge-{VERSION}-source.tar.zst"));
    let allowed_digest = PathBuf::from(format!(
        "release/ara2-bridge-{VERSION}-source.tar.zst.sha256"
    ));
    for path in seen {
        let evidence_json = path
            .parent()
            .is_some_and(|parent| parent == Path::new("evidence"))
            && path.extension().and_then(|value| value.to_str()) == Some("json");
        if !evidence_json && path != allowed_source && path != allowed_digest {
            return Err(format!(
                "unexpected evidence archive path {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn sha256(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        Err(format!("unsafe evidence path {}", path.display()))
    } else {
        Ok(())
    }
}
