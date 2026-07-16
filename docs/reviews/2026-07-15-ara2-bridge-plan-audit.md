# ARA2 Bridge Implementation Plan Audit

Date: 2026-07-15  
Scope: all normative files under `docs/specs/ara2-bridge/` and all eight files under `docs/superpowers/plans/`

## Method

An independent read-only reviewer audited the plans repeatedly against the specifications, the current repository state, pinned SDK inputs, clean-checkout execution, exact file staging, dependency and lockfile ownership, target matrices, deterministic generation, safety gates, and release packaging. Each pass rechecked prior findings and searched for regressions; no step was accepted by inference.

## Revisions made

The reviews closed gaps in reproducible SDK bootstrap and licensing, exhaustive core/companion symbol coverage, the synthetic C++-only chunk constant, workspace dependency staging, production-path TSan, deterministic fixtures and fuzz corpora, exact CLAP/VST3/AUv2 probe artifacts, facade/testkit acyclicity, bridge-native rustdoc classification, and workspace-member diagnostics.

Release planning was tightened to define a deterministic, operator-built source bundle with exact notices, coverage/provenance inputs, vendored dependencies, clean-room extraction layout, lock regeneration, fresh `CARGO_HOME`, and offline builds. Dirty preflight packaging and clean candidate packaging now use explicit commands, while candidate-bound artifacts are emitted only after the immutable candidate commit exists.

## Result

The final independent audit returned `CLEAR`. The plan gate is closed and Phase 0 may begin. Implementation evidence that changes behavior, safety invariants, public APIs, scope, or executable sequencing must update the affected specification or plan in the same change.

## Phase 1 implementation re-audit

The 2026-07-15 Phase 1 implementation added the matching channel-layout extent rule to Task 5, pinned MSRV-compatible development dependencies, and expanded the gate with generated ABI-envelope freshness. Phase 2 Task 1 now also owns the generator, scalar type tests, and three refreshed probe envelopes required by its event decoder discovery. These revisions make the original acceptance criteria executable without changing phase order or scope. The focused plan re-audit result is `CLEAR`.

## Phase 2 audio-file implementation re-audit

The 2026-07-15 Task 4 and Task 5 implementation made fixture provenance globally path-sorted so independent fixture sets remain check-order invariant, added nested extension-preservation evidence, exercised a real RF64/BW64 `ds64` table-sized iXML chunk, and included stable, Miri, Clippy, MSRV, freshness, and provenance gates. These changes make the existing hostile-input and large-container acceptance criteria executable without adding scope or weakening an invariant. The focused plan re-audit result is `CLEAR`.

Task 6 corrected its negative half-sample vector from `-1` to the pinned upstream result `0`, then added source hashes and stable/Miri/MSRV evidence for time, tempo, bar, harmony, channel, processing, range, and licensing utilities. This is a test-oracle correction rather than a scope change. The focused plan re-audit result remains `CLEAR`.

## Phase 3 C++ interoperability implementation re-audit

The 2026-07-15 Task 3 implementation gates all SDK-dependent compilation behind the off-by-default `cpp-interop` feature, compiles the exact pinned upstream TestHost and TestPlugIn sources, and exposes only exception-contained POD C entry points. Ten buildable direct-factory scenarios run in both directions with exact generation, scenario, diagnostic, callback, and teardown assertions. Companion-only rendering/editor scenarios and decoder-only chunk loading retain explicit coverage ownership instead of silent skips. Native failures also exposed and corrected two production ABI requirements: analysis-start notification is deferred to `notifyModelUpdates`, and stored audio-file chunks return the factory-published archive ID pointer after value validation. Both are locked into the default Rust contract suite. The focused gate passed Rust scenarios, both native pairings, no-default-features, rustdoc warnings, and Clippy `-D warnings`; the focused plan re-audit result is `CLEAR`.

## Phase 3 safety implementation re-audit

The 2026-07-15 Task 4 implementation makes each safety claim executable at the appropriate boundary. Realtime evidence instruments the production head/tail callback path for zero allocation and audits that exact callback segment for blocking synchronization, file I/O, and synchronous logging. Deterministic state models are paired with production public-API concurrency integrations, and both TSan lanes rebuild the standard library and dependencies with the instrumented ABI. Invalid readable storage is rejected by Rust; deliberately unreadable storage is isolated under ASan, while the UBSan contract is exercised by a Clang-instrumented C foreign caller because rustc does not expose UBSan.

