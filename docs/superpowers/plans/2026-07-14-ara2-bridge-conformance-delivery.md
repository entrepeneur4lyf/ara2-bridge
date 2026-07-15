# ARA2 Bridge Conformance and Delivery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove full ARA 2.3 support, cross-language interoperability, safety, packaging independence, and documentation readiness, then prepare the `0.2.0-alpha.1` release surface.

**Architecture:** Machine-readable ABI metadata joins delegate and contract-test manifests. Shared scenarios run through public APIs across Rust/Rust and Rust/C++ pairings. CI layers fast contract tests before Miri, sanitizers, fuzzing, native companion tests, package smoke builds, and manual-source traceability.

**Tech Stack:** Rust testkit, C++17 upstream TestHost/TestPlugIn, Miri, ASan/UBSan/TSan, cargo-fuzz, CI matrices, rustdoc, cargo package.

---

Read first: all specs `00`–`09`, the compatibility manifest, the spec audit report, and handoffs `phase-0-abi.md` through `phase-5-companions.md` under `docs/superpowers/handoffs/`. Do not load prior task narratives unless a handoff links a disputed decision.

### Task 1: Join ABI, delegate, and contract-test manifests

**Files:**
- Create: `ara2-bridge-testkit/src/coverage.rs`
- Create: `ara2-bridge-testkit/tests/coverage_join.rs`
- Modify: `ara2-bridge-testkit/src/lib.rs`
- Create: `docs/conformance/interface-coverage.md`
- Create: `docs/conformance/interface-coverage.json`
- Create: `xtask/src/coverage.rs`
- Create: `xtask/tests/coverage.rs`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/ara.rs`

- [x] **Step 1: Register a minimal compiling report API, then write the exhaustive join with a deterministic freshness failure**

```rust
#[test]
fn every_public_slot_is_delegated_and_classified() {
    let report = CoverageReport::build(
        ara2_bridge_sys::compatibility::all_slots(),
        ara2_bridge_testkit::all_delegates(),
        ara2_bridge_testkit::all_contract_tests(),
    );
    let semantic_gaps = report.semantic_gaps();
    let freshness = std::fs::read_to_string(report.markdown_path())
        .map(|checked_in| checked_in == report.render_markdown())
        .map_err(|error| error.to_string());
    assert!(
        semantic_gaps.is_empty() && freshness == Ok(true),
        "semantic gaps: {semantic_gaps:#?}; report freshness: {freshness:#?}"
    );
}
```

Before the red run, declare `coverage` from `ara2-bridge-testkit/src/lib.rs` and `xtask/src/lib.rs`. Implement the minimal `CoverageReport` constructor/accessors, delegate/test enumeration exports, aggregate `semantic_gaps()`, and deterministic Markdown renderer needed for this test to compile and always attempt `read_to_string`; leave `docs/conformance/interface-coverage.md` absent. Both integration tests aggregate semantic and freshness failures in one diagnostic, so gaps cannot short-circuit the required missing-report red condition. Unresolved imports or methods are not acceptable red results.

- [x] **Step 2: Verify failure**

Run: `cargo test -p xtask --test coverage`  
Expected: FAIL because `docs/conformance/interface-coverage.md` is deliberately absent, not because the command or module is unresolved.  
Run: `cargo test -p ara2-bridge-testkit --test coverage_join`  
Expected: FAIL on the same missing report; if semantic gaps also exist, list them by interface/method/generation in the same failure.

- [x] **Step 3: Implement deterministic coverage reporting**

Join `ara2-bridge-sys/generated/symbol-coverage.json`, the CLAP/VST3/AUv2 companion symbol manifests, factory, every host interface, all 54 controller callbacks, extension roles, targets, and versioned structs. Every header declaration must resolve to a generated Rust symbol, audited shim symbol, or explicit target/SDK exclusion; no companion-deferred classification may remain at release. Require positive, prefix/absence, malformed-input, user/peer failure, panic/exception, lifecycle/thread, and teardown classifications appropriate to each callable signature. Test and implement `cargo xtask ara coverage --write|--check` in `xtask/src/coverage.rs`; generate both the packaged machine-readable JSON and Markdown report from the joined data with shared provenance metadata.

- [x] **Step 4: Close every reported gap and lock freshness**

Run: `cargo xtask ara coverage --write && cargo xtask ara coverage --check && cargo test -p xtask --test coverage && cargo test -p ara2-bridge-testkit --test coverage_join`  
Expected: PASS with zero unclassified public slots and a clean regenerated report.

- [x] **Step 5: Commit**

```bash
git add -- ara2-bridge-testkit/src/lib.rs ara2-bridge-testkit/src/coverage.rs ara2-bridge-testkit/tests/coverage_join.rs docs/conformance/interface-coverage.md docs/conformance/interface-coverage.json xtask/src/coverage.rs xtask/tests/coverage.rs xtask/src/lib.rs xtask/src/ara.rs
git commit -m "test(conformance): join ara interface coverage"
```

### Task 2: Port every upstream TestHost scenario

**Files:**
- Modify: `ara2-bridge-testkit/src/scenarios/mod.rs`
- Create: `ara2-bridge-testkit/src/scenarios/properties.rs`
- Create: `ara2-bridge-testkit/src/scenarios/content.rs`
- Create: `ara2-bridge-testkit/src/scenarios/persistence.rs`
- Create: `ara2-bridge-testkit/src/scenarios/rendering.rs`
- Create: `ara2-bridge-testkit/src/scenarios/extensions.rs`
- Create: `ara2-bridge-testkit/src/scenarios/processing.rs`
- Create: `ara2-bridge-testkit/tests/upstream_scenarios.rs`
- Create: `docs/conformance/upstream-scenarios.toml`
- Modify: `sdk-provenance.toml`
- Create: `ara2-bridge-testkit/fixtures/scenarios/ara1-full.archive`
- Create: `ara2-bridge-testkit/fixtures/scenarios/ara2-full.archive`
- Create: `ara2-bridge-testkit/fixtures/scenarios/ara2-partial-a.archive`
- Create: `ara2-bridge-testkit/fixtures/scenarios/ara2-partial-b.archive`
- Create: `ara2-bridge-testkit/fixtures/scenarios/chunk-wave.wav`
- Create: `ara2-bridge-testkit/fixtures/scenarios/chunk-aiff.aiff`
- Modify: `xtask/src/fixtures.rs`
- Modify: `xtask/tests/fixtures.rs`

- [x] **Step 1: Red-test scenario-fixture freshness and write a failing named-scenario manifest test**

Extend `xtask/tests/fixtures.rs` with absent and one-byte stale cases for the `upstream-scenarios` set. Require exact scenario entries for property/content updates, content reading, modification cloning, full/split archives, drag/import, playback with/without stretch, editor view, algorithms, chunk load, and chunk save. Include bridge-specific ARA1 persistence, 2.3 dirtiness, role combinations, poisoning, and teardown scenarios.

- [x] **Step 2: Verify failure**

Run: `cargo test -p xtask --test fixtures && cargo test -p ara2-bridge-testkit --test upstream_scenarios`  
Expected: fixture test FAILS on the absent/stale scenario outputs and the scenario test lists missing runners/fixtures.

- [x] **Step 3: Implement reusable public-API scenario runners**

Each runner records generation, capability prerequisites, setup, operations, assertions, teardown, expected calls, fixture hashes, and skip policy. The capability-rich Rust fixture must run every applicable scenario with expected skip count zero. Port behavior from the pinned upstream examples without importing their internal implementation architecture.

- [x] **Step 4: Add golden archives and chunk-bearing media**

Extend `cargo xtask ara fixtures --write|--check` with the `upstream-scenarios` set for the six exact paths. Generate them from structured recipes or pinned upstream inputs, atomically record source/license/input/output SHA-256 in the provenance manifest, and reject missing, extra, empty, or stale files. Assert compatible data was actually found and restored.

Run: `cargo xtask ara fixtures --write --set upstream-scenarios && cargo xtask ara fixtures --check --set upstream-scenarios && cargo test -p xtask --test fixtures && cargo xtask ara provenance --check`  
Expected: PASS with deterministic bytes and complete provenance before scenario parity runs.

- [x] **Step 5: Run scenario parity**

Run: `cargo test -p ara2-bridge-testkit --test upstream_scenarios -- --nocapture`  
Expected: PASS with every named scenario executed, zero capability skips, and all allocation/reference counters at zero.

- [x] **Step 6: Commit**

```bash
git add -- ara2-bridge-testkit/src/scenarios/mod.rs ara2-bridge-testkit/src/scenarios/properties.rs ara2-bridge-testkit/src/scenarios/content.rs ara2-bridge-testkit/src/scenarios/persistence.rs ara2-bridge-testkit/src/scenarios/rendering.rs ara2-bridge-testkit/src/scenarios/extensions.rs ara2-bridge-testkit/src/scenarios/processing.rs ara2-bridge-testkit/tests/upstream_scenarios.rs ara2-bridge-testkit/fixtures/scenarios/ara1-full.archive ara2-bridge-testkit/fixtures/scenarios/ara2-full.archive ara2-bridge-testkit/fixtures/scenarios/ara2-partial-a.archive ara2-bridge-testkit/fixtures/scenarios/ara2-partial-b.archive ara2-bridge-testkit/fixtures/scenarios/chunk-wave.wav ara2-bridge-testkit/fixtures/scenarios/chunk-aiff.aiff docs/conformance/upstream-scenarios.toml sdk-provenance.toml xtask/src/fixtures.rs xtask/tests/fixtures.rs
git commit -m "test(conformance): port upstream ara scenarios"
```

### Task 3: Add bidirectional C++ interoperability

**Files:**
- Create: `ara2-bridge-testkit/native/test_host_bridge.cpp`
- Create: `ara2-bridge-testkit/native/test_plugin_bridge.cpp`
- Create: `ara2-bridge-testkit/src/native.rs`
- Create: `ara2-bridge-testkit/tests/cpp_interop.rs`
- Modify: `ara2-bridge-testkit/src/lib.rs`
- Modify: `ara2-bridge-testkit/Cargo.toml`
- Modify: `Cargo.lock`
- Create: `docs/conformance/cpp-interoperability.md`
- Modify: `ara2-bridge-testkit/build.rs`

- [x] **Step 1: Register the gated native boundary and write failing pairing tests**

Add the off-by-default testkit-only `cpp-interop` feature, declare `[[test]] name = "cpp_interop"` with `required-features = ["cpp-interop"]`, and gate the `native` module and every SDK-dependent `build.rs` branch on it. Export a compiling placeholder native boundary that returns `NativeBridgeUnavailable`; the build branch is a no-op until the shim exists. Define two required pairings: Rust TestHost ↔ Celemony C++ TestPlugIn and Celemony C++ TestHost ↔ Rust TestPlugIn. Assert selected generation, scenario name, callback counts, diagnostics, and cleanup counters.

- [x] **Step 2: Verify failure**

Run: `ARA_SDK_DIR=$PWD/reference/ARA_SDK cargo test -p ara2-bridge-testkit --features cpp-interop --test cpp_interop`  
Expected: the test compiles and FAILS with `NativeBridgeUnavailable`, not with an unknown feature or unresolved import.

- [x] **Step 3: Build narrow exception-safe C entry points**

Replace the placeholder with shims compiled against the pinned local SDK provenance, catch all C++ exceptions, and expose scenario/config/result PODs only. Keep upstream objects behind the shim, map logs into structured diagnostics, and ensure portable workspace/package builds leave this path disabled and never require `ARA_SDK_DIR`.

- [x] **Step 4: Run all buildable scenarios in both directions**

Run: `ARA_SDK_DIR=$PWD/reference/ARA_SDK cargo test -p ara2-bridge-testkit --features cpp-interop --test cpp_interop -- --nocapture`  
Expected: PASS on Linux, Windows, and macOS; each unavailable upstream platform scenario has an explicit manifest reason, never a silent skip.

- [x] **Step 5: Commit**

```bash
git add -- Cargo.lock ara2-bridge-testkit/Cargo.toml ara2-bridge-testkit/native/test_host_bridge.cpp ara2-bridge-testkit/native/test_plugin_bridge.cpp ara2-bridge-testkit/src/lib.rs ara2-bridge-testkit/src/native.rs ara2-bridge-testkit/tests/cpp_interop.rs ara2-bridge-testkit/build.rs docs/conformance/cpp-interoperability.md
git commit -m "test(conformance): add cpp ara interoperability"
```

### Task 4: Complete safety, concurrency, and realtime verification

**Files:**
- Modify: `.gitignore`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `fuzz/Cargo.toml`
- Modify: `fuzz/Cargo.lock`
- Create: `fuzz/fuzz_targets/versioned_structs.rs`
- Create: `fuzz/fuzz_targets/references.rs`
- Create: `fuzz/fuzz_targets/content_events.rs`
- Create: `fuzz/fuzz_targets/archive_filters.rs`
- Create: `fuzz/fuzz_targets/audio_file_chunks.rs`
- Create: `fuzz/fuzz_targets/dispatch.rs`
- Create: `fuzz/corpus-manifest.toml`
- Create: `fuzz/corpus/versioned_structs/generation-1.bin`
- Create: `fuzz/corpus/versioned_structs/generation-2.bin`
- Create: `fuzz/corpus/versioned_structs/generation-3.bin`
- Create: `fuzz/corpus/versioned_structs/generation-4.bin`
- Create: `fuzz/corpus/versioned_structs/generation-5.bin`
- Create: `fuzz/corpus/versioned_structs/generation-6.bin`
- Create: `fuzz/corpus/versioned_structs/boundary-prefix.bin`
- Create: `fuzz/corpus/references/null.bin`
- Create: `fuzz/corpus/references/stale.bin`
- Create: `fuzz/corpus/references/foreign-session.bin`
- Create: `fuzz/corpus/content_events/upstream-all-kinds.bin`
- Create: `fuzz/corpus/content_events/boundary-invalid.bin`
- Create: `fuzz/corpus/archive_filters/split-restore.bin`
- Create: `fuzz/corpus/archive_filters/range-overflow.bin`
- Create: `fuzz/corpus/audio_file_chunks/legacy.bin`
- Create: `fuzz/corpus/audio_file_chunks/full-2.3.bin`
- Create: `fuzz/corpus/audio_file_chunks/malformed.bin`
- Create: `fuzz/corpus/dispatch/generation-1.bin`
- Create: `fuzz/corpus/dispatch/generation-6.bin`
- Create: `fuzz/corpus/dispatch/truncated-prefix.bin`
- Create: `fuzz/corpus/dispatch/null-slot.bin`
- Create: `fuzz/corpus/audio_file_xml/namespace-qualified.xml`
- Create: `fuzz/corpus/audio_file_xml/unrelated-ordering.xml`
- Create: `fuzz/corpus/audio_file_xml/multi-entry-order.xml`
- Create: `fuzz/corpus/audio_file_container/wave.bin`
- Create: `fuzz/corpus/audio_file_container/rf64.bin`
- Create: `fuzz/corpus/audio_file_container/bw64.bin`
- Create: `fuzz/corpus/audio_file_container/aiff.bin`
- Create: `fuzz/corpus/audio_file_container/aifc.bin`
- Create: `xtask/src/fuzz_corpus.rs`
- Create: `xtask/tests/fuzz_corpus.rs`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/ara.rs`
- Create: `ara2-bridge-testkit/tests/realtime.rs`
- Create: `ara2-bridge-testkit/tests/analysis_concurrency.rs`
- Create: `ara2-bridge-testkit/tests/sample_access_concurrency.rs`
- Create: `ara2-bridge-testkit/tests/editor_renderer_concurrency.rs`
- Create: `ara2-bridge-testkit/tests/invalid_pointer_subprocess.rs`
- Create: `ara2-bridge-testkit/src/bin/invalid_pointer_case.rs`
- Modify: `ara2-bridge-testkit/Cargo.toml`
- Create: `ara2-bridge-core/tests/state_models.rs`
- Create: `ci/invalid-pointer-ubsan.c`
- Create: `ci/run-sanitizers.sh`
- Create: `docs/conformance/safety.md`

