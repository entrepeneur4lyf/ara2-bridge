# ARA2 Bridge Companion Adapters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement complete redistributable ARA companion integration for CLAP, VST3, and Audio Unit v2 on both plug-in and host paths, while keeping companion audio processing explicitly external.

**Architecture:** `CompanionProcessorBinding` connects an externally owned processor to shared plug-in/host runtime state. CLAP uses audited direct Rust declarations; VST3 and AUv2 use narrow C++/Objective-C++ shims around pinned SDKs. Every adapter shares factory pointers, enforces one-shot pre-activation binding, and preserves both teardown orders.

**Tech Stack:** Rust, C11/C++17, CLAP 1.1.9 commit `094bb76c85366a13cc6c49292226d8608d6ae50c`, MIT-licensed VST3 SDK `v3.8.0_build_66` commit `9fad9770f2ae8542ab1a548a68c1ad1ac690abe0`, AudioUnitSDK `AudioUnitSDK-1.0.0`, platform Core Audio.

---

Read first: specs `01`, `02`, `03`, `04`, `06`, `07`, `08` and handoffs `phase-0-abi.md` through `phase-4-host.md` under `docs/superpowers/handoffs/`.

### Task 0: Provision and preflight companion SDK inputs

Use the Phase 0 lock/bootstrap boundary before any feature-gated build. Run portably: `ci/bootstrap-reference-sdks.sh fetch --component clap --accept-license MIT && ci/bootstrap-reference-sdks.sh check --component clap`. Run configured VST3 jobs with the locked MIT identity: `ci/bootstrap-reference-sdks.sh fetch --component vst3 --accept-license MIT && ci/bootstrap-reference-sdks.sh check --component vst3`. Run on macOS: `ci/bootstrap-reference-sdks.sh fetch --component audio-unit --accept-license Apache-2.0 && ci/bootstrap-reference-sdks.sh check --component audio-unit`. This initial preflight verifies only the exact commits, recursive identities, license choices, and clean state recorded in `ci/reference-sdks.lock.toml`. Tasks 2, 4, and 6 create the component provenance manifests, hash every transitively consumed source, and require `cargo xtask ara provenance --check --component <name>` before accepting generated output. Missing flags, wrong identities, later hash drift, and dirty checkouts fail before compilation. The script sets or documents the canonical SDK paths under `.third-party/` and never downloads during a package build.

### Task 1: Define the companion-neutral processor boundary

**Files:**
- Create: `ara2-bridge-companion/src/binding.rs`
- Create: `ara2-bridge-companion/src/lifecycle.rs`
- Create: `ara2-bridge-companion/tests/binding.rs`
- Modify: `ara2-bridge-companion/src/lib.rs`
- Modify: `ara2-bridge-companion/Cargo.toml`
- Modify: `ara2-bridge-testkit/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `ara2-bridge-companion/build.rs`

- [x] **Step 1: Write failing one-shot and ordering tests**

```rust
#[test]
fn binding_must_precede_processor_boundaries() {
    let processor = fixture_processor();
    processor.observe(LifecycleEvent::StateLoad).unwrap();
    assert!(processor.bind(controller(), roles()).is_err());
}

