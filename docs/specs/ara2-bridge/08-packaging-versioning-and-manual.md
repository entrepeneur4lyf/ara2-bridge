# Packaging, Versioning, and Manual Inputs

Status: Normative delivery specification  
Depends on: [System Overview](00-overview.md), [Conformance and Quality](07-conformance-and-quality.md)  
Last revised: 2026-07-16

## Crates and public surface

The published family is:

- `ara2-bridge-sys`: raw pregenerated ARA 2.3 ABI;
- `ara2-bridge-core`: safe shared types and diagnostics;
- `ara2-bridge-plugin`: plug-in authoring runtime;
- `ara2-bridge-host`: host authoring runtime;
- `ara2-bridge-companion`: feature-gated companion adapters;
- `ara2-bridge-testkit`: conformance and integration support;
- `ara2-bridge`: facade re-exporting the supported authoring surfaces.

Crate boundaries and dependency direction follow the overview. Users may depend on focused crates to minimize compile time. The facade is the recommended starting point and contains migration shims only when they do not weaken safety.

## Features and platforms

The facade's default feature is `plugin`. Additive features are `host`, `clap`, `vst3`, `audio-unit-v2`, and `testkit`. The off-by-default `testkit` feature is an aggregation-only facade dependency; testkit depends on focused crates and never on the facade. `full-portable` enables plug-in, host, CLAP, and VST3. `full-apple` adds Audio Unit v2 and is documented for Apple targets only; there is no ambiguous target-conditional `full` alias. Core ARA support never depends on a companion feature. Explicitly enabling `audio-unit-v2` on a non-Apple target fails with an explanatory compile error.

Features must be additive: enabling one cannot remove APIs or change behavior of another. Public cfg combinations are documented and compile-tested. Third-party SDK locations use explicit environment/Cargo configuration and never network downloads during a build.

## Rust and compatibility policy

Crates retain Rust edition 2021 and initially target MSRV 1.82.0. CI verifies MSRV. A required MSRV increase discovered during implementation must be justified in this spec before merging. Public types are `#[non_exhaustive]` where ARA or bridge evolution requires forward compatibility; exhaustive marker traits are sealed.

Development begins as `0.2.0-alpha.*`. `0.2.0` requires the full release gates in the conformance spec. Criteria for 1.0 are intentionally deferred until 0.2 has real external adoption; this suite makes no unverifiable product-integration promise.

The existing unused `0.1.x` API may be replaced. A migration document maps the old monolithic `DocumentController`, host traits, runtime construction, and build-time bindgen behavior to the new crates/builders/capabilities. Deprecated compatibility aliases are optional and must not retain unsound contracts.

## Documentation requirements

Every public item has rustdoc describing purpose, ARA generation, required thread/state, ownership and borrowing, failure behavior, and realtime status. Items with a direct ARA counterpart name the upstream C symbol; bridge-native builders, errors, guards, transports, and utilities explicitly state `No direct C counterpart` and name the ARA behavior they support. To avoid repetitive rustdoc that obscures the operational contract, an unambiguous crate, module, type, or trait classification applies to its documented child items; a child that crosses that classification boundary must override it explicitly. Unsafe items include a complete `# Safety` contract. Capability traits state whether absence yields a null slot, false result, or default behavior.

Each crate root contains:

- its role in the architecture and dependency boundaries;
- a minimal compiling example;
- lifecycle and threading summary;
- feature/platform table;
- links to related specs and upstream ARA documentation;
- compatibility and licensing notes.

Examples are compiled in CI. Longer examples build a minimal plug-in, minimal host, content provider/reader, archive round trip, audio-file chunk operation, CLAP binding, VST3 binding, and Audio Unit v2 binding. Platform examples are tested on their native runners.

## Manual source plan

The later manual shall be derivable from specs, rustdoc, and executable examples with this structure:

1. ARA concepts and the host/plug-in/model graph.
2. Installation, features, supported targets, and SDK licensing.
3. Plug-in quick start and factory configuration.
4. Document model, editing, analysis, content, and rendering.
5. Persistence, partial archives, and audio-file chunks.
6. Host quick start, plug-in discovery, model graph, and services.
7. CLAP, VST3, and Audio Unit v2 integration guides, plus AAX/AUv3 boundaries.
8. Threading, realtime safety, ownership, and teardown.
9. Errors, assertions, diagnostics, validation, and troubleshooting.
10. Testing with the conformance kit.
11. API-generation compatibility and migration from `0.1.x`.
12. Complete interface and feature reference.

Specs define normative behavior; the manual teaches workflows. Text may be reused, but implementation details and examples must be generated or verified against the released code. Every manual chapter has at least one associated runnable example or explicit reason none applies. The source-map verifier resolves every listed facade API through its focused crate's public module/re-export surface; a syntactically plausible but nonexistent path is a hard failure. Conformance instructions record exact TestHost arguments, companion binary paths, SDK environment variables, audio/chunk fixtures and hashes, required capabilities, platform registration/cache/signing steps, GUI/main-loop needs, timeouts, and expected skip count (zero for the capability-rich release fixture).

