# Phase 5 Handoff — Companion Integrations

Status: portable implementation complete; locked native evidence pending
Baseline: ARA API `releases/2.3.0`, normative commit `65ec5c43b943a48cb5446f448a0492db6af8534b`

## Implemented surface

`ara2-bridge-companion` provides a companion-neutral, one-shot processor binding plus reciprocal
plug-in and host adapters for CLAP, VST3, and Audio Unit v2. The adapters share the exact
`ARAFactory` pointer used by the core runtime, validate known and assigned roles, reject binding
after processor lifecycle boundaries, and support controller-first or companion-first teardown.
Companion audio processing, state, and GUI implementation remain owned by the integrating format.

CLAP uses checked-in Rust declarations generated from the locked CLAP 1.1.9 inputs. VST3 crosses
an opaque, exception-contained C++17 C ABI; no C++ object layout is exposed to Rust. Audio Unit v2
uses an Apple-only Objective-C++ property shim backed by the pinned AudioUnitSDK 1.0.0 headers.

## Features and SDK routing

- `clap` is portable and requires no SDK during consumer builds.
- `vst3` requires `ARA_VST3_SDK_DIR` and an operator-selected
  `ARA_VST3_LICENSE_POLICY` (`GPL-3.0-only` or `LicenseRef-Steinberg-VST3`).
- `audio-unit-v2` requires Apple targets and `ARA_AUDIO_UNIT_SDK_DIR`.
- `full-portable` enables plug-in, host, CLAP, and VST3; `full-apple` adds Audio Unit v2.

Builds never download SDKs. Provisioning and exact identities are documented in
`docs/companion-sdk-setup.md`; every native build validates repository, commit, tree, submodule,
license, and clean-state invariants before compilation.

## Portable gate evidence

- workspace tests, all-target clippy with warnings denied, and strict rustdoc pass;
- CLAP ABI, interoperability, provenance, and three canonical target probe families pass;
- strict-provenance Miri passes for the neutral binding and CLAP adapters;
- VST3 compatibility compilation and Rust adapter tests pass against a non-authoritative header
  set; this is diagnostic evidence only;
- non-Apple Audio Unit feature builds fail with the documented platform diagnostic;
- CLAP, VST3, and Audio Unit companion symbol manifests close all 47 unique symbols represented by
  the 49 companion-deferred core inventory records.

## Pending native release evidence

The VST3 provenance manifest, five canonical native probe results, and locked SDK test gate remain
pending until the operator chooses the applicable VST3 license policy. Audio Unit v2 native tests
and its two canonical probes require the configured macOS CI runners. These are release blockers,
not implementation waivers. CI contains the corresponding jobs and artifact emission paths.

## Revisions discovered during implementation

- Native companion boundaries must expose opaque C handles and explicit reference transfers.
- Secondary VST3 interfaces must return the canonical primary object for `FUnknown` queries.
- Plug-in adapters need an explicit controller-destruction observation because the companion may
  outlive the ARA controller.
- Audio Unit property failures must preserve caller output bytes and validate the ARA magic value
  before delegation.
- VST3 license selection is an operator policy decision and cannot be inferred by the build.
