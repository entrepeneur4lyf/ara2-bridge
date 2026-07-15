//! Deterministic generation of document-controller callback shells and coverage joins.

use crate::{bindings, Mode};
use std::path::Path;

type DynError = Box<dyn std::error::Error>;

pub(crate) const CALLBACKS: &[&str] = &[
    "destroyDocumentController",
    "getFactory",
    "beginEditing",
    "endEditing",
    "notifyModelUpdates",
    "beginRestoringDocumentFromArchive",
    "endRestoringDocumentFromArchive",
    "storeDocumentToArchive",
    "updateDocumentProperties",
    "createMusicalContext",
    "updateMusicalContextProperties",
    "updateMusicalContextContent",
    "destroyMusicalContext",
    "createAudioSource",
    "updateAudioSourceProperties",
    "updateAudioSourceContent",
    "enableAudioSourceSamplesAccess",
    "deactivateAudioSourceForUndoHistory",
    "destroyAudioSource",
    "createAudioModification",
    "cloneAudioModification",
    "updateAudioModificationProperties",
    "deactivateAudioModificationForUndoHistory",
    "destroyAudioModification",
    "createPlaybackRegion",
    "updatePlaybackRegionProperties",
    "destroyPlaybackRegion",
    "isAudioSourceContentAvailable",
    "isAudioSourceContentAnalysisIncomplete",
    "requestAudioSourceContentAnalysis",
    "getAudioSourceContentGrade",
    "createAudioSourceContentReader",
    "isAudioModificationContentAvailable",
    "getAudioModificationContentGrade",
    "createAudioModificationContentReader",
    "isPlaybackRegionContentAvailable",
    "getPlaybackRegionContentGrade",
    "createPlaybackRegionContentReader",
    "getContentReaderEventCount",
    "getContentReaderDataForEvent",
    "destroyContentReader",
    "createRegionSequence",
    "updateRegionSequenceProperties",
    "destroyRegionSequence",
    "getPlaybackRegionHeadAndTailTime",
    "restoreObjectsFromArchive",
    "storeObjectsToArchive",
    "getProcessingAlgorithmsCount",
    "getProcessingAlgorithmProperties",
    "getProcessingAlgorithmForAudioSource",
    "requestProcessingAlgorithmForAudioSource",
    "isLicensedForCapabilities",
    "storeAudioSourceToAudioFileChunk",
    "isAudioModificationPreservingAudioSourceSignal",
];

fn message(text: impl Into<String>) -> DynError {
    std::io::Error::other(text.into()).into()
}

/// Generates or checks the plug-in callback derivative below `root`.
pub fn generate(root: &Path, mode: Mode) -> Result<(), DynError> {
    let raw = std::fs::read_to_string(root.join("ara2-bridge-sys/src/generated/x86_64.rs"))?;
    let fields = controller_fields(&raw)?;
    let callbacks = fields
        .iter()
        .filter(|(name, _)| name != "structSize")
        .collect::<Vec<_>>();
    let actual = callbacks
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    if actual != CALLBACKS {
        return Err(message(
            "document-controller callback order differs from the audited manifest",
        ));
    }
    let rendered = bindings::format_rust(&render(&callbacks))?;
    bindings::validate_generated_metadata(&rendered)?;
    bindings::apply(
        mode,
        &root.join("ara2-bridge-plugin/src/ffi/generated_callbacks.rs"),
        rendered.as_bytes(),
    )
}