- [x] **Step 1: Add failing instrumentation/model tests**

Instrument allocation, blocking synchronization, file I/O, and logging on designated realtime callbacks. Model reader disable/read, analysis cancellation, editor updates, and controller/companion teardown interleavings deterministically.

- [x] **Step 2: Verify targeted failures**

Run: `cargo test -p ara2-bridge-testkit --test realtime && cargo test -p ara2-bridge-core --test state_models`  
Expected: FAIL until hooks and models are integrated.

- [x] **Step 3: Red-test, generate, and verify fuzz corpora**

Export `xtask::fuzz_corpus`, register `fuzz-corpus --write|--check`, and first run `cargo test -p xtask --test fuzz_corpus`; it must FAIL on a deliberately absent seed and a one-byte stale seed, not an unresolved command. Implement deterministic generation/import for every enumerated path above. `fuzz/corpus-manifest.toml` records target, semantic class, source path/repository, source license, source SHA-256, and output SHA-256; `--check` rejects missing, extra, empty, stale, or unlicensed seeds. Seed every API generation, boundary size, upstream example, golden fixture, and previous regression. Targets call the same production validators/decoders and assert no panic, UB sentinel, unbounded allocation, or illegal accepted state. Preserve the XML/container targets created in phase 2 and copy their named golden inputs deterministically rather than maintaining divergent bytes.

