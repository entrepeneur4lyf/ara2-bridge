# Companion API Integrations

Status: Normative component specification  
Depends on: [Plug-in Runtime](03-plugin-runtime.md), [Host Runtime](04-host-runtime.md)  
Last revised: 2026-07-15

## Scope

`ara2-bridge-companion` binds an ARA factory and plug-in extension to the redistributable companion mechanisms included in ARA SDK 2.3: CLAP, VST3, and Audio Unit v2. It supplies both plug-in exposure and host discovery/binding paths. It is not a replacement for the companion SDK's audio-processing API.

## Common contract

Every adapter shall:

1. expose a factory pointer through every factory/instance path that the specific companion API defines, with identical pointers where multiple paths exist;
2. discover ARA capability without instantiating every audio processor where that companion API permits;
3. bind a companion instance to one document controller at most once and before the companion-specific activation/state/UI boundary;
4. validate known and assigned role flags and return an extension instance containing exactly those supported roles;
5. keep both possible destruction orders safe: companion first or document controller first; and
6. preserve initialization balance and thread/realtime rules across adapter callbacks.

Adapters use shared runtime objects; they do not duplicate document or factory state. `CompanionProcessorBinding` is the explicit integration boundary with an externally owned audio processor. It exposes factory lookup, one-shot ARA binding, lifecycle observations (state load, activation, view creation, destruction), role capabilities, and shared render/model state. The bridge never implements the companion audio processor, DSP callback, state format, or GUI.

## CLAP

The `clap` feature implements and consumes:

- `CLAP_EXT_ARA_FACTORY` (`org.ara-audio.ara.factory/2`), including factory count, ARA factory lookup, and associated CLAP plug-in ID;
- `CLAP_EXT_ARA_PLUGINEXTENSION` (`org.ara-audio.ara.pluginextension/2`), including factory lookup and role-aware controller binding;
- the temporary draft-compatible IDs accepted by SDK 2.3;
- `ara:supported` and `ara:required` feature declarations.

Index and ID lifetimes last through CLAP entry deinitialization. Binding must precede activation, state load, processing-related extension use, and GUI creation. Tests cover binaries containing multiple CLAP plug-ins with only a subset ARA-capable.

## VST3

The `vst3` feature supplies audited implementations/wrappers for:

- `ARA::IMainFactory` and its published IID/class category;
- `ARA::IPlugInEntryPoint`, including generation-1 `bindToDocumentController` semantics;
- `ARA::IPlugInEntryPoint2` and role-aware binding;
- COM query-interface/reference-count ownership and processor/factory class matching.

The main-factory class name matches its associated processor class name, and processor `PClassInfo.name` matches `ARAFactory::plugInName`; ambiguous duplicate matches are rejected. Factory pointers are identical across interfaces. Role-aware binding precedes `setActive`, state/process-context setup, or view creation. An audited C++ shim built against the MIT-licensed VST3 SDK `v3.8.0_build_66`, commit `9fad9770f2ae8542ab1a548a68c1ad1ac690abe0`, supplies COM interfaces and catches foreign exceptions. Windows MSVC-target crate builds and fresh probes use the `clang-cl` driver, select the MSVC C++17 dialect (`/std:c++17`), and enable standard C++ exception unwinding (`/EHsc`) so the containment boundary is compiled rather than rejected by `clang-cl`'s default exception-disabled mode. The shim publishes only an `extern "C"` API: Rust never names a C++ layout. An external processor delegates the three ARA IID queries to `Vst3PluginEntryAdapter`; every successful query transfers one COM reference. Matching rejects duplicate IDs, names, and factory pointers as well as missing or inconsistent names.

## Audio Unit

The `audio-unit-v2` feature is Apple-target-only and implements both host and plug-in handling for:

- the `ARA` Audio Component tag;
- `kAudioUnitProperty_ARAFactory` using `ARAAudioUnitFactory`;
- deprecated `kAudioUnitProperty_ARAPlugInExtensionBinding` with generation-1 role semantics;
- `kAudioUnitProperty_ARAPlugInExtensionBindingWithRoles` using the full binding structure;
- `kARAAudioUnitMagic` input validation and output preservation.

Properties use global scope and exact read-only behavior. AU discovery is instance-property based; the component tag supports cache discovery but is not a factory-level pointer path. Role-aware binding precedes initialization, state/preset assignment, and custom view creation. The shim builds against AudioUnitSDK `AudioUnitSDK-1.0.0` plus platform Core Audio headers. An external `AUBase` subclass delegates only the three ARA properties to `AudioUnitPluginAdapter`; the bridge remains outside the audio processor inheritance hierarchy. Failures preserve caller-owned output fields, while host reads validate size, mutability, magic, unchanged inputs, and non-null output.