pub(crate) fn controller_fields(raw: &str) -> Result<Vec<(String, String)>, DynError> {
    let marker = "pub struct ARADocumentControllerInterface {";
    let start = raw
        .find(marker)
        .ok_or_else(|| message("missing ARADocumentControllerInterface"))?
        + marker.len();
    let bytes = raw.as_bytes();
    let mut cursor = start;
    let mut fields = Vec::new();
    loop {
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if bytes.get(cursor) == Some(&b'}') {
            break;
        }
        if raw.get(cursor..cursor + 4) != Some("pub ") {
            return Err(message("unexpected document-controller field syntax"));
        }
        cursor += 4;
        let colon = raw[cursor..]
            .find(':')
            .map(|offset| cursor + offset)
            .ok_or_else(|| message("unterminated document-controller field name"))?;
        let name = raw[cursor..colon].trim().to_owned();
        cursor = colon + 1;
        let type_start = cursor;
        let mut angle = 0_i32;
        let mut paren = 0_i32;
        let mut bracket = 0_i32;
        loop {
            let byte = *bytes
                .get(cursor)
                .ok_or_else(|| message("unterminated document-controller field type"))?;
            match byte {
                b'<' => angle += 1,
                b'>' if angle > 0 && bytes.get(cursor.wrapping_sub(1)) != Some(&b'-') => angle -= 1,
                b'(' => paren += 1,
                b')' => paren -= 1,
                b'[' => bracket += 1,
                b']' => bracket -= 1,
                b',' if angle == 0 && paren == 0 && bracket == 0 => break,
                _ => {}
            }
            cursor += 1;
        }
        fields.push((name, raw[type_start..cursor].trim().to_owned()));
        cursor += 1;
    }
    Ok(fields)
}

fn render(callbacks: &[&(String, String)]) -> String {
    let mut output = bindings::rust_banner();
    output.push_str("//! Generated ARA document-controller callback shells and coverage join.\n\n");
    output.push_str(
        "#![allow(non_snake_case, unused_variables, unsafe_op_in_unsafe_fn, clippy::undocumented_unsafe_blocks)]\n\n",
    );
    output.push_str("use ara2_bridge_sys::*;\n\n");
    output.push_str("pub(crate) trait ControllerDelegate {\n");
    for (name, field_type) in callbacks {
        let signature = callback_signature(field_type);
        let (parameters, return_type) = signature_parts(signature);
        output.push_str(&format!("    fn {}(&mut self", snake(name)));
        for parameter in parameters.iter().skip(1) {
            output.push_str(", ");
            output.push_str(parameter);
        }
        output.push(')');
        if let Some(return_type) = return_type {
            output.push_str(" -> ");
            output.push_str(return_type);
        }
        output.push_str(" {\n");
        if let Some(return_type) = return_type {
            output.push_str("        ");
            output.push_str(default_return(name, return_type));
            output.push('\n');
        }
        output.push_str("    }\n");
    }
    output.push_str("}\n\n");
    for (name, field_type) in callbacks.iter() {
        let signature = callback_signature(field_type);
        let (parameters, return_type) = signature_parts(signature);
        let function = snake(name);
        let declaration = signature.replacen(
            "unsafe extern \"C\" fn",
            &format!("pub(crate) unsafe extern \"C\" fn {function}"),
            1,
        );
        output.push_str(&declaration);
        output.push_str(" {\n");
        let fallback = return_type.map_or("()", |return_type| default_return(name, return_type));
        let arguments = parameters
            .iter()
            .skip(1)
            .map(|parameter| {
                parameter
                    .split_once(':')
                    .expect("callback parameter has a name")
                    .0
                    .trim()
            })
            .collect::<Vec<_>>()
            .join(", ");
        if name == "destroyDocumentController" {
            output.push_str(&format!(
                "    super::callbacks::dispatch(controllerRef, {fallback}, |delegate| delegate.{function}({arguments}));\n"
            ));
            output.push_str("    super::callbacks::destroy_controller_ref(controllerRef)\n");
        } else {
            output.push_str(&format!(
                "    super::callbacks::dispatch(controllerRef, {fallback}, |delegate| delegate.{function}({arguments}))\n"
            ));
        }
        output.push_str("}\n\n");
    }
    output.push_str(
        "pub(crate) fn raw_interface(struct_size: usize) -> ARADocumentControllerInterface {\n",
    );
    output.push_str("    ARADocumentControllerInterface {\n        structSize: struct_size,\n");
    for (name, _) in callbacks {
        output.push_str(&format!("        {name}: Some({}),\n", snake(name)));
    }
    output.push_str("    }\n}\n\n");
    output.push_str("pub(crate) fn represented_callbacks_are_non_null(raw: &ARADocumentControllerInterface, count: usize) -> bool {\n");
    output.push_str("    let callbacks = [\n");
    for (name, _) in callbacks {
        output.push_str(&format!(
            "        unsafe {{ ::std::ptr::addr_of!(raw.{name}).read_unaligned().is_some() }},\n"
        ));
    }
    output.push_str("    ];\n    callbacks.iter().take(count).all(|present| *present)\n}\n\n");
    output.push_str("/// Exact one-to-one join from C callback slots to generated delegates.\n");
    output.push_str("pub static PLUGIN_DELEGATES: &[super::callbacks::Delegate] = &[\n");
    for (index, (name, _)) in callbacks.iter().enumerate() {
        output.push_str(&format!(
            "    super::callbacks::Delegate::new(\"{name}\", {index}),\n"
        ));
    }
    output.push_str("]\n;\n\n");
    output.push_str("/// Required conformance-test entry for every generated callback slot.\n");
    output
        .push_str("pub static PLUGIN_CONTRACT_TESTS: &[super::callbacks::CallbackContract] = &[\n");
    for (index, (name, _)) in callbacks.iter().enumerate() {
        output.push_str(&format!(
            "    super::callbacks::CallbackContract::new(\"{name}\", {index}),\n"
        ));
    }
    output.push_str("]\n;\n");
    output
}

