//! Generation of sized-struct compatibility and access metadata.

use crate::{bindings, provenance, Mode};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type DynError = Box<dyn std::error::Error>;

#[derive(Clone, Debug)]
struct Field {
    name: String,
    rust_type: String,
}

#[derive(Clone, Debug)]
struct Record {
    kind: &'static str,
    surface: String,
    name: Option<String>,
    generation_min: Option<i64>,
    generations: Vec<i64>,
    terminal: Option<String>,
    terminal_kind: &'static str,
    callbacks: Vec<String>,
    dependency_group: Option<String>,
    fallbacks: Vec<(String, String)>,
    attributes: Vec<(String, String)>,
}

fn message(text: impl Into<String>) -> DynError {
    std::io::Error::other(text.into()).into()
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child")
}

/// Generates or verifies compatibility metadata.
pub fn generate(mode: Mode) -> Result<(), DynError> {
    provenance::verify(root(), root().join("sdk-provenance.toml"))?;
    let raw = std::fs::read_to_string(root().join("ara2-bridge-sys/src/generated/x86_64.rs"))?;
    let structs = parse_raw_structs(&raw)?;
    let manifest_path = root().join("docs/specs/ara2-bridge/api-compatibility.toml");
    let manifest_text = std::fs::read_to_string(&manifest_path)?;
    let manifest: toml::Value = toml::from_str(&manifest_text)?;
    let records = parse_records(&manifest)?;
    validate_records(&records, &structs)?;

    let generated = root().join("ara2-bridge-sys/src/generated");
    let layout = bindings::format_rust(&render_layout(&structs))?;
    bindings::validate_generated_metadata(&layout)?;
    bindings::apply(mode, &generated.join("layout.rs"), layout.as_bytes())?;

    let access = bindings::format_rust(&render_access())?;
    bindings::validate_generated_metadata(&access)?;
    bindings::apply(mode, &generated.join("access.rs"), access.as_bytes())?;

    let compatibility = bindings::format_rust(&render_compatibility(&records)?)?;
    bindings::validate_generated_metadata(&compatibility)?;
    bindings::apply(
        mode,
        &generated.join("compatibility.rs"),
        compatibility.as_bytes(),
    )?;
    Ok(())
}

fn parse_raw_structs(raw: &str) -> Result<BTreeMap<String, Vec<Field>>, DynError> {
    let mut structs = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut fields = Vec::new();
    let mut pending: Option<(String, String)> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if current.is_none() {
            if let Some(rest) = trimmed.strip_prefix("pub struct ") {
                let name = rest
                    .split_whitespace()
                    .next()
                    .ok_or_else(|| message("malformed generated struct declaration"))?;
                current = Some(name.to_owned());
            }
            continue;
        }

        let begins_field = line.starts_with("    pub ");
        if (begins_field || trimmed == "}") && pending.is_some() {
            let (name, rust_type) = pending.take().unwrap();
            fields.push(Field {
                name,
                rust_type: rust_type.trim().trim_end_matches(',').to_owned(),
            });
        }
        if trimmed == "}" {
            let name = current.take().unwrap();
            if structs
                .insert(name.clone(), std::mem::take(&mut fields))
                .is_some()
            {
                return Err(message(format!("duplicate generated struct: {name}")));
            }
            continue;
        }
        if begins_field {
            let declaration = trimmed
                .strip_prefix("pub ")
                .expect("field prefix already checked");
            let (name, rust_type) = declaration
                .split_once(':')
                .ok_or_else(|| message(format!("malformed generated field: {trimmed}")))?;
            pending = Some((name.trim().to_owned(), rust_type.trim().to_owned()));
        } else if let Some((_, rust_type)) = &mut pending {
            rust_type.push(' ');
            rust_type.push_str(trimmed);
        }
    }
    if current.is_some() {
        return Err(message("unterminated generated struct declaration"));
    }
    Ok(structs)
}