The corpus generator verifies 29 licensed, source-hashed named seeds across all eight targets and distinguishes ignored libFuzzer discoveries from reviewed inputs. Fifty-five explicit Miri tests, all sanitizer modes, all eight 30-second nightly fuzz targets, formatting, Clippy `-D warnings`, corpus freshness, generator tests, and production concurrency tests passed. The plan was revised to record the nightly cargo-fuzz requirement, Clang UBSan harness, TSan `-Zbuild-std`, dependency ownership, and exact staging paths. No safety invariant, acceptance criterion, or scope was weakened; the focused plan re-audit result is `CLEAR`.

## Phase 3 CI and evidence implementation re-audit

The 2026-07-15 Task 5 implementation separates fast portable, desktop-native, and scheduled safety/supply-chain validation workflows. A checked-in canonical matrix validates an exact 13-job set, required platform/command coverage, immutable 40-hex Action revisions, explicit SDK license acceptance, evidence emission/upload, and preservation of the phase-0 generation and probe gates. Evidence fragments use a closed JSON schema and reject unknown fields, non-success conclusions, mixed commits, missing canonical jobs, and missing matrix legs. No release workflow exists; automated checks cannot package, attest, sign, upload, or publish a release.

Execution exposed and closed three issues rather than weakening gates: the Intel runner label was corrected to `macos-15-large`; an unnecessary `const fn` qualifier was removed for Rust 1.82 compatibility; and the advisory/license gates found an unlicensed private xtask plus two high-severity quick-xml 0.37.5 advisories. The xtask now carries the workspace license, cargo-deny enforces only reviewed licenses/sources, and quick-xml 0.41.0 is covered by parser/container tests, MSRV, all fuzz-bin builds, and three renewed 30-second parser fuzz runs. Actionlint, six CI-tool tests, the exact MSRV command, 199 workspace tests, warning-free rustdoc, Clippy, generation/probe/corpus freshness, cargo-audit, and cargo-deny all pass. The focused plan re-audit result is `CLEAR`.

Clean-checkout execution on Linux and macOS then exposed Cargo/cache-dependent `.crate` compression and vendored `.gitignore` differences. Task 8 now canonicalizes crate path order and container metadata before checksumming, removes only cache-specific ignore metadata, regenerates vendor directory checksums, and retains published package digests plus all source/license files. Unit tests cover metadata/order equivalence and vendor normalization; the offline clean-room verifier remains unchanged. This strengthens the original byte-determinism requirement without altering package contents or dependency identity. The focused plan re-audit result is `CLEAR`.

## Phase 3 facade and migration implementation re-audit

The 2026-07-15 Task 6 implementation replaces the unsound, partial 0.1 facade with additive feature-gated re-exports of the focused runtime crates. The default remains plugin authoring; host, testkit, CLAP, VST3, and Audio Unit v2 support are explicit, while `full-portable` and `full-apple` compose only their documented members. No compatibility alias preserves a contract that the focused runtimes cannot uphold. The migration guide maps old construction and host traits to compiling 0.2 equivalents and documents ownership, SDK configuration, and removed APIs.

Feature tests compile isolated consumer crates so workspace feature unification cannot hide missing dependencies. They cover default, empty, singleton, combined, and aggregate feature sets; assert exact VST3 configuration failures when the SDK is absent; and assert the Apple-only Audio Unit diagnostic on non-Apple targets. The locked MIT VST3 3.8 checkout produced a verified dependency-closure manifest, five canonical native probe envelopes, configured VST3/full-portable builds, and six VST3 ABI/interoperability tests on Linux, Windows, and both macOS architectures. Audio Unit v2 provenance, both canonical probes, and three ABI/interoperability tests pass on macOS x86_64 and AArch64. Formatting, Clippy `-D warnings`, workspace tests, warning-free rustdoc, and the Rust 1.82 all-target workspace check pass. The focused plan re-audit result is `CLEAR`.

## Phase 3 executable documentation implementation re-audit

