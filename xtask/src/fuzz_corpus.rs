//! Deterministic, licensed fuzz-corpus generation and freshness validation.

use crate::{bindings, Mode};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

type DynError = Box<dyn std::error::Error>;

const REPOSITORY: &str = "https://github.com/entrepeneur4lyf/ara2-bridge";
const GENERATOR_SOURCE: &[u8] = include_bytes!("fuzz_corpus.rs");
const LEGACY_XML: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/legacy-missing-distinct.xml");
const FULL_XML: &[u8] = include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/full-2.3.xml");
const NAMESPACE_XML: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/namespace-qualified.xml");
const ORDERING_XML: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/unrelated-ordering.xml");
const MULTI_XML: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/multi-entry-order.xml");
const SPLIT_ARCHIVE: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/scenarios/ara2-partial-a.archive");
const WAVE: &[u8] = include_bytes!("../../ara2-bridge-testkit/fixtures/audio/wave-unknown-odd.wav");
const RF64: &[u8] = include_bytes!("../../ara2-bridge-testkit/fixtures/audio/rf64-ds64.wav");
const BW64: &[u8] = include_bytes!("../../ara2-bridge-testkit/fixtures/audio/bw64-ds64.wav");
const AIFF: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/audio/aiff-unknown-odd.aiff");
const AIFC: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/audio/aifc-unknown-odd.aifc");

struct Recipe {
    path: &'static str,
    target: &'static str,
    semantic_class: &'static str,
    source_path: &'static str,
    source_license: &'static str,
    source: &'static [u8],
    bytes: Vec<u8>,
}

#[derive(Serialize)]
struct Manifest {
    schema: u32,
    seed: Vec<SeedEntry>,
}

#[derive(Serialize)]
struct SeedEntry {
    path: &'static str,
    target: &'static str,
    semantic_class: &'static str,
    source_path: &'static str,
    source_repository: &'static str,
    source_license: &'static str,
    source_sha256: String,
    output_sha256: String,
}

/// Generates or verifies every named fuzz seed and its licensing manifest.
pub fn generate(root: &Path, mode: Mode) -> Result<(), DynError> {
    let recipes = recipes();
    validate_recipes(&recipes)?;
    reject_extra_outputs(root, &recipes)?;
    for recipe in &recipes {
        bindings::apply(mode, &root.join(recipe.path), &recipe.bytes)?;
    }
    let manifest = Manifest {
        schema: 1,
        seed: recipes
            .iter()
            .map(|recipe| SeedEntry {
                path: recipe.path,
                target: recipe.target,
                semantic_class: recipe.semantic_class,
                source_path: recipe.source_path,
                source_repository: REPOSITORY,
                source_license: recipe.source_license,
                source_sha256: sha256(recipe.source),
                output_sha256: sha256(&recipe.bytes),
            })
            .collect(),
    };
    let rendered = toml::to_string_pretty(&manifest)?;
    bindings::apply(
        mode,
        &root.join("fuzz/corpus-manifest.toml"),
        rendered.as_bytes(),
    )
}