#[test]
fn binding_is_one_shot() {
    let processor = fixture_processor();
    processor.bind(controller(), roles()).unwrap();
    assert!(processor.bind(controller(), roles()).is_err());
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-companion --test binding`  
Expected: FAIL on missing binding API.

- [x] **Step 3: Define features/build routing and implement shared state**

Define independent off-by-default `clap`, `vst3`, and `audio-unit-v2` features in the companion crate and matching testkit forwarders before any feature-gated test runs. Add workspace-pinned `cc = "1"` as the companion build dependency and update `Cargo.lock`; create a no-op-by-default `build.rs` that emits feature-specific SDK diagnostics only when a native feature is enabled. Expose stable factory lookup, supported roles, one-shot controller binding, state-load/activation/view/destruction observations, and shared render/model state. Do not own or implement DSP, companion state, or GUI. Keep tombstoned shared state valid when either controller or processor dies first.

- [x] **Step 4: Run lifetime and thread tests**

Run: `cargo test -p ara2-bridge-companion --test binding && cargo miri test -p ara2-bridge-companion --test binding`  
Expected: PASS for every pre-binding boundary, invalid roles, repeated bind, both close orders, and concurrent observation rules.

- [ ] **Step 5: Commit**

```bash
git add -- Cargo.toml Cargo.lock ara2-bridge-companion/Cargo.toml ara2-bridge-companion/build.rs ara2-bridge-companion/src/lib.rs ara2-bridge-companion/src/binding.rs ara2-bridge-companion/src/lifecycle.rs ara2-bridge-companion/tests/binding.rs ara2-bridge-testkit/Cargo.toml
git commit -m "feat(companion): add processor binding boundary"
```

### Task 2: Generate and probe direct CLAP ARA declarations

**Files:**
- Create: `ara2-bridge-companion/src/clap/mod.rs`
- Create: `ara2-bridge-companion/src/clap/sys.rs`
- Modify: `ara2-bridge-companion/src/lib.rs`
- Create: `ara2-bridge-companion/provenance/clap.toml`
- Create: `ara2-bridge-testkit/native/clap_probe.c`
- Create: `ara2-bridge-testkit/tests/clap_abi.rs`
- Modify: `xtask/src/provenance.rs`
- Create: `xtask/src/companion_probe.rs`
- Create: `xtask/tests/companion_probe.rs`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/ara.rs`
- Create: `ara2-bridge-companion/probes/clap-symbols.json`
- Create: `ara2-bridge-companion/probes/clap-x86_64.json`
- Create: `ara2-bridge-companion/probes/clap-aarch64.json`
- Create: `ara2-bridge-companion/probes/clap-i686.json`

- [x] **Step 1: Write failing CLAP constant/layout probes**

Declare the feature-gated `clap` module from `lib.rs`, create a minimal `clap/mod.rs` that exports `sys`, then compare stable and draft extension IDs, structs, function signatures, feature strings, sizes, alignment, and offsets against the pinned CLAP and ARA headers.

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-testkit --test clap_abi --features clap`  
Expected: FAIL until direct declarations and native probes exist.

- [x] **Step 3: Register and red-test the probe command**

Export `xtask::companion_probe`, register the command shell, and add tests for `--help`, an unknown component, and a CLAP probe whose canonical result is still absent.

Run: `cargo test -p xtask --test companion_probe`  
Expected: FAIL on the deliberately unimplemented CLAP probe, not on an unresolved module or command.

- [x] **Step 4: Implement generation, freshness, and ABI probes**

Implement `clap`, `vst3`, and `audio-unit-v2` probe families with non-mutating `--check-all`, runner-local `--emit <envelope> --target <triple>`, and atomic `--import-dir <dir>` modes. Consume only the transitively required CLAP headers from tag 1.1.9/commit `094bb76c85366a13cc6c49292226d8608d6ae50c`. Record repository, tag, commit, license, and SHA-256 for every input. Generate `clap-symbols.json` to close every CLAP/ARA declaration classified as companion-deferred by the core symbol manifest. The three canonical CLAP ABI results are the exact task paths above; emit/import validates target identity plus source/probe/payload hashes and refuses missing, duplicate, or mismatched families. Generated declarations, symbol manifest, and probe results carry and freshness-check shared provenance metadata. User builds compile checked-in declarations and never invoke bindgen, probes, or downloads.

Run on matching runners: `cargo xtask ara companion-probe clap --emit target/companion-probes/clap-x86_64.probe.tar.zst --target x86_64-unknown-linux-gnu`, `cargo xtask ara companion-probe clap --emit target/companion-probes/clap-aarch64.probe.tar.zst --target aarch64-unknown-linux-gnu`, and `cargo xtask ara companion-probe clap --emit target/companion-probes/clap-i686.probe.tar.zst --target i686-pc-windows-msvc`. Collect all three envelopes without renaming into `target/companion-probes/`, then run: `cargo xtask ara companion-probe clap --import-dir target/companion-probes && cargo xtask ara companion-probe clap --check-all && cargo test -p xtask --test companion_probe && cargo xtask ara provenance --check --component clap && cargo test -p ara2-bridge-testkit --test clap_abi --features clap`  
Expected: PASS with zero provenance drift, deterministic canonical artifacts, complete CLAP symbol classification, and exact C/Rust ABI equality.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-companion/src/lib.rs ara2-bridge-companion/src/clap/mod.rs ara2-bridge-companion/src/clap/sys.rs ara2-bridge-companion/provenance/clap.toml ara2-bridge-companion/probes/clap-symbols.json ara2-bridge-companion/probes/clap-x86_64.json ara2-bridge-companion/probes/clap-aarch64.json ara2-bridge-companion/probes/clap-i686.json ara2-bridge-testkit/native/clap_probe.c ara2-bridge-testkit/tests/clap_abi.rs xtask/src/provenance.rs xtask/src/companion_probe.rs xtask/tests/companion_probe.rs xtask/src/lib.rs xtask/src/ara.rs
git commit -m "build(companion): add pinned clap ara declarations"
```

### Task 3: Implement CLAP plug-in and host adapters

**Files:**
- Modify: `ara2-bridge-companion/src/clap/mod.rs`
- Create: `ara2-bridge-companion/src/clap/plugin.rs`
- Create: `ara2-bridge-companion/src/clap/host.rs`
- Create: `ara2-bridge-testkit/tests/clap_interop.rs`

- [x] **Step 1: Write failing multi-plug-in discovery tests**

Create a CLAP fixture with multiple IDs, only a subset ARA-capable. Assert factory count/index/associated ID, identical factory pointers across paths, stable and draft ID lookup, feature declarations, and discovery without processor instantiation.

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-testkit --test clap_interop --features clap`  
Expected: FAIL on missing CLAP adapters.

- [x] **Step 3: Implement plug-in exposure and host discovery**

Implement `CLAP_EXT_ARA_FACTORY` and `CLAP_EXT_ARA_PLUGINEXTENSION` version 2 plus accepted draft IDs. Keep factory indices, IDs, and pointers alive through entry deinit. Enforce ARA binding before activation, state load, processing-related extension use, or GUI creation; map known/assigned roles exactly.

- [x] **Step 4: Run lifecycle, role, and teardown tests**

Run: `cargo test -p ara2-bridge-testkit --test clap_interop --features clap`  
Expected: PASS for discovery, one-shot binding, every role combination, lifecycle rejection, and both teardown orders.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-companion/src/clap/mod.rs ara2-bridge-companion/src/clap/plugin.rs ara2-bridge-companion/src/clap/host.rs ara2-bridge-testkit/tests/clap_interop.rs
git commit -m "feat(companion): add clap ara adapters"
```

### Task 4: Build the pinned VST3 shim and ABI boundary

**Files:**
- Create: `ara2-bridge-companion/native/vst3/ara_vst3_shim.hpp`
- Create: `ara2-bridge-companion/native/vst3/ara_vst3_shim.cpp`
- Create: `ara2-bridge-companion/src/vst3/mod.rs`
- Create: `ara2-bridge-companion/src/vst3/ffi.rs`
- Modify: `ara2-bridge-companion/src/lib.rs`
- Create: `ara2-bridge-companion/provenance/vst3.toml`
- Create: `ara2-bridge-testkit/tests/vst3_abi.rs`
- Create: `ara2-bridge-testkit/fixtures/empty-vst3-sdk/.keep`
- Modify: `ara2-bridge-companion/build.rs`
- Create: `ara2-bridge-companion/probes/vst3-symbols.json`
- Create: `ara2-bridge-companion/probes/vst3-linux-x86_64.json`
- Create: `ara2-bridge-companion/probes/vst3-linux-aarch64.json`
- Create: `ara2-bridge-companion/probes/vst3-windows-x86_64.json`
- Create: `ara2-bridge-companion/probes/vst3-macos-x86_64.json`
- Create: `ara2-bridge-companion/probes/vst3-macos-aarch64.json`

- [x] **Step 1: Write failing IID/layout/ownership probes**

Declare the feature-gated `vst3` module from `lib.rs`, create a minimal `vst3/mod.rs` that exports `ffi`, then cover `IMainFactory`, `IPlugInEntryPoint`, `IPlugInEntryPoint2`, their IIDs/categories, shim result types, COM query/refcount behavior, and C++ exception containment.

- [x] **Step 2: Verify configured failure**

Run: `ARA_VST3_SDK_DIR=$PWD/ara2-bridge-testkit/fixtures/empty-vst3-sdk cargo test -p ara2-bridge-testkit --test vst3_abi --features vst3`  
Expected: FAIL after inspecting that exact empty fixture, naming missing version `v3.8.0_build_66` and `ARA_VST3_SDK_DIR`. A second run with the variable unset produces the same actionable configuration contract.

- [x] **Step 3: Implement a narrow `extern "C"` shim**

Build only when `vst3` is enabled. Validate the configured SDK provenance before compiling. Catch every C++ exception, translate COM ownership explicitly, expose no C++ layout directly to Rust, and provide ABI probes for constants/IIDs used by the adapter. Generate `vst3-symbols.json` for every ARAVST3/shim declaration and emit/import the five exact OS/architecture results above with the same immutable envelope checks as CLAP.

- [x] **Step 4: Run shim probes**

Run these exact emit commands on matching native or system-emulated runners: `cargo xtask ara companion-probe vst3 --emit target/companion-probes/vst3-linux-x86_64.probe.tar.zst --target x86_64-unknown-linux-gnu`, `cargo xtask ara companion-probe vst3 --emit target/companion-probes/vst3-linux-aarch64.probe.tar.zst --target aarch64-unknown-linux-gnu`, `cargo xtask ara companion-probe vst3 --emit target/companion-probes/vst3-windows-x86_64.probe.tar.zst --target x86_64-pc-windows-msvc`, `cargo xtask ara companion-probe vst3 --emit target/companion-probes/vst3-macos-x86_64.probe.tar.zst --target x86_64-apple-darwin`, and `cargo xtask ara companion-probe vst3 --emit target/companion-probes/vst3-macos-aarch64.probe.tar.zst --target aarch64-apple-darwin`, each with `ARA_VST3_SDK_DIR` set to the locked checkout. Collect those five envelopes without renaming into `target/companion-probes/`, then run: `ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo xtask ara companion-probe vst3 --import-dir target/companion-probes && ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo xtask ara companion-probe vst3 --check-all && cargo xtask ara provenance --check --component vst3 && ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo test -p ara2-bridge-testkit --test vst3_abi --features vst3`  
Expected: PASS against exact `v3.8.0_build_66` MIT inputs with complete symbol classification and five deterministic results.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-companion/native/vst3/ara_vst3_shim.hpp ara2-bridge-companion/native/vst3/ara_vst3_shim.cpp ara2-bridge-companion/src/lib.rs ara2-bridge-companion/src/vst3/mod.rs ara2-bridge-companion/src/vst3/ffi.rs ara2-bridge-companion/provenance/vst3.toml ara2-bridge-companion/probes/vst3-symbols.json ara2-bridge-companion/probes/vst3-linux-x86_64.json ara2-bridge-companion/probes/vst3-linux-aarch64.json ara2-bridge-companion/probes/vst3-windows-x86_64.json ara2-bridge-companion/probes/vst3-macos-x86_64.json ara2-bridge-companion/probes/vst3-macos-aarch64.json ara2-bridge-companion/build.rs ara2-bridge-testkit/tests/vst3_abi.rs ara2-bridge-testkit/fixtures/empty-vst3-sdk/.keep
git commit -m "build(companion): add audited vst3 shim"
```

### Task 5: Implement VST3 plug-in and host adapters

**Files:**
- Modify: `ara2-bridge-companion/src/vst3/mod.rs`
- Create: `ara2-bridge-companion/src/vst3/plugin.rs`
- Create: `ara2-bridge-companion/src/vst3/host.rs`
- Create: `ara2-bridge-testkit/tests/vst3_interop.rs`

- [x] **Step 1: Write failing factory/class-match tests**

Test main-factory class category/name, processor `PClassInfo.name`, `ARAFactory::plugInName`, ambiguous duplicate rejection, identical factory pointers, generation-1 and role-aware entry points, COM ownership, and pre-activation binding.

- [x] **Step 2: Verify failure**

Run: `ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo test -p ara2-bridge-testkit --test vst3_interop --features vst3`  
Expected: FAIL on missing adapters.

- [x] **Step 3: Implement reciprocal adapter paths**

Expose/query `IMainFactory`; associate processor and factory classes unambiguously; implement both entry-point generations; enforce role validation and binding before `setActive`, state/process-context setup, or view creation. Share runtime factory/extension state through `CompanionProcessorBinding`.

- [x] **Step 4: Run native interoperability tests**

Run: `ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo test -p ara2-bridge-testkit --test vst3_interop --features vst3`  
Expected: PASS for host and plug-in paths, exception injection, reference counts returning to zero, and both teardown orders.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-companion/src/vst3/mod.rs ara2-bridge-companion/src/vst3/plugin.rs ara2-bridge-companion/src/vst3/host.rs ara2-bridge-testkit/tests/vst3_interop.rs
git commit -m "feat(companion): add vst3 ara adapters"
```

### Task 6: Implement Audio Unit v2 shim and reciprocal adapters

**Files:**
- Create: `ara2-bridge-companion/native/audio_unit/ara_au_shim.mm`
- Create: `ara2-bridge-companion/native/audio_unit/ara_au_shim.h`
- Create: `ara2-bridge-companion/src/audio_unit/mod.rs`
- Create: `ara2-bridge-companion/src/audio_unit/ffi.rs`
- Create: `ara2-bridge-companion/src/audio_unit/plugin.rs`
- Create: `ara2-bridge-companion/src/audio_unit/host.rs`
- Create: `ara2-bridge-companion/provenance/audio-unit.toml`
- Create: `ara2-bridge-testkit/tests/audio_unit_interop.rs`
- Modify: `ara2-bridge-companion/build.rs`
- Modify: `ara2-bridge-companion/src/lib.rs`
- Create: `ara2-bridge-companion/probes/audio-unit-symbols.json`
- Create: `ara2-bridge-companion/probes/audio-unit-macos-x86_64.json`
- Create: `ara2-bridge-companion/probes/audio-unit-macos-aarch64.json`

- [x] **Step 1: Write failing property and magic-preservation tests**

On macOS, declare the feature-gated `audio_unit` module from `ara2-bridge-companion/src/lib.rs`, then test the `ARA` component tag, factory property, both binding properties, global scope/read-only behavior, size negotiation, `kARAAudioUnitMagic` input validation, and unchanged output on failure.

- [x] **Step 2: Verify platform behavior**

Run on macOS: `ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo test -p ara2-bridge-testkit --test audio_unit_interop --features audio-unit-v2`  
Expected: FAIL on missing AUv2 implementation. On non-Apple targets, `cargo check -p ara2-bridge-companion --features audio-unit-v2` must fail with the documented Apple-only message.

- [x] **Step 3: Implement the Apple-only shim and adapters**

Validate `AudioUnitSDK-1.0.0` provenance and use platform Core Audio headers. Implement instance-property discovery, generation-1 and role-aware binding, exact scope/mutability, and pre-initialization/state/preset/view ordering. Preserve factory identity across paths; treat the component tag only as cache discovery. Generate the exact Audio Unit symbol manifest and two architecture result artifacts above through emit/import/check-all.

- [x] **Step 4: Run host/plug-in, role, and teardown tests**

Run on the matching macOS runners: `ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo xtask ara companion-probe audio-unit-v2 --emit target/companion-probes/audio-unit-macos-x86_64.probe.tar.zst --target x86_64-apple-darwin` and `ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo xtask ara companion-probe audio-unit-v2 --emit target/companion-probes/audio-unit-macos-aarch64.probe.tar.zst --target aarch64-apple-darwin`. Collect both envelopes without renaming into `target/companion-probes/`, then run on macOS: `ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo xtask ara companion-probe audio-unit-v2 --import-dir target/companion-probes && ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo xtask ara companion-probe audio-unit-v2 --check-all && cargo xtask ara provenance --check --component audio-unit && ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo test -p ara2-bridge-testkit --test audio_unit_interop --features audio-unit-v2`  
Expected: PASS for every property path, role combination, invalid magic, lifecycle boundary, and both destruction orders.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-companion/native/audio_unit/ara_au_shim.mm ara2-bridge-companion/native/audio_unit/ara_au_shim.h ara2-bridge-companion/src/lib.rs ara2-bridge-companion/src/audio_unit/mod.rs ara2-bridge-companion/src/audio_unit/ffi.rs ara2-bridge-companion/src/audio_unit/plugin.rs ara2-bridge-companion/src/audio_unit/host.rs ara2-bridge-companion/provenance/audio-unit.toml ara2-bridge-companion/probes/audio-unit-symbols.json ara2-bridge-companion/probes/audio-unit-macos-x86_64.json ara2-bridge-companion/probes/audio-unit-macos-aarch64.json ara2-bridge-companion/build.rs ara2-bridge-testkit/tests/audio_unit_interop.rs
git commit -m "feat(companion): add audio unit v2 ara adapters"
```

### Task 7: Gate feature combinations, dependencies, and portable/native integration

**Files:**
- Modify: `ara2-bridge-companion/Cargo.toml`
- Modify: `ara2-bridge/Cargo.toml`
- Create: `ara2-bridge-companion/README.md`
- Create: `ara2-bridge-testkit/tests/companion_features.rs`
- Create: `docs/companion-sdk-setup.md`
- Create: `docs/superpowers/handoffs/phase-5-companions.md`

- [x] **Step 1: Finalize independent feature bundles**

Make `clap`, `vst3`, and `audio-unit-v2` additive. Ensure core ARA code has no companion dependency and enabling one adapter cannot remove APIs. Document explicit SDK variables, versions, licenses, hashes, and no-download build behavior.

- [x] **Step 2: Compile zero/one/bundle combinations**

Run portably: `cargo test -p ara2-bridge-testkit --test companion_features && cargo check -p ara2-bridge-companion --no-default-features && cargo check -p ara2-bridge-companion --features clap`  
Run with VST3 provisioned: `ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo check -p ara2-bridge --features full-portable`  
Run on macOS: `ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo check -p ara2-bridge --features full-apple`  
Expected: PASS; a separate non-Apple compile-fail fixture proves `audio-unit-v2` emits the documented Apple-only error.

- [x] **Step 3: Run the companion phase gate**

Run portably: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p ara2-bridge-companion --all-targets --features clap -- -D warnings && cargo test --workspace && cargo test -p ara2-bridge-companion --features clap && cargo test -p ara2-bridge-testkit --features clap --test clap_abi --test clap_interop && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps && RUSTDOCFLAGS="-D warnings" cargo doc -p ara2-bridge-companion --features clap --no-deps && cargo xtask ara companion-probe clap --check-all && cargo xtask ara provenance --check`  
Run with VST3 provisioned: `ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo clippy -p ara2-bridge-companion --all-targets --features vst3 -- -D warnings && ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk RUSTDOCFLAGS="-D warnings" cargo doc -p ara2-bridge-companion --features vst3 --no-deps && ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo xtask ara companion-probe vst3 --check-all && ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo test -p ara2-bridge-testkit --features vst3 --test vst3_abi --test vst3_interop`  
Run on macOS: `ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo clippy -p ara2-bridge-companion --all-targets --features audio-unit-v2 -- -D warnings && ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK RUSTDOCFLAGS="-D warnings" cargo doc -p ara2-bridge-companion --features audio-unit-v2 --no-deps && ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo xtask ara companion-probe audio-unit-v2 --check-all && ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo test -p ara2-bridge-testkit --features audio-unit-v2 --test audio_unit_interop`  
Expected: PASS portably and on each configured native job.

- [x] **Step 4: Write the compact phase handoff**

Record feature forwarding, SDK variables/versions, probe outputs, native gate commands/results, lifecycle boundaries, and normative revisions already committed in this phase. The gate fails if any discovered normative revision remains pending.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-companion/Cargo.toml ara2-bridge-companion/README.md ara2-bridge/Cargo.toml ara2-bridge-testkit/tests/companion_features.rs docs/companion-sdk-setup.md docs/superpowers/handoffs/phase-5-companions.md
git commit -m "test(companion): gate companion integrations"
```