## Licensing and provenance

Cargo packages include MIT OR Apache-2.0 project licensing and preserve Celemony's Apache-2.0 notices for headers, generated derivatives, ported utilities, test vectors, fixtures, and example-derived behavior. Companion SDK licenses are listed separately and are not implied by installing this crate. The pinned VST3 SDK baseline is `v3.8.0_build_66` under MIT; the obsolete 3.7 GPL/proprietary policy is not accepted as release provenance. Generated-file headers identify source tag, commit, generator version, and license; the provenance manifest hashes all normative upstream source, not only headers.

## Release artifacts

A release includes crates plus a deterministic source bundle containing the packaged `.crate` files, API docs/manual sources, exhaustive core and companion coverage manifests, conformance report, migration guide, changelog, SDK provenance manifest, and license notices. For a clean release, the tool captures the candidate commit once and materializes every recipe and Cargo package input from an immutable `git archive` snapshot of that exact object; it never copies release bytes from the mutable working tree. Untracked, ignored, or concurrently edited artifacts therefore cannot enter a bundle that claims the captured commit. The release tool canonicalizes Cargo-produced crate containers by sorted path, normalized ownership/mode/time metadata, and deterministic gzip encoding before computing their registry checksums. It also removes cache-specific vendored `.gitignore` files and regenerates each directory checksum while retaining the published package digest; license and source files are never filtered. Verification re-extracts every packaged `.crate`, byte-and-mode compares it with the corresponding clean-room source tree, and builds that verified extraction offline. Evidence bundling accepts only the canonical versioned source-bundle filename after complete inventory and same-commit verification. The bundle has a schema-versioned manifest and SHA-256 inventory and is generated and verified by the release tool.

Release execution is intentionally manual. From a clean, immutable candidate commit, the operator runs the documented local audit, conformance, package, source-bundle, checksum, signing, and publication commands; inspects their outputs; and deliberately publishes each crate in dependency order. The canonical `release audit-api` binding-freshness gate runs on Linux or macOS; Windows validation consumes those checked-in bindings and runs its target-native ABI tests because Windows libclang cannot represent the packed `ARAFactory`. No GitHub Actions workflow or other CI job may construct, attest, sign, upload, or publish a release artifact. CI results may be consulted as additional candidate evidence, but they neither authorize nor perform a release.

## Acceptance criteria

The feature matrix is additive and documented; published packages build without clang or local reference sources; MSRV/stable jobs pass; all examples compile; rustdoc is warning-free; licenses and provenance are complete; the local manual release procedure reproduces and verifies every artifact without CI authority; and every manual chapter can be drafted from an identified spec/API/example source.

## Decisions and revisions

- 2026-07-14: Focused crates plus a facade selected to bound implementation and user context.
- 2026-07-14: Edition 2021 and MSRV 1.82.0 selected initially for audio plug-in ecosystem compatibility.
- 2026-07-14: Full conformance gates `0.2.0`; 1.0 policy is deferred until external adoption supplies evidence.
- 2026-07-14: Audit replaced ambiguous `full`/AUv3 features with explicit portable and Apple-v2 bundles.
- 2026-07-15: Audit clarified the acyclic facade/testkit feature and documentation classification for bridge-native APIs.
- 2026-07-15: Audit defined the deterministic source bundle as the release boundary for workspace-level coverage, provenance, manual sources, and notices that cannot live inside every member crate tarball.
- 2026-07-15: Cross-platform validation requires canonical crate compression and normalized vendored cache metadata so Linux and macOS produce byte-identical source bundles from one commit.
- 2026-07-15: Implementation audit allowed unambiguous enclosing-item C-counterpart classifications so associated methods retain focused operational rustdoc without duplicating classification boilerplate.
- 2026-07-15: VST3 release provenance now requires the MIT-licensed 3.8 SDK baseline and rejects the obsolete 3.7 policy model.
- 2026-07-15: Releases are created and published only through an operator-controlled local procedure; CI is limited to validation and cannot produce release artifacts.
- 2026-07-15: Manual-source validation resolves public API paths and rejects fabricated or stale names before they can reach the future manual.
- 2026-07-15: The manual release procedure names the 40-fragment same-SHA evidence join and all seven ordered package dry-runs as executable pre-tag gates.
- 2026-07-15: Canonical binding freshness and `release audit-api` are Linux/macOS gates; Windows must test the checked-in ABI and explicitly report that its libclang cannot perform canonical generation.
- 2026-07-16: Source bundles copy only commit-owned recipe inputs, prove clean-room sources equal their packaged archives, and must pass same-commit verification before entering an evidence archive.
- 2026-07-16: Clean release bundles materialize one captured commit through `git archive`, eliminating working-tree races between cleanliness validation, recipe copying, and Cargo packaging.