fn recipes() -> Vec<Recipe> {
    let mut recipes = Vec::new();
    for generation in 1_u8..=6 {
        let path = match generation {
            1 => "fuzz/corpus/versioned_structs/generation-1.bin",
            2 => "fuzz/corpus/versioned_structs/generation-2.bin",
            3 => "fuzz/corpus/versioned_structs/generation-3.bin",
            4 => "fuzz/corpus/versioned_structs/generation-4.bin",
            5 => "fuzz/corpus/versioned_structs/generation-5.bin",
            6 => "fuzz/corpus/versioned_structs/generation-6.bin",
            _ => unreachable!(),
        };
        direct(
            &mut recipes,
            path,
            "versioned_structs",
            "released-generation-prefix",
            vec![generation, 8, 0, 0, 0, 0, 0, 0, 0, generation],
        );
    }
    direct(
        &mut recipes,
        "fuzz/corpus/versioned_structs/boundary-prefix.bin",
        "versioned_structs",
        "field-boundary-prefix",
        vec![6, 7, 0, 0, 0, 0, 0, 0, 0],
    );
    direct(
        &mut recipes,
        "fuzz/corpus/references/null.bin",
        "references",
        "null-reference",
        vec![0],
    );
    direct(
        &mut recipes,
        "fuzz/corpus/references/stale.bin",
        "references",
        "stale-reference",
        vec![1, 0xA5],
    );
    direct(
        &mut recipes,
        "fuzz/corpus/references/foreign-session.bin",
        "references",
        "foreign-session-reference",
        vec![2, 0x5A],
    );
    direct(
        &mut recipes,
        "fuzz/corpus/content_events/upstream-all-kinds.bin",
        "content_events",
        "upstream-all-event-kinds",
        content_event_seed(),
    );
    direct(
        &mut recipes,
        "fuzz/corpus/content_events/boundary-invalid.bin",
        "content_events",
        "truncated-and-invalid-event",
        vec![2, 1, 0xFF],
    );
    copied(
        &mut recipes,
        "fuzz/corpus/archive_filters/split-restore.bin",
        "archive_filters",
        "split-restore-golden",
        "ara2-bridge-testkit/fixtures/scenarios/ara2-partial-a.archive",
        SPLIT_ARCHIVE,
    );
    direct(
        &mut recipes,
        "fuzz/corpus/archive_filters/range-overflow.bin",
        "archive_filters",
        "count-and-range-overflow",
        vec![0xFF; 32],
    );
    copied(
        &mut recipes,
        "fuzz/corpus/audio_file_chunks/legacy.bin",
        "audio_file_chunks",
        "legacy-chunk",
        "ara2-bridge-testkit/fixtures/chunks/legacy-missing-distinct.xml",
        LEGACY_XML,
    );
    copied(
        &mut recipes,
        "fuzz/corpus/audio_file_chunks/full-2.3.bin",
        "audio_file_chunks",
        "full-2.3-chunk",
        "ara2-bridge-testkit/fixtures/chunks/full-2.3.xml",
        FULL_XML,
    );
    direct(
        &mut recipes,
        "fuzz/corpus/audio_file_chunks/malformed.bin",
        "audio_file_chunks",
        "malformed-chunk",
        b"<BWFXML><ARA><audioSources>".to_vec(),
    );
    for (path, semantic_class, bytes) in [
        (
            "fuzz/corpus/dispatch/generation-1.bin",
            "generation-1-prefix",
            vec![1, 0, 1],
        ),
        (
            "fuzz/corpus/dispatch/generation-6.bin",
            "generation-6-prefix",
            vec![6, 0, 1],
        ),
        (
            "fuzz/corpus/dispatch/truncated-prefix.bin",
            "truncated-prefix",
            vec![6],
        ),
        (
            "fuzz/corpus/dispatch/null-slot.bin",
            "null-callback-slot",
            vec![6, 3, 0],
        ),
    ] {
        direct(&mut recipes, path, "dispatch", semantic_class, bytes);
    }
    for (path, semantic_class, source_path, bytes) in [
        (
            "fuzz/corpus/audio_file_xml/namespace-qualified.xml",
            "namespace-qualified",
            "ara2-bridge-testkit/fixtures/chunks/namespace-qualified.xml",
            NAMESPACE_XML,
        ),
        (
            "fuzz/corpus/audio_file_xml/unrelated-ordering.xml",
            "unrelated-order-preservation",
            "ara2-bridge-testkit/fixtures/chunks/unrelated-ordering.xml",
            ORDERING_XML,
        ),
        (
            "fuzz/corpus/audio_file_xml/multi-entry-order.xml",
            "multi-entry-order",
            "ara2-bridge-testkit/fixtures/chunks/multi-entry-order.xml",
            MULTI_XML,
        ),
    ] {
        copied(
            &mut recipes,
            path,
            "audio_file_xml",
            semantic_class,
            source_path,
            bytes,
        );
    }
    for (path, semantic_class, source_path, bytes) in [
        (
            "fuzz/corpus/audio_file_container/wave.bin",
            "riff-wave",
            "ara2-bridge-testkit/fixtures/audio/wave-unknown-odd.wav",
            WAVE,
        ),
        (
            "fuzz/corpus/audio_file_container/rf64.bin",
            "rf64-ds64",
            "ara2-bridge-testkit/fixtures/audio/rf64-ds64.wav",
            RF64,
        ),
        (
            "fuzz/corpus/audio_file_container/bw64.bin",
            "bw64-ds64",
            "ara2-bridge-testkit/fixtures/audio/bw64-ds64.wav",
            BW64,
        ),
        (
            "fuzz/corpus/audio_file_container/aiff.bin",
            "aiff",
            "ara2-bridge-testkit/fixtures/audio/aiff-unknown-odd.aiff",
            AIFF,
        ),
        (
            "fuzz/corpus/audio_file_container/aifc.bin",
            "aifc",
            "ara2-bridge-testkit/fixtures/audio/aifc-unknown-odd.aifc",
            AIFC,
        ),
    ] {
        copied(
            &mut recipes,
            path,
            "audio_file_container",
            semantic_class,
            source_path,
            bytes,
        );
    }
    recipes.sort_by_key(|recipe| recipe.path);
    recipes
}