## Features and dependencies

Companion features are independent and off by default in the companion crate. `clap`, `vst3`, and `audio-unit-v2` enable only their required external dependencies. CLAP declarations are generated directly from CLAP tag `1.1.9`, commit `094bb76c85366a13cc6c49292226d8608d6ae50c`; no Rust CLAP sys crate is used. The project-local installer clones ARA recursively plus VST3 `v3.8.0_build_66`, CLAP 1.1.9, and AudioUnitSDK 1.0.0, builds the available examples, and records `ARA_SDK_DIR` plus applicable companion paths in the consuming project's `.cargo/config.toml`. Cargo builds consume those paths but never download code. The checked-in provenance manifest records the exact tag, commit, repository URL, SPDX license, and SHA-256 of every transitively consumed CLAP header and every shim input. Provisioning accepts the exact recorded license identifier (`MIT` for VST3) and rejects commit, tree, submodule, or hash drift.

Pure constant, field-offset, size, and alignment probe envelopes may be produced either by executing the probe natively or by compiling the exact probe/header set with a target compiler and consuming its record-layout output. Every envelope records the evidence method, target, input/probe/payload hashes, and SDK commit/tree. Source hashes use a domain-separated, length-framed sequence of repository-relative, slash-separated input paths and file bytes; absolute checkout paths and platform path separators never affect an envelope. Target-compiler evidence does not satisfy runtime interoperability; host/plug-in behavior and native shim ownership still run on each required platform job.

External SDK versions and licenses are documented in Cargo metadata and the manual. Build scripts may locate an explicitly configured SDK but may not download code or accept licenses implicitly. Missing SDKs produce actionable configuration errors.

## AAX boundary

AAX is fully supported by the ARA 2.3 ecosystem but cannot be implemented or redistributed from this checkout because its definitions are inside Avid's proprietary SDK. It is an explicit externally licensed adapter boundary. Audio Unit v3 is also excluded from 2.3 production scope because the released changelog postpones its App Extension/IPC contract and the example references an absent IPC header. Core contracts remain companion-neutral so either adapter can be added later without changing host or plug-in APIs.

## Acceptance criteria

For each enabled companion, a minimal external test processor using `CompanionProcessorBinding` is discoverable, exposes the correct factory, binds once with roles, renders/edits through shared test state, and survives both teardown orders. A Rust host performs the reciprocal discovery and binding. CLAP runs in portable CI; VST3 and Audio Unit v2 run on configured platform jobs. ABI probes compare every companion constant, IID, property, and structure with the upstream header.

## Decisions and revisions

- 2026-07-14: Companion adapters are feature-gated and share core runtime state.
- 2026-07-14: No automatic third-party SDK downloads or license acceptance.
- 2026-07-14: AAX and postponed AUv3 remain explicit external boundaries.
- 2026-07-14: Audit added the external audio-processor binding contract and pinned companion SDK inputs.
- 2026-07-15: Constant/layout-only companion envelopes may use deterministic target-compiler record layouts; runtime interoperability remains a separate native-platform requirement.
- 2026-07-15: The neutral processor boundary permanently rejects binding after an observed state, activation, processing, or view boundary and retains tombstoned state through either controller/processor teardown order.
- 2026-07-15: Native adapters expose `observe_controller_destruction`; hosts call it before controller-first teardown so later boundaries fail without dereferencing stale controller state.
- 2026-07-15: VST3 COM objects and Audio Unit property handlers are opaque C ABI allocations; C++ exceptions and ownership never cross into Rust.
- 2026-07-15: VST3 moved from the obsolete 3.7 dual-license pin to `v3.8.0_build_66`, commit `9fad9770f2ae8542ab1a548a68c1ad1ac690abe0`, under MIT. Provisioning requires the literal recorded `MIT` identifier; there is no operator-selected VST3 policy variable.
- 2026-07-15: Probe source identities use normalized repository-relative paths so independently located Linux and macOS checkouts produce the same canonical hash.
- 2026-07-15: Windows validation requires both the VST3 crate build and fresh probe to use `/std:c++17` and `/EHsc`, preserving the specified native-exception containment boundary under `clang-cl` and MSVC.
- 2026-07-16: The recursive project-local installer supplies ARA and companion SDK paths through relocatable Cargo configuration while build scripts remain download-free.
