//! Joined interface-coverage report generation and verification.

use ara2_bridge_testkit::coverage::{
    all_contract_tests, all_delegates, ContractTest, CoverageReport,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{Path, PathBuf};

type DynError = Box<dyn Error + Send + Sync + 'static>;

#[derive(Deserialize)]
struct CoreManifest {
    metadata: Value,
    records: Vec<CoreSymbol>,
}

#[derive(Deserialize)]
struct CoreSymbol {
    symbol: String,
    kind: String,
    header: String,
    classification: String,
    required_sdks: Vec<String>,
}

#[derive(Deserialize)]
struct CompanionManifest {
    records: Vec<CompanionSymbol>,
}

#[derive(Clone, Deserialize)]
struct CompanionSymbol {
    symbol: String,
    rust: String,
    classification: String,
}

#[derive(Serialize)]
struct JoinedSymbol {
    symbol: String,
    kind: String,
    header: String,
    source_classification: String,
    resolution: String,
    resolution_classification: String,
    required_sdks: Vec<String>,
}

#[derive(Serialize)]
struct Summary {
    callback_slots: usize,
    source_symbols: usize,
    core_abi_symbols: usize,
    companion_records: usize,
    companion_unique_symbols: usize,
    unresolved_symbols: usize,
}

/// Writes or verifies the deterministic interface coverage reports.
pub fn generate(root: &Path, mode: crate::Mode) -> Result<(), DynError> {
    let contracts = all_contract_tests();
    validate_contract_sources(root, &contracts)?;
    let callbacks = CoverageReport::build(
        ara2_bridge_sys::compatibility::RECORDS,
        &all_delegates(),
        &contracts,
    );
    if !callbacks.semantic_gaps().is_empty() {
        return Err(format!("semantic coverage gaps: {:#?}", callbacks.semantic_gaps()).into());
    }

    let core: CoreManifest = read_json(
        &root.join("ara2-bridge-sys/generated/symbol-coverage.json"),
        "core symbol coverage",
    )?;
    let companion_paths = [
        (
            "clap",
            root.join("ara2-bridge-companion/probes/clap-symbols.json"),
        ),
        (
            "vst3",
            root.join("ara2-bridge-companion/probes/vst3-symbols.json"),
        ),
        (
            "audio-unit",
            root.join("ara2-bridge-companion/probes/audio-unit-symbols.json"),
        ),
    ];
    let mut companion = BTreeMap::<String, BTreeMap<String, CompanionSymbol>>::new();
    for (component, path) in companion_paths {
        let manifest: CompanionManifest = read_json(&path, component)?;
        let mut records = BTreeMap::new();
        for record in manifest.records {
            if records.insert(record.symbol.clone(), record).is_some() {
                return Err(format!(
                    "duplicate {component} companion symbol in {}",
                    path.display()
                )
                .into());
            }
        }
        companion.insert(component.to_owned(), records);
    }

    let mut joined = Vec::new();
    let mut unresolved = Vec::new();
    let mut deferred_unique = BTreeSet::new();
    let mut core_count = 0;
    let companion_record_count = core
        .records
        .iter()
        .filter(|record| record.classification == "companion-deferred")
        .count();
    for record in core.records {
        let (resolution, resolution_classification) = match record.classification.as_str() {
            "core-abi" => {
                core_count += 1;
                (
                    "ara2-bridge-sys pregenerated ABI and core probes".to_owned(),
                    "core-abi".to_owned(),
                )
            }
            "companion-deferred" => {
                deferred_unique.insert(record.symbol.clone());
                let mut matches = record.required_sdks.iter().filter_map(|sdk| {
                    companion
                        .get(sdk)
                        .and_then(|symbols| symbols.get(&record.symbol))
                });
                match (matches.next(), matches.next()) {
                    (Some(found), None) => (found.rust.clone(), found.classification.clone()),
                    (None, _) => {
                        unresolved.push(format!(
                            "{} requires {:?}",
                            record.symbol, record.required_sdks
                        ));
                        ("unresolved".to_owned(), "unresolved".to_owned())
                    }
                    (Some(_), Some(_)) => {
                        unresolved
                            .push(format!("{} has ambiguous companion closure", record.symbol));
                        ("ambiguous".to_owned(), "ambiguous".to_owned())
                    }
                }
            }
            other => {
                unresolved.push(format!(
                    "{} has unknown classification {other}",
                    record.symbol
                ));
                ("unresolved".to_owned(), "unresolved".to_owned())
            }
        };
        joined.push(JoinedSymbol {
            symbol: record.symbol,
            kind: record.kind,
            header: record.header,
            source_classification: record.classification,
            resolution,
            resolution_classification,
            required_sdks: record.required_sdks,
        });
    }
    if !unresolved.is_empty() {
        return Err(format!("unresolved symbol coverage: {unresolved:#?}").into());
    }

    let summary = Summary {
        callback_slots: callbacks.entries().len(),
        source_symbols: joined.len(),
        core_abi_symbols: core_count,
        companion_records: companion_record_count,
        companion_unique_symbols: deferred_unique.len(),
        unresolved_symbols: 0,
    };
    let markdown = render_markdown(&callbacks, &summary);
    let json = format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({
            "metadata": core.metadata,
            "summary": summary,
            "callbacks": callbacks,
            "symbols": joined,
        }))?
    );
    write_or_check(
        root.join("docs/conformance/interface-coverage.md"),
        &markdown,
        mode,
    )?;
    write_or_check(
        root.join("docs/conformance/interface-coverage.json"),
        &json,
        mode,
    )
}

