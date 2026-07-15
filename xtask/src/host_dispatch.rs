//! Deterministic generation of host-side document-controller call shells.

use crate::{bindings, plugin_dispatch, Mode};
use std::path::Path;

type DynError = Box<dyn std::error::Error>;

fn message(text: impl Into<String>) -> DynError {
    std::io::Error::other(text.into()).into()
}

/// Generates or checks the host dispatch derivative below `root`.
pub fn generate(root: &Path, mode: Mode) -> Result<(), DynError> {
    let raw = std::fs::read_to_string(root.join("ara2-bridge-sys/src/generated/x86_64.rs"))?;
    let fields = plugin_dispatch::controller_fields(&raw)?;
    let callbacks = fields
        .iter()
        .filter(|(name, _)| name != "structSize")
        .collect::<Vec<_>>();
    let actual = callbacks
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    if actual != plugin_dispatch::CALLBACKS {
        return Err(message(
            "document-controller callback order differs from the audited manifest",
        ));
    }
    let rendered = bindings::format_rust(&render(&callbacks))?;
    bindings::validate_generated_metadata(&rendered)?;
    bindings::apply(
        mode,
        &root.join("ara2-bridge-host/src/plugin/generated_dispatch.rs"),
        rendered.as_bytes(),
    )
}

fn render(callbacks: &[&(String, String)]) -> String {
    let mut output = bindings::rust_banner();
    output.push_str("//! Generated host-to-plug-in document-controller call shells.\n\n");
    output.push_str("#![allow(dead_code, non_snake_case, unsafe_op_in_unsafe_fn, clippy::undocumented_unsafe_blocks)]\n\n");
    output.push_str("use ara2_bridge_core::AraError;\nuse ara2_bridge_sys::*;\n\n");
    output.push_str("/// Exact generated description of one controller callback.\n");
    output.push_str("#[derive(Clone, Copy, Debug, Eq, PartialEq)]\n");
    output.push_str("pub struct DispatchMethod {\n    /// Released C callback name.\n    pub c_name: &'static str,\n    /// Zero-based slot in released header order.\n    pub index: usize,\n    /// Byte offset of the callback field.\n    pub field_offset: usize,\n    /// Minimum represented interface size for this slot.\n    pub field_extent: usize,\n}\n\n");
    output.push_str("/// All controller callbacks in released header order.\n");
    output.push_str("pub static DISPATCH_METHODS: &[DispatchMethod] = &[\n");
    for (index, (name, _)) in callbacks.iter().enumerate() {
        output.push_str(&format!(
            "    DispatchMethod {{ c_name: \"{name}\", index: {index}, field_offset: ::std::mem::offset_of!(ARADocumentControllerInterface, {name}), field_extent: layout::ARADOCUMENT_CONTROLLER_INTERFACE_{} }},\n",
            screaming(name)
        ));
    }
    output.push_str("];\n\n");
    for (name, field_type) in callbacks {
        let signature = plugin_dispatch::callback_signature(field_type);
        let (parameters, return_type) = plugin_dispatch::signature_parts(signature);
        let function = plugin_dispatch::snake(name);
        output.push_str("pub(crate) unsafe fn ");
        output.push_str(&function);
        output.push_str("(interface: *const ARADocumentControllerInterface");
        for parameter in &parameters {
            output.push_str(", ");
            output.push_str(parameter);
        }
        output.push_str(") -> Result<");
        output.push_str(return_type.unwrap_or("()"));
        output.push_str(", AraError> {\n");
        output.push_str("    let callback = unsafe { super::dispatch::callback::<");
        output.push_str(signature);
        output.push_str(">(interface, ");
        output.push_str(&format!(
            "layout::ARADOCUMENT_CONTROLLER_INTERFACE_{}, ::std::mem::offset_of!(ARADocumentControllerInterface, {name}), \"{name}\")? }};\n",
            screaming(name)
        ));
        let arguments = parameters
            .iter()
            .map(|parameter| {
                parameter
                    .split_once(':')
                    .expect("callback parameter has a name")
                    .0
                    .trim()
            })
            .collect::<Vec<_>>()
            .join(", ");
        if return_type.is_some() {
            output.push_str(&format!("    Ok(unsafe {{ callback({arguments}) }})\n"));
        } else {
            output.push_str(&format!(
                "    unsafe {{ callback({arguments}) }};\n    Ok(())\n"
            ));
        }
        output.push_str("}\n\n");
    }
    output.push_str("impl super::DocumentController<'_, '_> {\n");
    for (name, field_type) in callbacks {
        if name == "destroyDocumentController" {
            continue;
        }
        let signature = plugin_dispatch::callback_signature(field_type);
        let (parameters, return_type) = plugin_dispatch::signature_parts(signature);
        let function = plugin_dispatch::snake(name);
        output.push_str("    pub(crate) unsafe fn raw_");
        output.push_str(&function);
        output.push_str("(&mut self");
        for parameter in parameters.iter().skip(1) {
            output.push_str(", ");
            output.push_str(parameter);
        }
        output.push_str(") -> Result<");
        output.push_str(return_type.unwrap_or("()"));
        output.push_str(", AraError> {\n        unsafe { ");
        output.push_str(&function);
        output.push_str("(self.interface_ptr(), self.as_raw_ref()");
        for parameter in parameters.iter().skip(1) {
            output.push_str(", ");
            output.push_str(
                parameter
                    .split_once(':')
                    .expect("callback parameter has a name")
                    .0
                    .trim(),
            );
        }
        output.push_str(") }\n    }\n\n");
    }
    output.push_str("}\n");
    output
}

fn screaming(name: &str) -> String {
    let mut output = String::with_capacity(name.len() + 8);
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_uppercase() && index != 0 {
            output.push('_');
        }
        output.push(character.to_ascii_uppercase());
    }
    output
}