fn parse_records(manifest: &toml::Value) -> Result<Vec<Record>, DynError> {
    let mut records = Vec::new();
    for (table_name, kind) in [
        ("prefix", "Prefix"),
        ("capability_group", "CapabilityGroup"),
        ("data_surface", "DataSurface"),
    ] {
        let tables = manifest
            .get(table_name)
            .and_then(toml::Value::as_array)
            .ok_or_else(|| {
                message(format!(
                    "compatibility manifest is missing [[{table_name}]]"
                ))
            })?;
        for value in tables {
            let table = value
                .as_table()
                .ok_or_else(|| message(format!("[[{table_name}]] must be a table")))?;
            let surface = table
                .get("surface")
                .or_else(|| table.get("name"))
                .and_then(toml::Value::as_str)
                .ok_or_else(|| message(format!("[[{table_name}]] has no surface/name")))?
                .to_owned();
            let name = (table_name == "capability_group")
                .then(|| {
                    table
                        .get("name")
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned)
                })
                .flatten();
            let generation_min = table
                .get("generation_min")
                .and_then(toml::Value::as_integer);
            let generations = string_or_integer_array(table.get("generations"))?;
            if generation_min.is_none() && generations.is_empty() {
                return Err(message(format!(
                    "compatibility record {surface} has no generation selector"
                )));
            }

            let (terminal, terminal_kind) = if let Some(value) = table.get("required_terminal") {
                (value.as_str().map(str::to_owned), "Required")
            } else if let Some(value) = table.get("optional_terminal") {
                (value.as_str().map(str::to_owned), "Optional")
            } else if let Some(value) = table.get("terminal") {
                (value.as_str().map(str::to_owned), "Capability")
            } else {
                (None, "None")
            };
            let callbacks = string_array(table.get("callbacks"))?;
            let mut fallbacks = Vec::new();
            let mut attributes = Vec::new();
            for (key, value) in table {
                if matches!(
                    key.as_str(),
                    "surface"
                        | "name"
                        | "generation_min"
                        | "generations"
                        | "required_terminal"
                        | "optional_terminal"
                        | "terminal"
                        | "callbacks"
                ) {
                    continue;
                }
                let value = compact_value(value)?;
                if key.contains("fallback") || key.ends_with("default") {
                    fallbacks.push((key.clone(), value));
                } else {
                    attributes.push((key.clone(), value));
                }
            }
            records.push(Record {
                kind,
                surface,
                name: name.clone(),
                generation_min,
                generations,
                terminal,
                terminal_kind,
                callbacks,
                dependency_group: name,
                fallbacks,
                attributes,
            });
        }
    }
    Ok(records)
}

fn string_array(value: Option<&toml::Value>) -> Result<Vec<String>, DynError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .ok_or_else(|| message("expected a string array"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| message("expected a string array member"))
        })
        .collect()
}

fn string_or_integer_array(value: Option<&toml::Value>) -> Result<Vec<i64>, DynError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .ok_or_else(|| message("expected an integer array"))?
        .iter()
        .map(|item| {
            item.as_integer()
                .ok_or_else(|| message("expected an integer array member"))
        })
        .collect()
}

fn compact_value(value: &toml::Value) -> Result<String, DynError> {
    match value {
        toml::Value::String(value) => Ok(value.clone()),
        toml::Value::Integer(value) => Ok(value.to_string()),
        toml::Value::Boolean(value) => Ok(value.to_string()),
        toml::Value::Array(values) => values
            .iter()
            .map(compact_value)
            .collect::<Result<Vec<_>, _>>()
            .map(|values| values.join(",")),
        _ => Err(message("nested compatibility values are unsupported")),
    }
}

fn validate_records(
    records: &[Record],
    structs: &BTreeMap<String, Vec<Field>>,
) -> Result<(), DynError> {
    let mut callbacks_by_surface: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for record in records {
        let fields = structs
            .get(&record.surface)
            .ok_or_else(|| message(format!("unknown compatibility surface: {}", record.surface)))?;
        let field_names: BTreeSet<_> = fields.iter().map(|field| field.name.as_str()).collect();
        if let Some(terminal) = &record.terminal {
            if !field_names.contains(terminal.as_str()) {
                return Err(message(format!(
                    "terminal {}.{} does not exist in generated bindings",
                    record.surface, terminal
                )));
            }
        }
        for callback in &record.callbacks {
            if !field_names.contains(callback.as_str()) {
                return Err(message(format!(
                    "callback {}.{} does not exist in generated bindings",
                    record.surface, callback
                )));
            }
            callbacks_by_surface
                .entry(&record.surface)
                .or_default()
                .push(callback);
        }
        for generation in record
            .generation_min
            .iter()
            .chain(record.generations.iter())
        {
            if !(1..=6).contains(generation) {
                return Err(message(format!(
                    "invalid generation {generation} for {}",
                    record.surface
                )));
            }
        }
    }

    for (surface, callbacks) in callbacks_by_surface {
        let fields = &structs[surface];
        let positions: BTreeMap<_, _> = fields
            .iter()
            .enumerate()
            .map(|(index, field)| (field.name.as_str(), index))
            .collect();
        let mut previous = None;
        let mut seen = BTreeSet::new();
        for callback in callbacks {
            if !seen.insert(callback) {
                return Err(message(format!(
                    "duplicate callback in compatibility manifest: {surface}.{callback}"
                )));
            }
            let position = positions[callback];
            if previous.is_some_and(|previous| position <= previous) {
                return Err(message(format!(
                    "callback order disagrees with header for {surface}.{callback}"
                )));
            }
            previous = Some(position);
        }
    }

    let document = &structs["ARADocumentControllerInterface"];
    let document_fields: Vec<_> = document
        .iter()
        .filter(|field| field.name != "structSize")
        .map(|field| field.name.as_str())
        .collect();
    let document_callbacks: Vec<_> = records
        .iter()
        .filter(|record| record.surface == "ARADocumentControllerInterface")
        .flat_map(|record| record.callbacks.iter().map(String::as_str))
        .collect();
    if document_callbacks.len() != 54 || document_callbacks != document_fields {
        return Err(message(format!(
            "document-controller semantic join failed: manifest={} header={} (expected 54 in exact order)",
            document_callbacks.len(),
            document_fields.len()
        )));
    }
    Ok(())
}

