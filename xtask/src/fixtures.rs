//! Deterministic reviewed fixture recipes.

use crate::{bindings, Mode};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

type DynError = Box<dyn std::error::Error>;

struct StaticRecipe {
    path: &'static str,
    bytes: &'static [u8],
}

struct Recipe {
    path: &'static str,
    bytes: Vec<u8>,
}

const CHUNK_XML: &[StaticRecipe] = &[
    StaticRecipe {
        path: "ara2-bridge-testkit/fixtures/chunks/legacy-missing-distinct.xml",
        bytes: br#"<?xml version="1.0" encoding="UTF-8"?>
<BWFXML><IXML_VERSION>2.0</IXML_VERSION><ARA><audioSources><audioSource><documentArchiveID>com.example.archive</documentArchiveID><persistentID>source-legacy</persistentID><archiveData></archiveData></audioSource></audioSources></ARA></BWFXML>
"#,
    },
    StaticRecipe {
        path: "ara2-bridge-testkit/fixtures/chunks/full-2.3.xml",
        bytes: br#"<?xml version="1.0" encoding="UTF-8"?>
<BWFXML><ARA version="2.3"><audioSources><audioSource><documentArchiveID>full.archive</documentArchiveID><openAutomatically>true</openAutomatically><createDistinctAudioModification>true</createDistinctAudioModification><suggestedPlugIn><plugInName>Example</plugInName><lowestSupportedVersion>2.3</lowestSupportedVersion><manufacturerName>Example Corp</manufacturerName><informationURL>https://example.test/ara</informationURL></suggestedPlugIn><persistentID>source-full</persistentID><archiveData>AQIDBA==</archiveData></audioSource></audioSources></ARA></BWFXML>
"#,
    },
    StaticRecipe {
        path: "ara2-bridge-testkit/fixtures/chunks/namespace-qualified.xml",
        bytes: br#"<?xml version="1.0" encoding="UTF-8"?>
<ix:BWFXML xmlns:ix="urn:ebu:metadata-schema:ebuCore_2014"><ix:PROJECT>keep</ix:PROJECT><ix:ARA custom="retained"><ix:audioSources><ix:audioSource><ix:documentArchiveID>first</ix:documentArchiveID><ix:openAutomatically>false</ix:openAutomatically><ix:persistentID>source-first</ix:persistentID><ix:archiveData>AQ==</ix:archiveData></ix:audioSource><ix:audioSource><ix:documentArchiveID>second</ix:documentArchiveID><ix:persistentID>source-second</ix:persistentID><ix:archiveData>Ag==</ix:archiveData></ix:audioSource></ix:audioSources></ix:ARA><ix:NOTE priority="1">after</ix:NOTE></ix:BWFXML>
"#,
    },
    StaticRecipe {
        path: "ara2-bridge-testkit/fixtures/chunks/unrelated-ordering.xml",
        bytes: br#"<?xml version="1.0" encoding="UTF-8"?>
<BWFXML before="yes"><PROJECT>before</PROJECT><ARA><vendorBefore code="1"/><audioSources vendorSources="keep"><vendorDictionaryBefore rank="0"/><audioSource customSource="yes"><vendorSourceBefore rank="1"/><documentArchiveID vendorId="keep">ordered</documentArchiveID><suggestedPlugIn vendorSuggested="keep"><plugInName>Ordered</plugInName><vendorSuggestion rank="2"/></suggestedPlugIn><persistentID>source-ordered</persistentID><archiveData vendorData="keep">AA==</archiveData><vendorSourceAfter rank="3"/></audioSource><vendorDictionaryAfter rank="4"/></audioSources><vendorAfter code="2"/></ARA><NOTE>after</NOTE></BWFXML>
"#,
    },
    StaticRecipe {
        path: "ara2-bridge-testkit/fixtures/chunks/multi-entry-order.xml",
        bytes: br#"<?xml version="1.0" encoding="UTF-8"?>
<BWFXML><ARA><audioSources><audioSource><documentArchiveID>zeta</documentArchiveID><persistentID>z</persistentID><archiveData>eg==</archiveData></audioSource><audioSource><documentArchiveID>alpha</documentArchiveID><persistentID>a</persistentID><archiveData>YQ==</archiveData></audioSource><audioSource><documentArchiveID>middle</documentArchiveID><persistentID>m</persistentID><archiveData>bQ==</archiveData></audioSource></audioSources></ARA></BWFXML>
"#,
    },
];