pub(crate) fn callback_signature(field_type: &str) -> &str {
    let inner = field_type
        .trim()
        .strip_prefix("::std::option::Option<")
        .and_then(|value| value.strip_suffix('>'))
        .expect("document-controller callbacks are optional raw function pointers")
        .trim();
    inner.strip_suffix(',').unwrap_or(inner).trim()
}

pub(crate) fn signature_parts(signature: &str) -> (Vec<&str>, Option<&str>) {
    let open = signature.find("fn(").expect("callback function signature") + 2;
    let bytes = signature.as_bytes();
    let mut cursor = open + 1;
    let mut depth = 0_i32;
    let close = loop {
        match bytes[cursor] {
            b'(' => depth += 1,
            b')' if depth == 0 => break cursor,
            b')' => depth -= 1,
            _ => {}
        }
        cursor += 1;
    };
    let parameters = split_parameters(&signature[open + 1..close]);
    let return_type = signature[close + 1..]
        .trim()
        .strip_prefix("->")
        .map(str::trim);
    (parameters, return_type)
}

fn split_parameters(parameters: &str) -> Vec<&str> {
    if parameters.trim().is_empty() {
        return Vec::new();
    }
    let bytes = parameters.as_bytes();
    let mut result = Vec::new();
    let mut start = 0;
    let mut angle = 0_i32;
    let mut paren = 0_i32;
    for (cursor, byte) in bytes.iter().copied().enumerate() {
        match byte {
            b'<' => angle += 1,
            b'>' if angle > 0 && bytes.get(cursor.wrapping_sub(1)) != Some(&b'-') => angle -= 1,
            b'(' => paren += 1,
            b')' => paren -= 1,
            b',' if angle == 0 && paren == 0 => {
                let parameter = parameters[start..cursor].trim();
                if !parameter.is_empty() {
                    result.push(parameter);
                }
                start = cursor + 1;
            }
            _ => {}
        }
    }
    let parameter = parameters[start..].trim();
    if !parameter.is_empty() {
        result.push(parameter);
    }
    result
}

fn default_return(callback: &str, return_type: &str) -> &'static str {
    if callback == "isLicensedForCapabilities" {
        return "kARATrue";
    }
    if return_type.starts_with("*const") || return_type == "ARAPersistentID" {
        "::std::ptr::null()"
    } else if return_type.starts_with("*mut") || return_type.ends_with("Ref") {
        "::std::ptr::null_mut()"
    } else if return_type == "ARABool" {
        "kARAFalse"
    } else {
        "0"
    }
}

pub(crate) fn snake(name: &str) -> String {
    let mut output = String::with_capacity(name.len() + 8);
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_uppercase() && index != 0 {
            output.push('_');
        }
        output.push(character.to_ascii_lowercase());
    }
    output
}