Run: `cargo xtask ara fuzz-corpus --write && cargo xtask ara fuzz-corpus --check && cargo test -p xtask --test fuzz_corpus`  
Expected: PASS with all eight non-empty target corpora, exact hashes, complete licensing/source metadata, and deterministic regeneration.

- [x] **Step 4: Add isolated invalid-pointer sanitizer cases**

Create `ci/run-sanitizers.sh` with explicit `asan-invalid-pointer`, `ubsan-invalid-pointer`, `tsan-state-models`, and `tsan-production` modes. Run unreadable, null-adjacent, and guard-page cases only in child processes compiled/instrumented by the ASan mode. Because rustc nightly does not expose UBSan, the UBSan mode uses a Clang-instrumented C foreign-caller harness for the same pointer-readability contract and separately runs the Rust caller-valid malformed-storage test. For caller-valid storage with malformed contents, assert typed rejection. For genuinely unreadable storage, assert the documented foreign-caller contract and sanitizer report/exit classification without contaminating the test runner; never claim safe Rust can probe arbitrary address readability. `tsan-state-models` runs deterministic models. `tsan-production` runs the three public-API testkit integrations against real analysis workers/cancellation, concurrent audio readers plus access revocation, and editor-renderer assignment/update/teardown paths; both TSan modes use nightly `-Zbuild-std` so the standard library and dependencies share the instrumented ABI. The production lane asserts actual callbacks and cleanup counters so an empty or model-only run fails.

