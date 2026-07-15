# ARA2 Bridge Specification Audit

Date: 2026-07-14  
Scope: `docs/specs/ara2-bridge/` against Celemony ARA SDK 2.3.0

## Evidence

The audit compared the specifications with the pinned ARA API headers, `ARA_Library` dispatch behavior, TestHost/TestPlugIn expectations, companion headers, and the local SDK provenance. It also mechanically verified that `api-compatibility.toml` parses, lists every callback in exact header order, contains all 54 `ARADocumentControllerInterface` callbacks exactly once, covers every host and extension interface, and classifies every `structSize`-versioned surface.

## Revisions made

The initial reviews found and closed issues in foreign-pointer safety preconditions, per-factory generation state, ARA2 role enablement, reliable 2.3 notifications, host audio threading/layout, host-content callback scope, compatibility prefixes and fallbacks, CLAP provenance, audio-file chunk defaults/cardinality, and Linux AArch64 runtime coverage. A follow-up review also corrected nullable-vtable wording: every represented function pointer is non-null; optionality uses a shorter prefix or a non-null semantic default.

The final compatibility manifest matched these SDK counts: factory 3 callbacks, host interfaces 3/6/9/5/5 callbacks, document controller 54 callbacks, and extension interfaces 2/2/4/2 callbacks. No duplicate or out-of-order slot was found.

## Result

The final focused independent audit returned `CLEAR`. The specification gate is closed. Implementation discoveries that alter behavior, public API shape, safety invariants, or boundaries must revise the affected normative spec in the same change.

## Implementation re-audit

On 2026-07-15, Phase 1 evidence clarified ownership of companion-defined variable channel-layout payloads. Specification `02` now requires a validated data-type-specific extent and explicitly returns `Unsupported` until the corresponding companion adapter can provide one. Phase 2 decoder evidence also made specification `01` explicit that bindgen-omitted or widened scalar macros are normalized to their C types and checked at compile time. Archive-filter evidence clarified that one document-scoped `RegistrySession` is shared across typed registries. These changes preserve the existing ownership, pointer-validation, ABI, and companion-boundary rules and introduce no contradiction with specs `05` or `06`. The focused re-audit result is `CLEAR`.

On 2026-07-15, native macOS and Rosetta validation exposed four portability assumptions. Specifications `01`, `06`, `07`, and `09` now require host-independent coverage discovery, checkout-independent probe hashes, explicit target-inapplicable generation scenarios, and compiler-stable negative auto-trait assertions. These revisions preserve the canonical target matrix, per-family ABI probes, zero capability-skip rule, and ownership model; they neither synthesize ARA1 on AArch64 nor substitute cross-compilation for native interoperability. The focused re-audit found no conflicting acceptance criteria or weakened gate. Result: `CLEAR`.

On 2026-07-15, audio-file implementation evidence refined specification `05`: XML extensions are retained by structural templates while typed scalar values are canonicalized in place; RF64/BW64 iXML sentinel sizes remain linked to their `ds64` table entry; and XML/container structural allocations are bounded before allocation. These refinements strengthen forward compatibility and hostile-input behavior without changing the accepted schema, supported container set, or atomic-replacement contract. The focused re-audit result is `CLEAR`.

The subsequent utility source port made the negative half-sample boundary explicit, separated safe typed channel layouts from unsafe future opaque layouts, and bound processing strings and license subsets to controller-owned validation objects. These rules directly match the pinned ARA Library algorithms and API lifetime text, preserve unknown future flag bits, and do not conflict with the earlier FFI ownership rules. The focused re-audit result remains `CLEAR`.