/// Generates or checks one named fixture set below `root`.
pub fn generate(root: &Path, mode: Mode, set: &str) -> Result<(), DynError> {
    let (directory, recipes) = match set {
        "chunk-xml" => (
            "ara2-bridge-testkit/fixtures/chunks",
            CHUNK_XML
                .iter()
                .map(|recipe| Recipe {
                    path: recipe.path,
                    bytes: recipe.bytes.to_vec(),
                })
                .collect(),
        ),
        "audio-containers" => (
            "ara2-bridge-testkit/fixtures/audio",
            audio_container_recipes(),
        ),
        "upstream-scenarios" => (
            "ara2-bridge-testkit/fixtures/scenarios",
            upstream_scenario_recipes(),
        ),
        _ => return Err(std::io::Error::other(format!("unknown fixture set: {set}")).into()),
    };
    reject_extra_outputs(root, directory, &recipes)?;
    for recipe in &recipes {
        bindings::apply(mode, &root.join(recipe.path), &recipe.bytes)?;
    }
    update_provenance(root, mode, set, &recipes)
}

fn reject_extra_outputs(root: &Path, directory: &str, recipes: &[Recipe]) -> Result<(), DynError> {
    let expected: BTreeSet<_> = recipes
        .iter()
        .map(|recipe| root.join(recipe.path))
        .collect();
    let directory = root.join(directory);
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_file() && !expected.contains(&path) {
            return Err(std::io::Error::other(format!(
                "unexpected generated fixture: {}",
                path.display()
            ))
            .into());
        }
    }
    Ok(())
}

fn update_provenance(
    root: &Path,
    mode: Mode,
    set: &str,
    recipes: &[Recipe],
) -> Result<(), DynError> {
    let path = root.join("sdk-provenance.toml");
    if !path.is_file() {
        return Ok(());
    }
    let mut document: toml::Value = toml::from_str(&std::fs::read_to_string(&path)?)?;
    let files = document
        .get_mut("file")
        .and_then(toml::Value::as_array_mut)
        .ok_or_else(|| std::io::Error::other("sdk provenance has no file array"))?;
    let paths: BTreeSet<_> = recipes.iter().map(|recipe| recipe.path).collect();
    files.retain(|entry| {
        entry
            .get("path")
            .and_then(toml::Value::as_str)
            .is_none_or(|path| !paths.contains(path))
    });
    for recipe in recipes {
        let hash = sha256(&recipe.bytes);
        let mut entry = toml::map::Map::new();
        entry.insert("path".into(), recipe.path.into());
        entry.insert(
            "role".into(),
            format!("generated-fixture;set={set};source=reviewed-recipe;license=Apache-2.0;input-sha256={hash}").into(),
        );
        entry.insert("sha256".into(), hash.into());
        files.push(toml::Value::Table(entry));
    }
    files.sort_by(|left, right| {
        let left_path = left
            .get("path")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        let right_path = right
            .get("path")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        left_path.cmp(right_path)
    });
    let expected = toml::to_string_pretty(&document)?;
    match mode {
        Mode::Write => {
            let temporary = temporary_path(&path);
            std::fs::write(&temporary, &expected)?;
            std::fs::rename(temporary, path)?;
            Ok(())
        }
        Mode::Check => {
            if std::fs::read_to_string(&path)? == expected {
                Ok(())
            } else {
                Err(std::io::Error::other("stale chunk-xml fixture provenance").into())
            }
        }
    }
}

fn audio_container_recipes() -> Vec<Recipe> {
    let xml = CHUNK_XML[0].bytes;
    vec![
        Recipe {
            path: "ara2-bridge-testkit/fixtures/audio/wave-unknown-odd.wav",
            bytes: riff_fixture(*b"RIFF", &[(b"JUNK", &[1, 2, 3], 0xA5), (b"iXML", xml, 0)]),
        },
        Recipe {
            path: "ara2-bridge-testkit/fixtures/audio/rf64-ds64.wav",
            bytes: large_riff_fixture(*b"RF64", xml),
        },
        Recipe {
            path: "ara2-bridge-testkit/fixtures/audio/bw64-ds64.wav",
            bytes: large_riff_fixture(*b"BW64", xml),
        },
        Recipe {
            path: "ara2-bridge-testkit/fixtures/audio/aiff-unknown-odd.aiff",
            bytes: form_fixture(*b"AIFF", &[(b"ANNO", &[4, 5, 6], 0x5A), (b"iXML", xml, 0)]),
        },
        Recipe {
            path: "ara2-bridge-testkit/fixtures/audio/aifc-unknown-odd.aifc",
            bytes: form_fixture(
                *b"AIFC",
                &[(b"APPL", &[7, 8, 9, 10, 11], 0xC3), (b"iXML", xml, 0)],
            ),
        },
    ]
}