- [x] **Step 5: Run safety gates**

Run the explicit Miri-compatible suites: `cargo miri test -p ara2-bridge-core --test registry && cargo miri test -p ara2-bridge-core --test ffi_validation && cargo miri test -p ara2-bridge-core --test lifecycle && cargo miri test -p ara2-bridge-core --test dispatch && cargo miri test -p ara2-bridge-core --test state_models && cargo miri test -p ara2-bridge-plugin --test model_graph && cargo miri test -p ara2-bridge-plugin --test callback_manifest && cargo miri test -p ara2-bridge-plugin --test extensions && cargo miri test -p ara2-bridge-host --test audio_access && cargo miri test -p ara2-bridge-host --test plugin_dispatch && cargo miri test -p ara2-bridge-host --test document_graph && cargo miri test -p ara2-bridge-host --test restoration && cargo miri test -p ara2-bridge-host --test extensions`. These tests cover validation, lifecycle, dispatch, RAII, concurrency models, and destruction without invoking Cargo/rustc subprocess fixtures under Miri.  
Run the remaining gates: `cargo xtask ara fuzz-corpus --check && ci/run-sanitizers.sh asan-invalid-pointer && ci/run-sanitizers.sh ubsan-invalid-pointer && ci/run-sanitizers.sh tsan-state-models && ci/run-sanitizers.sh tsan-production && cargo +nightly fuzz run versioned_structs -- -max_total_time=30 && cargo +nightly fuzz run references -- -max_total_time=30 && cargo +nightly fuzz run content_events -- -max_total_time=30 && cargo +nightly fuzz run archive_filters -- -max_total_time=30 && cargo +nightly fuzz run audio_file_chunks -- -max_total_time=30 && cargo +nightly fuzz run audio_file_xml -- -max_total_time=30 && cargo +nightly fuzz run audio_file_container -- -max_total_time=30 && cargo +nightly fuzz run dispatch -- -max_total_time=30`  
Expected: PASS with no Miri findings or fuzz crashes; corpus freshness is included in CI evidence, ASan/UBSan child reports and exit codes match every deliberate invalid-pointer classification, and TSan reports no race in state models.

- [x] **Step 6: Commit**

```bash
git add -- .gitignore Cargo.toml Cargo.lock fuzz/Cargo.toml fuzz/Cargo.lock fuzz/fuzz_targets/versioned_structs.rs fuzz/fuzz_targets/references.rs fuzz/fuzz_targets/content_events.rs fuzz/fuzz_targets/archive_filters.rs fuzz/fuzz_targets/audio_file_chunks.rs fuzz/fuzz_targets/dispatch.rs fuzz/corpus-manifest.toml fuzz/corpus/versioned_structs/generation-1.bin fuzz/corpus/versioned_structs/generation-2.bin fuzz/corpus/versioned_structs/generation-3.bin fuzz/corpus/versioned_structs/generation-4.bin fuzz/corpus/versioned_structs/generation-5.bin fuzz/corpus/versioned_structs/generation-6.bin fuzz/corpus/versioned_structs/boundary-prefix.bin fuzz/corpus/references/null.bin fuzz/corpus/references/stale.bin fuzz/corpus/references/foreign-session.bin fuzz/corpus/content_events/upstream-all-kinds.bin fuzz/corpus/content_events/boundary-invalid.bin fuzz/corpus/archive_filters/split-restore.bin fuzz/corpus/archive_filters/range-overflow.bin fuzz/corpus/audio_file_chunks/legacy.bin fuzz/corpus/audio_file_chunks/full-2.3.bin fuzz/corpus/audio_file_chunks/malformed.bin fuzz/corpus/dispatch/generation-1.bin fuzz/corpus/dispatch/generation-6.bin fuzz/corpus/dispatch/truncated-prefix.bin fuzz/corpus/dispatch/null-slot.bin fuzz/corpus/audio_file_xml/namespace-qualified.xml fuzz/corpus/audio_file_xml/unrelated-ordering.xml fuzz/corpus/audio_file_xml/multi-entry-order.xml fuzz/corpus/audio_file_container/wave.bin fuzz/corpus/audio_file_container/rf64.bin fuzz/corpus/audio_file_container/bw64.bin fuzz/corpus/audio_file_container/aiff.bin fuzz/corpus/audio_file_container/aifc.bin xtask/src/fuzz_corpus.rs xtask/tests/fuzz_corpus.rs xtask/src/lib.rs xtask/src/ara.rs ara2-bridge-testkit/Cargo.toml ara2-bridge-testkit/tests/realtime.rs ara2-bridge-testkit/tests/analysis_concurrency.rs ara2-bridge-testkit/tests/sample_access_concurrency.rs ara2-bridge-testkit/tests/editor_renderer_concurrency.rs ara2-bridge-testkit/tests/invalid_pointer_subprocess.rs ara2-bridge-testkit/src/bin/invalid_pointer_case.rs ara2-bridge-core/tests/state_models.rs ci/invalid-pointer-ubsan.c ci/run-sanitizers.sh docs/conformance/safety.md
git commit -m "test(safety): add ara miri fuzz and realtime gates"
```