fn validate_contract_sources(root: &Path, contracts: &[ContractTest]) -> Result<(), DynError> {
    let mut checked = BTreeSet::new();
    for contract in contracts {
        for identifier in contract.test_ids {
            if !checked.insert(*identifier) {
                continue;
            }
            let (relative, test) = identifier.split_once('#').ok_or_else(|| {
                format!("contract evidence must use path#test_function: {identifier}")
            })?;
            if relative.starts_with('/')
                || relative
                    .split('/')
                    .any(|part| part.is_empty() || part == "..")
            {
                return Err(format!("invalid contract evidence path: {identifier}").into());
            }
            let path = root.join(relative);
            let source = std::fs::read_to_string(&path).map_err(|error| {
                format!("cannot read contract evidence {}: {error}", path.display())
            })?;
            if !source.contains(&format!("fn {test}(")) {
                return Err(format!(
                    "contract evidence test `{test}` is missing from {}",
                    path.display()
                )
                .into());
            }
        }
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, label: &str) -> Result<T, DynError> {
    serde_json::from_slice(&std::fs::read(path)?)
        .map_err(|error| format!("invalid {label} manifest {}: {error}", path.display()).into())
}

fn render_markdown(callbacks: &CoverageReport, summary: &Summary) -> String {
    let mut output = callbacks.render_markdown();
    output.push_str("\n## Symbol closure\n\n");
    output.push_str(&format!(
        "- Source declaration records: {}\n- Core ABI records: {}\n- Companion-deferred records: {} ({} unique symbols)\n- Unresolved records: {}\n",
        summary.source_symbols,
        summary.core_abi_symbols,
        summary.companion_records,
        summary.companion_unique_symbols,
        summary.unresolved_symbols
    ));
    output
}

fn write_or_check(path: PathBuf, content: &str, mode: crate::Mode) -> Result<(), DynError> {
    match mode {
        crate::Mode::Write => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content)?;
            Ok(())
        }
        crate::Mode::Check => {
            let existing = std::fs::read_to_string(&path)?;
            if existing == content {
                Ok(())
            } else {
                Err(format!("coverage report is stale: {}", path.display()).into())
            }
        }
    }
}