fn upstream_scenario_recipes() -> Vec<Recipe> {
    let archive = |path: &'static str, generation: &str, scope: &str| {
        Recipe {
        path,
        bytes: format!(
            "ARA2-BRIDGE-ARCHIVE\nrecipe-version=1\ngeneration={generation}\nscope={scope}\ndocument-archive-id=org.ara2-bridge.test.archive\ncontent=capability-rich-fixture\n"
        )
        .into_bytes(),
    }
    };
    let xml = CHUNK_XML[1].bytes;
    vec![
        archive(
            "ara2-bridge-testkit/fixtures/scenarios/ara1-full.archive",
            "1-final",
            "full-document",
        ),
        archive(
            "ara2-bridge-testkit/fixtures/scenarios/ara2-full.archive",
            "2.3-final",
            "full-document",
        ),
        archive(
            "ara2-bridge-testkit/fixtures/scenarios/ara2-partial-a.archive",
            "2.3-final",
            "audio-sources,audio-modifications",
        ),
        archive(
            "ara2-bridge-testkit/fixtures/scenarios/ara2-partial-b.archive",
            "2.3-final",
            "document-data",
        ),
        Recipe {
            path: "ara2-bridge-testkit/fixtures/scenarios/chunk-wave.wav",
            bytes: riff_fixture(*b"RIFF", &[(b"iXML", xml, 0)]),
        },
        Recipe {
            path: "ara2-bridge-testkit/fixtures/scenarios/chunk-aiff.aiff",
            bytes: form_fixture(*b"AIFF", &[(b"iXML", xml, 0)]),
        },
    ]
}

fn riff_fixture(signature: [u8; 4], chunks: &[(&[u8; 4], &[u8], u8)]) -> Vec<u8> {
    let mut output = Vec::from(signature);
    output.extend_from_slice(&[0; 4]);
    output.extend_from_slice(b"WAVE");
    for (id, data, pad) in chunks {
        push_chunk(&mut output, id, data, *pad, u32::to_le_bytes);
    }
    let size = u32::try_from(output.len() - 8).expect("reviewed RIFF fixture fits u32");
    output[4..8].copy_from_slice(&size.to_le_bytes());
    output
}

fn large_riff_fixture(signature: [u8; 4], xml: &[u8]) -> Vec<u8> {
    let mut ds64 = vec![0_u8; 40];
    ds64[24..28].copy_from_slice(&1_u32.to_le_bytes());
    ds64[28..32].copy_from_slice(b"iXML");
    ds64[32..40].copy_from_slice(
        &u64::try_from(xml.len())
            .expect("fixture XML length fits u64")
            .to_le_bytes(),
    );
    let mut output = riff_fixture(
        signature,
        &[
            (b"ds64", &ds64, 0),
            (b"JUNK", &[12, 13, 14], 0xD4),
            (b"iXML", xml, 0),
        ],
    );
    output[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    let ixml = output
        .windows(4)
        .rposition(|window| window == b"iXML")
        .expect("fixture contains iXML chunk");
    output[ixml + 4..ixml + 8].copy_from_slice(&u32::MAX.to_le_bytes());
    let riff_size = u64::try_from(output.len() - 8).expect("fixture length fits u64");
    ds64[..8].copy_from_slice(&riff_size.to_le_bytes());
    output[20..60].copy_from_slice(&ds64);
    output
}

fn form_fixture(form: [u8; 4], chunks: &[(&[u8; 4], &[u8], u8)]) -> Vec<u8> {
    let mut output = Vec::from(*b"FORM");
    output.extend_from_slice(&[0; 4]);
    output.extend_from_slice(&form);
    for (id, data, pad) in chunks {
        push_chunk(&mut output, id, data, *pad, u32::to_be_bytes);
    }
    let size = u32::try_from(output.len() - 8).expect("reviewed FORM fixture fits u32");
    output[4..8].copy_from_slice(&size.to_be_bytes());
    output
}

fn push_chunk(
    output: &mut Vec<u8>,
    id: &[u8; 4],
    data: &[u8],
    pad: u8,
    encode_size: fn(u32) -> [u8; 4],
) {
    output.extend_from_slice(id);
    let size = u32::try_from(data.len()).expect("reviewed fixture chunk fits u32");
    output.extend_from_slice(&encode_size(size));
    output.extend_from_slice(data);
    if data.len() & 1 != 0 {
        output.push(pad);
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".tmp-{}", std::process::id()));
    PathBuf::from(name)
}