### Task 5: Install the required CI matrix

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `ara2-bridge-core/src/audio_file/xml.rs`
- Modify: `ara2-bridge-host/src/plugin/mod.rs`
- Create: `deny.toml`
- Modify: `.github/workflows/ci.yml`
- Create: `.github/workflows/native-conformance.yml`
- Create: `.github/workflows/safety.yml`
- Create: `.github/workflows/release.yml`
- Modify: `ci/bootstrap-reference-sdks.sh`
- Verify: `ci/reference-sdks.lock.toml`
- Verify: `ci/run-sanitizers.sh`
- Create: `ci/write-evidence.sh`
- Create: `docs/conformance/ci-matrix.md`
- Create: `docs/conformance/evidence-schema.json`
- Create: `xtask/src/ci.rs`
- Create: `xtask/tests/ci.rs`
- Modify: `xtask/tests/workspace.rs`
- Modify: `xtask/Cargo.toml`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/main.rs`

- [x] **Step 1: Encode fast always-on jobs**

Run format, workspace check, clippy `-D warnings`, tests, rustdoc `-D warnings`, MSRV 1.82, stable, no-default-features, additive feature cases, binding freshness, C/C++ probes, and Rust conformance. Include Linux x86_64 and runtime-conformant Linux AArch64, Windows x86_64, Windows i686 running `cargo test --target i686-pc-windows-msvc -p ara2-bridge-core --test archive archive_larger_than_address_space_is_rejected`, macOS x86_64, and macOS AArch64. Add a workflow regression test proving the existing phase-0 ABI freshness/core-probe jobs remain present.

- [x] **Step 2: Encode native and scheduled gates**

Run cross-language conformance on all desktop OSes with testkit feature `cpp-interop` and `ARA_SDK_DIR`; keep that feature disabled in portable/package jobs. Provision ARA/CLAP/VST3/AUv2 only through the tracked bootstrap/lock file with explicit CI license-policy flags, then run CLAP everywhere, configured VST3 everywhere, and AUv2 on macOS. Schedule Miri, ASan/UBSan/TSan, all eight fuzz smoke targets (including `audio_file_xml` and `audio_file_container`), dependency/license audit, minimum-version resolution, and selected pairwise feature combinations. Pin and hash all installed external SDK inputs. Every gate emits a schema-validated evidence fragment containing repository, exact head SHA, workflow/run/job IDs, target/toolchain, command, conclusion, input hashes, and output hashes. The release workflow combines these only for one SHA into deterministic `ara2-evidence-<sha>.tar.zst`, uploads an artifact named `ara2-evidence-<sha>` containing that archive and its digest, and signs the archive itself with GitHub artifact attestation.

- [x] **Step 3: Register and red-test CI validation commands**

Export `xtask::ci`, register the `ci` command shell, and add integration tests for `--help`, invalid workflow input, missing-job diagnostics, and a canonical-matrix validation that is not implemented yet.

Run: `cargo test -p xtask --test ci`  
Expected: FAIL on the deliberate unimplemented canonical-matrix validation, not on an unresolved module or command.

- [x] **Step 4: Implement workflow parsing and canonical validation**

Implement `xtask/src/ci.rs` to parse the checked-in workflows and canonical matrix, then expose `validate` and `list-jobs`.

- [x] **Step 5: Validate workflows and local command parity**

Run: `cargo test -p xtask --test ci && cargo xtask ci validate && cargo xtask ci list-jobs && cargo +1.82.0 check --workspace --all-targets --locked && cargo deny check licenses sources && cargo audit`  
Run the pinned workflow semantic check: `go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.7 .github/workflows/ci.yml .github/workflows/native-conformance.yml .github/workflows/safety.yml .github/workflows/release.yml`.  
Expected: PASS; the 14 emitted jobs match `docs/conformance/ci-matrix.md`, include Linux AArch64 and i686 runtime tests, preserve phase-0 ABI jobs, require all 40 successful pre-release evidence fragments for one head SHA, emit/attest the release archive, and contain no unpinned Action/download or implicit SDK license acceptance. Advisory and license/source policy must be clean; findings are fixed rather than suppressed.

- [x] **Step 6: Commit**

```bash
git add -- Cargo.toml Cargo.lock ara2-bridge-core/src/audio_file/xml.rs ara2-bridge-host/src/plugin/mod.rs deny.toml .github/workflows/ci.yml .github/workflows/native-conformance.yml .github/workflows/safety.yml .github/workflows/release.yml ci/bootstrap-reference-sdks.sh ci/write-evidence.sh docs/conformance/ci-matrix.md docs/conformance/evidence-schema.json xtask/Cargo.toml xtask/src/ci.rs xtask/tests/ci.rs xtask/tests/workspace.rs xtask/src/lib.rs xtask/src/main.rs
git commit -m "ci: add full ara conformance matrix"
```

### Task 6: Finalize facade features and the 0.1 migration

**Files:**
- Modify: `ara2-bridge/src/lib.rs`
- Modify: `ara2-bridge/Cargo.toml`
- Modify: `Cargo.toml`
- Create: `docs/migration-0.1-to-0.2.md`
- Create: `ara2-bridge/tests/features.rs`
- Create: `ara2-bridge-companion/provenance/vst3.toml`
- Create: `ara2-bridge-companion/probes/vst3-linux-x86_64.json`

- [x] **Step 1: Write failing facade feature tests**

Have `features.rs` launch isolated consumer crates and assert the exact subprocess matrix: default features, no default features, each of `plugin`, `host`, `clap`, `vst3`, `audio-unit-v2`, and `testkit` alone, `plugin,host`, `full-portable`, and Apple-only `full-apple`. Supply required SDK variables per case. On runners without a configured VST3 SDK, assert the exact configuration error instead of silently skipping it; skip only target-inapplicable positive cases and assert the documented non-Apple AUv2 compile error.

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge --test features`  
Expected: FAIL until facade/feature migration is complete.

- [x] **Step 3: Replace the unused monolithic 0.1 surface**

Re-export focused crate builders, traits, handles, errors, and feature modules. Remove contracts that cannot be made sound. Map old `DocumentController`, host traits, runtime construction, and build-time bindgen to new equivalents with before/after compiling examples; add aliases only when they preserve safety.

- [x] **Step 4: Run target-specific feature and MSRV checks**