fn render_layout(structs: &BTreeMap<String, Vec<Field>>) -> String {
    let mut output = bindings::rust_banner();
    output.push_str(
        "//! Generated field extents for versioned and packed ARA records.\n\n\
         use super::*;\n\n\
         /// Returns the represented byte extent ending after field type `F` in record `T`.\n\
         pub const fn implemented_size<T, F>(field_offset: usize) -> usize {\n\
             let _ = ::std::mem::size_of::<T>();\n\
             field_offset + ::std::mem::size_of::<F>()\n\
         }\n\n",
    );
    for (surface, fields) in structs {
        let mut extent_constants = Vec::with_capacity(fields.len());
        for field in fields {
            let constant = screaming(&format!("{surface}_{}", field.name));
            extent_constants.push(constant.clone());
            output.push_str(&format!(
                "/// Byte extent through `{surface}::{}`.\n\
                 pub const {constant}: usize = implemented_size::<{surface}, {}>(\n\
                     ::std::mem::offset_of!({surface}, {})\n\
                 );\n\
                 const _: () = assert!({constant} <= ::std::mem::size_of::<{surface}>());\n\n",
                field.name, field.rust_type, field.name
            ));
        }
        let table = screaming(&format!("{surface}_field_extents"));
        output.push_str(&format!(
            "/// Complete field extents for `{surface}` in declaration order.\n\
             pub const {table}: &[usize] = &[{}];\n\n",
            extent_constants.join(", ")
        ));
    }
    output
}

fn render_access() -> String {
    format!(
        "{}//! Unaligned access primitives for packed ARA records and property buffers.\n\n\
         /// Copies a field value from `base + offset` without creating an unaligned reference.\n\
         ///\n\
         /// # Safety\n\
         ///\n\
         /// The caller must guarantee that `base..base + offset + size_of::<T>()` is readable,\n\
         /// initialized for `T`, and remains valid for the duration of this copy.\n\
         pub unsafe fn read_field<T: Copy>(base: *const u8, offset: usize) -> T {{\n\
             // SAFETY: guaranteed by the caller; `read_unaligned` creates no borrowed field.\n\
             unsafe {{ ::std::ptr::read_unaligned(base.add(offset).cast::<T>()) }}\n\
         }}\n\n\
         /// Copies `value` to `base + offset` without creating an unaligned reference.\n\
         ///\n\
         /// # Safety\n\
         ///\n\
         /// The caller must guarantee that `base..base + offset + size_of::<T>()` is writable,\n\
         /// properly owned for the write, and that overwriting it does not violate a live value.\n\
         pub unsafe fn write_field<T>(base: *mut u8, offset: usize, value: T) {{\n\
             // SAFETY: guaranteed by the caller; `write_unaligned` creates no borrowed field.\n\
             unsafe {{ ::std::ptr::write_unaligned(base.add(offset).cast::<T>(), value) }}\n\
         }}\n",
        bindings::rust_banner()
    )
}

