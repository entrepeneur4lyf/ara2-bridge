# Conformance and Quality

Status: Normative cross-cutting specification  
Depends on: All preceding component specs  
Last revised: 2026-07-15

## Scope

`ara2-bridge-testkit` supplies mock peers, fixtures, ABI probes, scenario runners, validators, and cross-language adapters. This spec defines the evidence required to claim API completeness, safety, interoperability, and release readiness.

## Test layers

Tests are organized so the smallest useful layer runs first:

1. Generated-symbol, constant, signature, size/alignment, and field-offset tests.
2. Core unit, compile-fail, property, Miri, and fuzz tests.
3. Per-interface callback and dispatch contract tests using mock peers.
4. Rust host ↔ Rust plug-in lifecycle and scenario tests.
5. Rust host ↔ C++ TestPlugIn and C++ TestHost ↔ Rust plug-in interoperability.
6. Companion-format discovery/binding/render tests on supported platforms.

Failures must identify interface, method, host-selected generation, lifecycle state, and object identity. Test helpers may not bypass the public safe API except in tests explicitly exercising malformed foreign input.

## Upstream scenario parity

The upstream TestHost scenarios are a named conformance manifest. Each has a Rust scenario with equivalent setup, operations, assertions, and teardown:

- property updates;
- content updates;
- content reading;
- audio-modification cloning;
- full archiving;
- split/partial archives;
- drag and drop/import;
- playback rendering, with and without supported time stretching;
- editor view selection/visibility;
- processing algorithms;
- audio-file chunk loading;
- audio-file chunk saving.

The shared basic-document creation path also verifies factory initialization, graph construction, sample access, requested analysis, editing boundaries, and destruction. A capability-rich Rust TestPlugIn must enable partial persistence, algorithms, chunk writing, all content kinds, analysis, licensing, signal preservation, head/tail, and all roles so no optional scenario passes by skipping. Golden archives and chunk-bearing WAVE/AIFF fixtures prove that load paths found and restored compatible data. Additional bridge-specific scenarios cover deprecated generation-1 persistence, 2.3 document dirtiness, all extension-role combinations, poisoning, and both companion/controller destruction orders.

## Interface contract matrix

Every C function slot has tests for:

- successful argument conversion and delegation;
- minimum and full `structSize` peers;
- optional slot absence where permitted;
- null, misaligned, wrong-kind, stale, and foreign references;
- invalid count/pointer and numeric inputs;
- user error and peer false/null return;
- user panic or C++ exception containment;
- valid and invalid lifecycle/thread state;
- teardown and retained-allocation behavior.

The machine-readable ABI coverage manifest and the test manifest are joined in CI. Any public slot lacking a safe delegate or contract-test classification fails the build.

## Safety verification

Core registries, RAII guards, callback recovery, and destruction run under Miri. FFI integration runs under AddressSanitizer and UndefinedBehaviorSanitizer where supported. ThreadSanitizer runs both deterministic state models and integration tests that exercise the production analysis-job, audio-reader/access-revocation, and editor-renderer update paths. Loom or deterministic model tests remain a separate layer for interleavings representable without foreign code.

Fuzz targets cover every inbound versioned struct family, opaque reference lookup, content-event validation, archive filters, Base64/XML audio-file chunks, AIFF/WAVE chunk mutation, and generated dispatch decoders. Corpus seeds include upstream examples, boundary sizes, previous failures, and all API generations.

No test may rely only on process termination to prove cleanup. Allocation/reference counters and weak ownership observations verify release. Realtime tests instrument allocation, blocking synchronization, file I/O, and logging on designated callback paths.

## CI matrix

Required always-on jobs:

- formatting, workspace check, clippy with warnings denied, tests, and rustdoc warnings denied;
- MSRV and stable Rust;
- no-default-features and every additive feature combination that can compile on the runner;
- Linux x86_64, Linux AArch64 (native or system-emulated runtime conformance), Windows x86_64, macOS x86_64, macOS AArch64;
- pregenerated-binding freshness and C/C++ ABI probes;
- Rust-only conformance on every platform;
- cross-language conformance on Linux, Windows, and macOS;
- CLAP on Linux/macOS/Windows, VST3 with the pinned SDK shim, and Audio Unit v2 on macOS.

Miri, sanitizers, fuzz smoke runs, dependency/license audit, and minimum-version resolution run on scheduled or release jobs if runtime makes them unsuitable for every PR. Feature CI exhausts zero/one-feature cases plus `plugin+host`, each companion with its required runtime, both published bundles, and pairwise combinations selected from changed dependency edges; it does not promise an unbounded power set. Release branches require their most recent successful result on the release commit.

## Coverage and review gates

Coverage is measured per unsafe module and interface group, not optimized as a global vanity percentage. Every unsafe branch and failure sentinel must be exercised. Every public safe method requires at least one behavioral test or a documented delegation to an already tested generic mechanism.

Release review includes: ABI diff against 2.3, unsafe-code review, dependency/license review, public API review, docs/manual readiness, and a clean full conformance matrix. Known deviations require a documented waiver in the overview; “not yet implemented” is not a valid waiver for a full-support release.

## Acceptance criteria

All manifests join without gaps; upstream and bridge-specific scenarios pass in the required pairings; Miri/sanitizer findings are zero; no realtime prohibition is observed; fuzz smoke tests are clean; and CI can reproduce generated artifacts and cross-language probes from a clean checkout.

## Decisions and revisions

- 2026-07-14: API completeness is enforced by joining generated ABI, delegate, and test manifests.
- 2026-07-14: Upstream scenarios are named release gates, not optional examples.
- 2026-07-14: Audit requires capability-rich fixtures and non-skipped positive coverage for every optional surface.
- 2026-07-15: Audit requires TSan against production synchronization paths in addition to abstract state models.
- 2026-07-15: Direct Rust/C++ pairing runs the ten upstream scenarios that require only an ARA factory. Rendering/editor scenarios remain companion-suite gates, while chunk loading remains a decoder-only gate; neither category is recorded as a runtime capability skip.