Run on every runner: `cargo test -p ara2-bridge --test features && cargo +1.82.0 check --workspace --all-targets && cargo check -p ara2-bridge && cargo check -p ara2-bridge --no-default-features && cargo check -p ara2-bridge --no-default-features --features plugin && cargo check -p ara2-bridge --no-default-features --features host && cargo check -p ara2-bridge --no-default-features --features clap && cargo check -p ara2-bridge --no-default-features --features testkit && cargo check -p ara2-bridge --no-default-features --features plugin,host`  
Run on Linux/Windows/macOS with the pinned VST3 SDK: `ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo check -p ara2-bridge --no-default-features --features vst3 && ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo check -p ara2-bridge --features full-portable`  
Run on macOS: `ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo check -p ara2-bridge --no-default-features --features audio-unit-v2 && ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo check -p ara2-bridge --features full-apple`  
Expected: PASS; a separate non-Apple compile-fail test enables only `audio-unit-v2` and matches the documented Apple-only error. The implementation gate is complete when portable checks, a configured native VST3 runner, and the unsupported-platform diagnostics pass; macOS AUv2 and the remaining native VST3 runners retain ownership of their positive release-evidence fragments. Do not infer those platform results from Linux.

- [x] **Step 5: Commit**

```bash
git add -- Cargo.toml ara2-bridge/Cargo.toml ara2-bridge/src/lib.rs ara2-bridge/tests/features.rs docs/migration-0.1-to-0.2.md ara2-bridge-companion/provenance/vst3.toml ara2-bridge-companion/probes/vst3-linux-x86_64.json
git commit -m "feat: finalize ara2 bridge facade and migration"
```

### Task 7: Produce executable documentation and manual-ready sources

**Files:**
- Create: `ara2-bridge/examples/minimal-plugin.rs`
- Create: `ara2-bridge/examples/minimal-host.rs`
- Create: `ara2-bridge/examples/content-reader.rs`
- Create: `ara2-bridge/examples/archive-roundtrip.rs`
- Create: `ara2-bridge/examples/audio-file-chunk.rs`
- Create: `ara2-bridge/examples/clap-binding.rs`
- Create: `ara2-bridge/examples/vst3-binding.rs`
- Create: `ara2-bridge/examples/audio-unit-v2-binding.rs`
- Create: `docs/manual-source-map.md`
- Create: `docs/troubleshooting.md`
- Modify: `docs/specs/ara2-bridge/08-packaging-versioning-and-manual.md`
- Modify: `README.md`
- Modify: `ara2-bridge/Cargo.toml`
- Modify: `ara2-bridge/src/lib.rs`
- Modify: `ara2-bridge-sys/src/lib.rs`
- Modify: `ara2-bridge-core/src/lib.rs`
- Modify: `ara2-bridge-plugin/src/lib.rs`
- Modify: `ara2-bridge-host/src/lib.rs`
- Modify: `ara2-bridge-companion/src/lib.rs`
- Modify: `ara2-bridge-testkit/src/lib.rs`
- Create: `xtask/src/docs.rs`
- Create: `xtask/tests/docs.rs`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/main.rs`

- [x] **Step 1: Add compile-failing documentation checks**

Create rustdoc checks requiring every public item to document generation, ownership/borrowing, state/thread, realtime status, failure behavior, and `# Safety` where unsafe. An item with a direct ARA counterpart must name the upstream C symbol; a bridge-native builder, error, guard, transport, or utility must use the exact `No direct C counterpart` classification plus the ARA behavior it supports. An unambiguous crate/module/type/trait classification covers its child items, while boundary-crossing children override it. Tests reject missing classifications and fabricated C names. Require each crate root to cover role, boundaries, lifecycle, features/platforms, compatibility, licensing, and a compiling example.

- [x] **Step 2: Implement runnable workflow examples**

Use public APIs only. Add exact `[[example]] required-features = [...]` entries in `ara2-bridge/Cargo.toml` so Cargo compiles and packages them. Native companion examples state exact environment variables and are compiled on configured runners. Keep example output deterministic enough for doc tests and manual reproduction.

- [x] **Step 3: Map every manual chapter to durable sources**

For all 12 chapters in spec `08`, name the normative specs, public APIs, runnable examples, conformance commands, exact TestHost arguments, companion binary paths, SDK environment variables, required capabilities, expected skips, fixture hashes, platform registration/cache/signing steps, GUI/main-loop requirements, timeouts, and troubleshooting entries. Encode these as required fields in the manual-source-map schema rather than free-form prose.

- [x] **Step 4: Register and red-test the manual-map verifier**

Export `xtask::docs`, register the command shell, and add integration tests for `--help`, a missing chapter, a missing example, an invalid command reference, and the not-yet-implemented complete manual map. Add one failing fixture for each omitted conformance field: TestHost arguments, companion binary path, SDK environment variables, and required capability set.

Run: `cargo test -p xtask --test docs`  
Expected: FAIL on the incomplete manual map, not on an unresolved module or command.

- [x] **Step 5: Implement the manual-map verifier**

Implement `xtask/src/docs.rs`, then expose `verify-manual-map` with deterministic chapter, example, command-target, fixture-hash, and troubleshooting-anchor validation. Expose `verify-public-docs` for crate-root section/classification checks, fabricated upstream-symbol rejection, and unsafe/missing-doc lint enforcement.

- [x] **Step 6: Run target-specific documentation gates**

Run portably: `cargo test -p xtask --test docs && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps && cargo test --workspace --doc && cargo test -p ara2-bridge --all-targets --features plugin,host,clap,testkit && cargo xtask docs verify-manual-map && cargo xtask docs verify-public-docs`  
Run with VST3: `ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk RUSTDOCFLAGS="-D warnings" cargo doc -p ara2-bridge --features full-portable --no-deps && ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo test -p ara2-bridge --all-targets --features full-portable`  
Run on macOS: `ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK RUSTDOCFLAGS="-D warnings" cargo doc -p ara2-bridge --features full-apple --no-deps && ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo test -p ara2-bridge --all-targets --features full-apple`  
Expected: PASS with no undocumented public items, broken links, noncompiling/package-excluded examples, or unmapped manual chapters. The implementation gate closes after portable and configured VST3 evidence passes; the positive `full-apple` build remains owned by the macOS native runner and must be present in release evidence before conformance is claimed.

