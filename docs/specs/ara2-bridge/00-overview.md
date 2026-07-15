# ARA2 Bridge 2.3 — System Overview

Status: Normative design specification  
Baseline: Celemony ARA SDK `releases/2.3.0`  
Last revised: 2026-07-15

## Purpose

`ara2-bridge` shall provide complete Rust support for authoring both ARA plug-ins and ARA hosts against the redistributable public surface of ARA SDK 2.3.0. “Complete” means every core interface, required compatibility behavior, companion API present in the supplied public SDK, audio-file chunk operation, and upstream TestHost scenario has a supported Rust path. The C ABI remains defined by Celemony; this project supplies ABI bindings, safe dispatch, lifecycle enforcement, Rust-native authoring APIs, integrations, and conformance tooling.

The SDK headers are normative for binary layout. The SDK documentation, `ARA_Library`, TestHost, and TestPlugIn are normative behavioral references. Where Rust safety requires a different shape than the C++ library, observable ARA behavior must remain equivalent.

## Design principles

1. **ABI exactness before ergonomics.** Layout, calling convention, struct-size negotiation, nullability, and API-generation rules are tested facts.
2. **Unsafe code is infrastructure.** Application authors use safe APIs for normal operation. Every unsafe boundary states its ownership, lifetime, alignment, thread, and panic invariants.
3. **Capabilities are explicit.** Optional ARA functions and extension roles are registered deliberately; absence produces the exact null slot or capability response prescribed by ARA.
4. **Lifecycle is encoded.** Editing, restoration, content-reader, sample-access, rendering, and destruction rules are enforced by ownership, guards, typestate, or checked runtime transitions.
5. **Host and plug-in are peers.** Shared semantics live in core; neither side is implemented as an afterthought.
6. **Reference behavior is executable.** Upstream examples become tests, not prose-only aspirations.
7. **Documentation is a deliverable.** Public APIs, examples, failure modes, and thread rules must be suitable as source material for the later manual.

## Workspace architecture

The workspace shall evolve into these bounded crates:

```text
ara2-bridge-sys       generated and audited C ABI
        │
ara2-bridge-core      shared types, validation, dispatch, safety
   ┌────┴────┐
plugin       host     complete plug-in and host authoring runtimes
   └────┬────┘
companion            CLAP, VST3, and Audio Unit adapters
        │
testkit              mock peers and conformance scenarios

ara2-bridge          facade and compatibility migration surface
```

Dependency edges point downward only. `core` depends on `sys`; ABI-owning `plugin` depends directly on both `core` and `sys`; `host` depends on `core` and adds a direct `sys` edge when its raw host vtables are introduced; feature-gated `companion` adapters depend on `core` and the relevant `plugin`/`host` runtime. `testkit` may depend on `sys`, `core`, `plugin`, `host`, and `companion`, but never on the facade. No focused production crate depends on `testkit`. The aggregation-only `ara2-bridge` facade may optionally depend on and re-export `testkit` behind its off-by-default `testkit` feature; because testkit never depends on the facade, this creates no cycle.

## Spec map

- [01-abi-and-generation.md](01-abi-and-generation.md): header provenance, generated bindings, layout, API generations, and ABI CI.
- [02-core-safety-and-dispatch.md](02-core-safety-and-dispatch.md): references, sized structs, strings, errors, panic containment, threading, and callback machinery.
- [03-plugin-runtime.md](03-plugin-runtime.md): factory, all 54 document-controller callbacks, model graph, extension roles, and plug-in lifecycle.
- [04-host-runtime.md](04-host-runtime.md): all five host interfaces, plug-in dispatch, graph editing, loading, and host lifecycle.
- [05-content-persistence-and-utilities.md](05-content-persistence-and-utilities.md): typed content, readers, analysis, archives, audio-file chunks, algorithms, licensing, and SDK utilities.
- [06-companion-integrations.md](06-companion-integrations.md): CLAP, VST3, Audio Unit, platform boundaries, and role binding.
- [07-conformance-and-quality.md](07-conformance-and-quality.md): upstream scenario parity, safety testing, CI matrix, and release gates.
- [08-packaging-versioning-and-manual.md](08-packaging-versioning-and-manual.md): features, migration, examples, rustdoc, and manual inputs.
- [09-generation-compatibility.md](09-generation-compatibility.md): ARA 1, ARA 2.0, and ARA 2.3 wire graphs, struct prefixes, fallbacks, and target restrictions.

An implementation session should load this overview, the active numbered spec, and directly referenced sections only.

## Supported and excluded scope

Included: ARA API generations through 2.3 Final where the target ABI exposes them; deprecated generation-1 entry points on non-ARM64 targets; plug-in and host interfaces; playback/editor roles; typed content events; partial persistence; audio-file chunks; processing-algorithm and license queries; CLAP, VST3, Audio Unit v2 integration; debug validation; and TestHost/TestPlugIn interoperability.

Excluded: DSP or analysis algorithms, DAW UI, a general audio plug-in framework, automatic archive schema design, Audio Unit v3 IPC, and proprietary AAX integration. ARA SDK 2.3 postpones AUv3, while AAX definitions ship only in Avid's separately licensed SDK. A separately licensed adapter may be added without changing core contracts. Project claims must say “complete public ARA SDK 2.3 support,” not imply that the absent proprietary AAX surface is included.

## Revision policy

These documents are living normative specifications. Implementation discoveries that change behavior, boundaries, safety invariants, or public API shape require a spec update in the same change. Revisions must update `Last revised`, explain the decision in the affected spec's “Decisions and revisions” section, and update cross-links or acceptance criteria. Purely mechanical implementation detail does not require a spec amendment.

When sources disagree, precedence is: released ARA 2.3 headers, released ARA documentation, observable upstream TestHost/TestPlugIn behavior, this suite, then implementation. Ambiguities must be recorded and resolved rather than silently guessed.

## System completion criteria

The system is complete when every item in the API coverage manifests is implemented or intentionally represented as an ARA-defined unsupported capability; the feature/platform matrix builds; ABI/layout tests pass on supported targets; the Rust host and plug-in pass all ported upstream scenarios against each other; cross-language TestHost/TestPlugIn interoperability passes; unsafe-code audit findings are closed; rustdoc contains no warnings or undocumented public items; and the manual outline can be populated without reverse-engineering implementation code.

## Decisions and revisions

- 2026-07-14: Full bidirectional support selected; breaking the unused `0.1.x` API is allowed.
- 2026-07-14: Layered handwritten runtimes with limited ABI/shim generation selected over a monolith or generated “safe” API.
- 2026-07-14: Specs split by implementation boundary to minimize context carried during each phase.
- 2026-07-14: Audit narrowed companion scope to released public CLAP, VST3, and Audio Unit v2; AUv3 and proprietary AAX are explicit external boundaries.
- 2026-07-15: Audit made the facade/testkit exception and acyclic dependency graph explicit.
- 2026-07-15: Implementation audit clarified that crates which publish raw ARA vtables require a direct downward `sys` edge in addition to `core`; hiding that compile-time ABI dependency behind a safe crate would misstate the actual architecture.
