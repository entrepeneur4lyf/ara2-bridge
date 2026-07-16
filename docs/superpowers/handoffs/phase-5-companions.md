# Phase 5 Handoff — Companion Integrations

Status: implementation and locked native evidence complete
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
- `vst3` requires `ARA_VST3_SDK_DIR` pointing to the locked MIT-licensed
  VST3 `v3.8.0_build_66` checkout; provisioning accepts the literal `MIT` identifier.
- `audio-unit-v2` requires Apple targets and `ARA_AUDIO_UNIT_SDK_DIR`.
- `full-portable` enables plug-in, host, CLAP, and VST3; `full-apple` adds Audio Unit v2.

Builds never download SDKs. Provisioning and exact identities are documented in
`docs/companion-sdk-setup.md`; every native build validates repository, commit, tree, submodule,
license, and clean-state invariants before compilation.

## Gate evidence

- workspace tests, all-target clippy with warnings denied, and strict rustdoc pass;
- CLAP ABI, interoperability, provenance, and three canonical target probe families pass;
- strict-provenance Miri passes for the neutral binding and CLAP adapters;
- VST3 provenance, ABI, and interoperability tests pass against the locked 3.8/MIT SDK on Linux
  x86_64, Windows x86_64, and macOS x86_64/AArch64;
- all five canonical VST3 probes pass: Linux x86_64/AArch64, Windows x86_64, and macOS
  x86_64/AArch64;
- Audio Unit v2 provenance, ABI, interoperability, and both canonical probes pass natively on
  macOS x86_64/AArch64;
- non-Apple Audio Unit feature builds fail with the documented platform diagnostic;
- CLAP, VST3, and Audio Unit companion symbol manifests close all 47 unique symbols represented by
  the 49 companion-deferred core inventory records.

Linux AArch64 VST3 evidence was produced by a target-compiled runner under system emulation. Runner
identity is derived from the compiled binary, so a host `rustc` cannot mislabel a cross-target
probe. Windows SDK bootstrap forces LF-preserving Git configuration before checkout so provenance
hashes remain byte-identical across operating systems. Automation may reproduce these results but
does not create or authorize a release.

## Revisions discovered during implementation

- Native companion boundaries must expose opaque C handles and explicit reference transfers.
- Secondary VST3 interfaces must return the canonical primary object for `FUnknown` queries.
- Plug-in adapters need an explicit controller-destruction observation because the companion may
  outlive the ARA controller.
- Audio Unit property failures must preserve caller output bytes and validate the ARA magic value
  before delegation.
- VST3 3.8 is pinned under MIT; the old operator-selected GPL/proprietary policy is removed.
- Probe target identity follows the compiled runner rather than the toolchain installed on its host.
- SDK bootstrap disables Git line-ending conversion before checkout for portable provenance hashes.