- [x] **Step 7: Commit**

```bash
git add -- README.md ara2-bridge/Cargo.toml ara2-bridge/src/lib.rs ara2-bridge/examples/minimal-plugin.rs ara2-bridge/examples/minimal-host.rs ara2-bridge/examples/content-reader.rs ara2-bridge/examples/archive-roundtrip.rs ara2-bridge/examples/audio-file-chunk.rs ara2-bridge/examples/clap-binding.rs ara2-bridge/examples/vst3-binding.rs ara2-bridge/examples/audio-unit-v2-binding.rs docs/manual-source-map.md docs/troubleshooting.md docs/specs/ara2-bridge/08-packaging-versioning-and-manual.md ara2-bridge-sys/src/lib.rs ara2-bridge-core/src/lib.rs ara2-bridge-plugin/src/lib.rs ara2-bridge-host/src/lib.rs ara2-bridge-companion/src/lib.rs ara2-bridge-testkit/src/lib.rs xtask/src/docs.rs xtask/tests/docs.rs xtask/src/lib.rs xtask/src/main.rs
git commit -m "docs: add executable ara manual sources"
```

### Task 8: Verify packages and assemble release evidence

**Files:**
- Create: `docs/releases/0.2.0-alpha.1-conformance.md`
- Create: `docs/releases/0.2.0-alpha.1-checklist.md`
- Create: `docs/releases/source-bundle.toml`
- Create: `CHANGELOG.md`
- Create: `LICENSE-MIT`
- Create: `LICENSES/ARA-SDK-Apache-2.0.txt`
- Create: `LICENSES/third-party.md`
- Modify: `ara2-bridge-sys/Cargo.toml`
- Modify: `ara2-bridge-core/Cargo.toml`
- Modify: `ara2-bridge-plugin/Cargo.toml`
- Modify: `ara2-bridge-host/Cargo.toml`
- Modify: `ara2-bridge-companion/Cargo.toml`
- Modify: `ara2-bridge-testkit/Cargo.toml`
- Modify: `ara2-bridge/Cargo.toml`
- Create: `xtask/src/release.rs`
- Create: `xtask/tests/release.rs`
- Modify: `xtask/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/main.rs`
- Modify: `.github/workflows/release.yml`
- Create: `docs/superpowers/handoffs/phase-6-delivery.md`

- [x] **Step 1: Register and red-test release commands**

Export `xtask::release`, register the command shell, and add integration tests for `--help`, invalid versions, missing/unattested/wrong-SHA evidence, direct-import attestation bypass attempts, wrong subject digest/repository/issuer/workflow identity, package contamination, unsafe-review gaps, license gaps, and missing/extra/stale source-bundle entries. Add negative package fixtures that independently remove or mismatch every generated-derivative provenance field: source repository, tag, commit, generator crate/version, SPDX license, and `DO NOT EDIT`. Leave the clean-room and source-bundle verifiers deliberately unimplemented.

Run: `cargo test -p xtask --test release`  
Expected: FAIL on the deliberate unimplemented clean-room verifier, not on an unresolved module or command.

- [x] **Step 2: Implement release commands and clean-room smoke tests**

Add workspace-pinned `tar = "0.4"` and `zstd = "0.13"`, opt `xtask` into them, and update `Cargo.lock`. Implement `import-evidence`, `verify`, `audit-api`, `audit-unsafe`, `audit-licenses`, `verify-source-inputs`, `source-bundle`, and `verify-source-bundle` in `xtask/src/release.rs`. `import-evidence` itself invokes the configured GitHub/Sigstore verifier, validates and writes a machine-readable receipt under ignored `target/release-evidence/` binding the archive digest to the expected repository, GitHub Actions issuer/workflow identity, and release commit; no prior shell command can substitute for this internal check. Because sibling `0.2.0-alpha.1` crates are not yet in crates.io, the precommit input check vendors the locked registry graph, then packages and inserts each sibling into a Cargo directory source in dependency order before packaging its consumers with `cargo package --allow-dirty --no-verify --locked`. The clean post-commit source-bundle workflow repeats that staged-directory-source process with `cargo package --no-verify --locked` and rejects a dirty tree. The defined vendored clean-room workspace is the mandatory replacement for Cargo's skipped registry-based package verification. Unpack and build there with no `reference/`, clang, network, or undeclared SDK. Check Cargo metadata, README, licenses, dependency versions, and every generated Rust/C/C++/JSON/TOML/Markdown derivative for exact source repository/tag/commit, generator crate/version, SPDX license, and `DO NOT EDIT` metadata.

`docs/releases/source-bundle.toml` is the exact, schema-versioned recipe. `cargo xtask release source-bundle --version 0.2.0-alpha.1 --output target/release-bundles/ara2-bridge-0.2.0-alpha.1-source.tar.zst` must produce a byte-deterministic archive containing: the seven publishable member `.crate` files under `packages/`; those archives unpacked under `clean-room/crates/<name>-<version>/`; `clean-room/Cargo.toml` listing those seven exact directories as workspace members; a generated `clean-room/Cargo.lock`; a versioned `vendor/` source directory containing the seven packaged crates for their normalized registry dependencies and every exact registry dependency from the release lock; a bundle-root `.cargo/config.toml` replacing `crates-io` with `vendor/`; root `Cargo.toml` and `Cargo.lock`; the existing Apache-2.0 project `LICENSE`, project `LICENSE-MIT`, and `LICENSES/**`; `sdk-provenance.toml`; `ara2-bridge-sys/generated/symbol-coverage.json`; every companion provenance and symbol/probe JSON; `docs/conformance/interface-coverage.{json,md}` and release conformance files; all normative specs; `docs/manual-source-map.md`, troubleshooting, migration, and changelog; plus generated `source-bundle.json` metadata and a sorted `MANIFEST.sha256` covering every other entry. Canonicalize each Cargo-produced `.crate` by sorted path, normalized metadata, and deterministic gzip before computing its package digest. Remove cache-specific vendored `.gitignore` files and regenerate directory checksums while retaining published package digests; retain every vendored license and source file. Reject any package not locked by name/version/source/checksum. Archive metadata fixes path order, uid/gid, modes, and timestamps to the candidate commit.