fn direct(
    recipes: &mut Vec<Recipe>,
    path: &'static str,
    target: &'static str,
    semantic_class: &'static str,
    bytes: Vec<u8>,
) {
    recipes.push(Recipe {
        path,
        target,
        semantic_class,
        source_path: "xtask/src/fuzz_corpus.rs",
        source_license: "MIT OR Apache-2.0",
        source: GENERATOR_SOURCE,
        bytes,
    });
}

fn copied(
    recipes: &mut Vec<Recipe>,
    path: &'static str,
    target: &'static str,
    semantic_class: &'static str,
    source_path: &'static str,
    bytes: &'static [u8],
) {
    recipes.push(Recipe {
        path,
        target,
        semantic_class,
        source_path,
        source_license: "Apache-2.0",
        source: bytes,
        bytes: bytes.to_vec(),
    });
}

fn content_event_seed() -> Vec<u8> {
    let sizes = [
        std::mem::size_of::<ara2_bridge_sys::ARAContentTempoEntry>(),
        std::mem::size_of::<ara2_bridge_sys::ARAContentBarSignature>(),
        std::mem::size_of::<ara2_bridge_sys::ARAContentNote>(),
        std::mem::size_of::<ara2_bridge_sys::ARAContentTuning>(),
        std::mem::size_of::<ara2_bridge_sys::ARAContentKeySignature>(),
        std::mem::size_of::<ara2_bridge_sys::ARAContentChord>(),
    ];
    let mut output = Vec::new();
    for (kind, size) in (0_u8..6).zip(sizes) {
        output.push(kind);
        output.extend_from_slice(&(size as u16).to_le_bytes());
        output.resize(output.len() + size, 0);
    }
    output
}

fn validate_recipes(recipes: &[Recipe]) -> Result<(), DynError> {
    let mut paths = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for recipe in recipes {
        if recipe.bytes.is_empty() {
            return Err(std::io::Error::other(format!("empty fuzz seed: {}", recipe.path)).into());
        }
        if recipe.source_license.trim().is_empty() || recipe.source_path.trim().is_empty() {
            return Err(
                std::io::Error::other(format!("unlicensed fuzz seed: {}", recipe.path)).into(),
            );
        }
        if !paths.insert(recipe.path) {
            return Err(
                std::io::Error::other(format!("duplicate fuzz seed: {}", recipe.path)).into(),
            );
        }
        targets.insert(recipe.target);
    }
    let required = BTreeSet::from([
        "archive_filters",
        "audio_file_chunks",
        "audio_file_container",
        "audio_file_xml",
        "content_events",
        "dispatch",
        "references",
        "versioned_structs",
    ]);
    if targets != required {
        return Err(std::io::Error::other("fuzz target corpus coverage is incomplete").into());
    }
    Ok(())
}

fn reject_extra_outputs(root: &Path, recipes: &[Recipe]) -> Result<(), DynError> {
    let expected: BTreeSet<_> = recipes
        .iter()
        .map(|recipe| root.join(recipe.path))
        .collect();
    let corpus = root.join("fuzz/corpus");
    if !corpus.is_dir() {
        return Ok(());
    }
    let mut directories = vec![corpus];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                directories.push(path);
            } else if path.is_file()
                && !expected.contains(&path)
                && !is_libfuzzer_runtime_artifact(&path)
            {
                return Err(std::io::Error::other(format!(
                    "unexpected fuzz seed: {}",
                    path.display()
                ))
                .into());
            }
        }
    }
    Ok(())
}

fn is_libfuzzer_runtime_artifact(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        let bytes = name.as_encoded_bytes();
        bytes.len() == 40
            && bytes
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    })
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