The 2026-07-15 Task 7 implementation turns the future manual outline into checked inputs rather than aspirational prose. Eight facade examples use public APIs only and carry exact required features; the portable plug-in, host, content, archive, chunk, and CLAP workflows execute deterministically, while the VST3 binding also executes against the locked configured SDK. Cargo package inventory includes all eight example sources. Audio Unit v2 remains a macOS-runner positive gate and is not inferred from Linux.

Every safe crate continues to deny missing public docs, unsafe operations, missing safety contracts, and undocumented unsafe blocks; the raw crate now enforces the same unsafe documentation policy. Crate roots cover role/boundaries, lifecycle/threading/ownership/failure, features/platforms, compatibility/licensing, upstream links, and compiling examples. The documentation spec was clarified to let an unambiguous crate/module/type/trait classification cover its children, avoiding repetitive boilerplate without permitting boundary-crossing items to inherit the wrong classification. The public-doc verifier rejects missing root sections, missing classification policy, and fabricated explicit ARA C symbols against the pregenerated symbol inventory.

The embedded 12-chapter TOML manual map requires normative specs, public APIs, examples, real Cargo command targets, exact TestHost configuration, stable example-binary paths, SDK variables, required capabilities, zero-skip expectations, byte-verified fixture hashes, platform steps, GUI/main-loop ownership, timeouts, and resolvable troubleshooting anchors. Twelve positive/negative verifier tests, all crate doctests, warning-free rustdoc, portable and full-portable all-target example/test builds, configured VST3 rustdoc, Clippy `-D warnings`, Rust 1.82, both documentation commands, and package inventory pass. The focused plan re-audit result is `CLEAR`.

## VST3 licensing and manual-release requirements re-audit

The 2026-07-15 requirements correction replaces VST3 SDK 3.7.11 and its GPL/proprietary policy choice with Steinberg's `v3.8.0_build_66` tag at commit `9fad9770f2ae8542ab1a548a68c1ad1ac690abe0`. Steinberg's tagged SDK and developer portal identify 3.8 as MIT and state that the former GPL/proprietary paths are no longer available. The revised companion and packaging specifications therefore require literal `MIT` provenance and reject the obsolete policy variable instead of mislabeling the old checkout.

The release boundary now distinguishes automated validation from release authority. CI may reproduce tests and emit commit-bound validation evidence, but it cannot construct, attest, sign, upload, or publish a release artifact. A clean immutable candidate is packaged, checked, signed, and published only by an operator-controlled local procedure. The overview, component rules, acceptance criteria, and revision records agree on that boundary; searches across all normative specs found no remaining 3.7 policy or CI-release requirement. Both changes are externally verifiable, fail closed on identity drift, preserve the existing conformance bar, and remove unnecessary operator choice. The focused specification re-audit result is `CLEAR`.

The corresponding execution plans now pin VST3 `v3.8.0_build_66`/`9fad9770f2ae8542ab1a548a68c1ad1ac690abe0`, accept only `MIT`, and require fresh provenance plus native probes on every supported platform. The conformance plan retains 13 validation jobs and all 40 commit-bound validation fragments while deleting `release.yml`; a regression gate rejects any future CI release workflow or publication command. Candidate audits, deterministic packaging, offline verification, checksums, dry-runs, signed tagging, and final crate publication are sequenced as explicit local operator actions. No implementation step depends on a GitHub attestation or grants automation release authority. The focused implementation-plan re-audit result is `CLEAR`.

## Final evidence-integrity re-audit

A final independent source review found seven places where a green check could overstate what had been proved. The implementation now records resolvable `path#test_function` callback evidence plus the shared production mechanism, re-executes runner-owned companion probes before comparing all canonical envelope fields, makes `release audit-api` run generation/compatibility/semantic freshness checks, enumerates every workflow YAML file when rejecting release-capable automation, resolves manual-map API paths, and applies Windows destination permissions to the validated temporary before replacement. The operator procedure now executes the 40-fragment same-SHA join and all seven ordered `cargo publish --dry-run --locked` commands before the optional signed tag.

These corrections preserve the MIT VST3 3.8 baseline, validation-only CI boundary, manual publication authority, and AAX/AUv3 exclusions. The corrected source tests and regenerated coverage artifacts pass their focused validators, and an independent read-only source audit found no remaining concrete defect. The final evidence-integrity re-audit result is `CLEAR`; platform evidence remains candidate-SHA-specific and is not inferred from this document.