`verify-source-bundle` extracts to a temporary root, sets its current directory to that root so `.cargo/config.toml` is discovered, sets `CARGO_HOME` to a new empty sibling directory, saves and removes `clean-room/Cargo.lock`, and runs `cargo generate-lockfile --manifest-path clean-room/Cargo.toml --offline`. It requires the regenerated lock to be byte-identical to the saved bundle lock and semantically consistent with the root release lock/source-bundle manifest, then runs `cargo build --manifest-path clean-room/Cargo.toml --workspace --offline --locked`. It rejects any missing, extra, duplicated, unhashed, stale, unresolved companion-deferred, unlicensed, ambient-cache-dependent, or non-reproducible entry. No package-local lockfile is assumed. The sys `.crate` itself must include `ara2-bridge-sys/generated/symbol-coverage.json`; workspace-level evidence and notices are carried by this defined source bundle rather than an undefined package set.

Update `.github/workflows/release.yml` to run both source-bundle commands for the candidate SHA and include the exact source archive plus its digest inside `ara2-evidence-<sha>.tar.zst` before that evidence archive is attested. The bundle manifest records the same repository and candidate SHA, so evidence import rejects a source bundle built from another commit.

Run: `cargo test -p xtask --test release`  
Expected: PASS for all synthetic valid/invalid attestation, evidence, audit, package, clean-room lock, and deterministic source-bundle fixtures. Do not run the real release bundle yet because its tracked candidate inputs are created in Step 3.

- [x] **Step 3: Produce and audit the tracked release candidate**

Generate the changelog, licenses, package metadata, release checklist, and a conformance document that names the immutable candidate inputs, required evidence schema, exact commands, known AAX/AUv3 boundaries, and the external attested-artifact location. Run-specific workflow/run/job IDs, sanitizer/fuzz durations, package hashes, receipts, and conclusions remain in the signed evidence artifact under ignored `target/release-evidence/`; the tracked document must not claim a gate ran before the candidate commit exists.

Run after all listed tracked candidate inputs have their final bytes: `cargo xtask release audit-api && cargo xtask release audit-unsafe && cargo xtask release audit-licenses && cargo xtask release verify-source-inputs --version 0.2.0-alpha.1`  
`verify-source-inputs` deliberately invokes `cargo package --allow-dirty --no-verify --locked` for all seven crates into a temporary preflight directory, records that these are non-release test packages, validates the recipe's complete input set and vendorable lock graph, and runs the same custom vendored clean-room verification used after commit without emitting a release artifact or claiming a candidate commit. The post-commit workflow reruns `cargo package --no-verify --locked` from a clean tree and then performs that custom verification again. Expected: PASS with reviewed diffs, every unsafe block linked to a tested invariant, complete redistributable notices, and all final tracked inputs ready for the post-commit workflow.

- [x] **Step 4: Write the final compact handoff**

Record candidate crates/features, evidence requirements, known AAX/AUv3 boundaries, manual source-map location, and every normative revision. State that run-specific package hashes and exact conformance results are published only in the signed evidence artifact for the candidate SHA. This is the starting point for manual authoring and maintenance.

- [ ] **Step 5: Commit the complete release candidate before requesting evidence**

```bash
git add -- Cargo.toml Cargo.lock CHANGELOG.md LICENSE-MIT LICENSES/ARA-SDK-Apache-2.0.txt LICENSES/third-party.md docs/releases/0.2.0-alpha.1-conformance.md docs/releases/0.2.0-alpha.1-checklist.md docs/releases/source-bundle.toml docs/superpowers/handoffs/phase-6-delivery.md ara2-bridge-sys/Cargo.toml ara2-bridge-core/Cargo.toml ara2-bridge-plugin/Cargo.toml ara2-bridge-host/Cargo.toml ara2-bridge-companion/Cargo.toml ara2-bridge-testkit/Cargo.toml ara2-bridge/Cargo.toml xtask/Cargo.toml xtask/src/release.rs xtask/tests/release.rs xtask/src/lib.rs xtask/src/main.rs .github/workflows/release.yml
git commit -m "chore(release): prepare 0.2.0-alpha.1 candidate"
```

- [ ] **Step 6: Run the complete matrix, verify that exact SHA, and tag without further tracked changes**

Push a release-candidate branch whose head is the Step 5 commit and trigger the full release workflow on that branch. The workflow must run `source-bundle` and `verify-source-bundle` only after checkout of that exact clean candidate commit, so archive metadata and `source-bundle.json` bind to an existing immutable SHA. Wait for every required job and the deterministic, archive-level attestation. Set `ARA_RELEASE_RUN_ID` only to that workflow run after confirming its `headSha` equals `COMMIT`.

Run: `COMMIT=$(git rev-parse HEAD) && test -z "$(git status --porcelain)" && gh run view "$ARA_RELEASE_RUN_ID" --json headSha --jq '.headSha' | grep -Fx "$COMMIT" && gh run download "$ARA_RELEASE_RUN_ID" --name "ara2-evidence-$COMMIT" --dir target/release-evidence && cargo xtask release import-evidence --bundle "target/release-evidence/ara2-evidence-$COMMIT.tar.zst" --repository entrepeneur4lyf/ara2-bridge --commit "$COMMIT" && cargo xtask release verify --version 0.2.0-alpha.1 --commit "$COMMIT" && test -z "$(git status --porcelain --untracked-files=no)" && git tag -s v0.2.0-alpha.1 "$COMMIT" -m "ara2-bridge 0.2.0-alpha.1"`  
Expected: PASS only after the attested bundle proves formatting, clippy, tests, rustdoc, manifests, scenarios, Miri/sanitizer/fuzz results, native companion jobs, cross-language pairings, dependency/license audit, MSRV, and package smoke evidence all refer to the committed candidate SHA. The tag points to that same SHA; importing/verifying evidence produces no tracked changes, and any later code or documentation change requires a new candidate commit and a complete matrix rerun.