fn render_compatibility(records: &[Record]) -> Result<String, DynError> {
    let mut fallback_variants = BTreeMap::new();
    for record in records {
        for (_, action) in &record.fallbacks {
            fallback_variants
                .entry(action.clone())
                .or_insert_with(|| camel(action));
        }
    }

    let mut output = bindings::rust_banner();
    output.push_str(
        "//! Generated compatibility rules joined to the released ARA header order.\n\n\
         /// The source category of a compatibility record.\n\
         #[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
         pub enum RecordKind { Prefix, CapabilityGroup, DataSurface }\n\n\
         /// Whether a terminal field is required, optional, or a capability cut point.\n\
         #[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
         pub enum TerminalKind { None, Required, Optional, Capability }\n\n\
         /// A reviewed semantic action used when an optional tail is absent or disabled.\n\
         #[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
         pub enum Fallback {\n",
    );
    for variant in fallback_variants.values() {
        output.push_str(&format!("    {variant},\n"));
    }
    output.push_str(
        "}\n\n\
         /// One context-specific fallback rule.\n\
         #[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
         pub struct FallbackRule { pub context: &'static str, pub action: Fallback }\n\n\
         /// Additional reviewed compatibility data not encoded by the structural fields.\n\
         #[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
         pub struct Attribute { pub key: &'static str, pub value: &'static str }\n\n\
         /// One generated prefix, capability, or data-surface compatibility record.\n\
         #[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
         pub struct CompatibilityRecord {\n\
             pub kind: RecordKind,\n\
             pub surface: &'static str,\n\
             pub name: Option<&'static str>,\n\
             pub generation_min: Option<u32>,\n\
             pub generations: &'static [u32],\n\
             pub terminal: Option<&'static str>,\n\
             pub terminal_kind: TerminalKind,\n\
             pub callbacks: &'static [&'static str],\n\
             pub dependency_group: Option<&'static str>,\n\
             pub fallbacks: &'static [FallbackRule],\n\
             pub attributes: &'static [Attribute],\n\
         }\n\n",
    );

    let document_callbacks: Vec<_> = records
        .iter()
        .filter(|record| record.surface == "ARADocumentControllerInterface")
        .flat_map(|record| record.callbacks.iter())
        .collect();
    output.push_str("/// All 54 document-controller callback slots in released header order.\n");
    output.push_str("pub const DOCUMENT_CONTROLLER_CALLBACKS: &[&str] = &[");
    for callback in document_callbacks {
        output.push_str(&format!("{:?},", callback));
    }
    output.push_str("];\n\n/// All generated compatibility records.\npub const RECORDS: &[CompatibilityRecord] = &[\n");

    for record in records {
        let generations = record
            .generations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let callbacks = record
            .callbacks
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let fallback_rules = record
            .fallbacks
            .iter()
            .map(|(context, action)| {
                let variant = &fallback_variants[action];
                format!("FallbackRule {{ context: {context:?}, action: Fallback::{variant} }}")
            })
            .collect::<Vec<_>>()
            .join(",");
        let attributes = record
            .attributes
            .iter()
            .map(|(key, value)| format!("Attribute {{ key: {key:?}, value: {value:?} }}"))
            .collect::<Vec<_>>()
            .join(",");
        output.push_str(&format!(
            "    CompatibilityRecord {{\n\
             kind: RecordKind::{}, surface: {:?}, name: {},\n\
             generation_min: {}, generations: &[{}], terminal: {}, terminal_kind: TerminalKind::{},\n\
             callbacks: &[{}], dependency_group: {}, fallbacks: &[{}], attributes: &[{}],\n\
             }},\n",
            record.kind,
            record.surface,
            option_string(record.name.as_deref()),
            record
                .generation_min
                .map(|value| format!("Some({value})"))
                .unwrap_or_else(|| "None".to_owned()),
            generations,
            option_string(record.terminal.as_deref()),
            record.terminal_kind,
            callbacks,
            option_string(record.dependency_group.as_deref()),
            fallback_rules,
            attributes,
        ));
    }
    output.push_str("];\n");
    Ok(output)
}

fn option_string(value: Option<&str>) -> String {
    value
        .map(|value| format!("Some({value:?})"))
        .unwrap_or_else(|| "None".to_owned())
}

fn screaming(value: &str) -> String {
    let mut output = String::new();
    let mut previous_lowercase = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if character.is_ascii_uppercase() && previous_lowercase && !output.ends_with('_') {
                output.push('_');
            }
            output.push(character.to_ascii_uppercase());
            previous_lowercase = character.is_ascii_lowercase() || character.is_ascii_digit();
        } else if !output.ends_with('_') {
            output.push('_');
            previous_lowercase = false;
        }
    }
    output
}

fn camel(value: &str) -> String {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            let mut output = characters
                .next()
                .map(|character| character.to_ascii_uppercase().to_string())
                .unwrap_or_default();
            output.extend(characters);
            output
        })
        .collect()
}
