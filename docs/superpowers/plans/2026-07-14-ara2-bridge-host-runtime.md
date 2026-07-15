# ARA2 Bridge Host Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the complete safe Rust ARA host runtime: all host services, version-aware plug-in dispatch, the host document graph, edit/restore orchestration, extension control, and deterministic teardown.

**Architecture:** `HostServicesBuilder` owns stable ABI interface/reference pairs. `LoadedFactory` and `DocumentController` validate foreign prefixes and dispatch through generated compatibility metadata. `DocumentSession` owns typed host records and plug-in references; scoped edit/restore guards enforce ordering, while extension assignments share the graph without owning it.

**Tech Stack:** `ara2-bridge-sys`, `ara2-bridge-core`, generated compatibility metadata, deterministic mock peers, Miri, sanitizers. The production host crate does not depend on `ara2-bridge-plugin`; Rust plug-in fixtures enter only through testkit dev-dependencies.

---

Read first: specs `02`, `04`, `05`, `07`, `09` and handoffs `phase-0-abi.md` through `phase-3-plugin.md` under `docs/superpowers/handoffs/`.

### Task 1: Build stable host service instances

**Files:**
- Create: `ara2-bridge-host/src/services/mod.rs`
- Create: `ara2-bridge-host/src/services/builder.rs`
- Create: `ara2-bridge-host/src/services/dispatch.rs`
- Create: `ara2-bridge-host/tests/services_builder.rs`
- Modify: `ara2-bridge-host/src/lib.rs`

- [x] **Step 1: Write failing interface-presence and lifetime tests**

```rust
#[test]
fn optional_service_is_absent_instead_of_zeroed() {
    let services = HostServicesBuilder::new(required_audio(), required_archive())
        .build(ApiGeneration::V23Final).unwrap();
    assert!(!services.instance().audioAccessControllerInterface.is_null());
    assert!(services.instance().contentAccessControllerInterface.is_null());
    assert_eq!(services.instance_ptr(), services.instance_ptr());
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-host --test services_builder`  
Expected: FAIL on missing host-service builder.

- [x] **Step 3: Implement owned interface/reference pairs**

Pin one host instance plus each registered state/vtable for the document lifetime. Validate generation-required services and complete prefixes. Represent optional services as null pairs; every callback in an advertised prefix is non-null. Route callbacks through common panic, reference, thread, and poison handling.

- [x] **Step 4: Run malformed-prefix and panic tests**

Run: `cargo test -p ara2-bridge-host --test services_builder`  
Expected: PASS for minimum/full prefixes, optional absence, stable pointers, panic containment, and independent document quarantine.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-host/src/lib.rs ara2-bridge-host/src/services/mod.rs ara2-bridge-host/src/services/builder.rs ara2-bridge-host/src/services/dispatch.rs ara2-bridge-host/tests/services_builder.rs
git commit -m "feat(host): add stable ara host services"
```

### Task 2: Implement audio access with exact sample semantics

**Files:**
- Create: `ara2-bridge-host/src/services/audio.rs`
- Create: `ara2-bridge-host/tests/audio_access.rs`
- Modify: `ara2-bridge-host/src/services/builder.rs`
- Modify: `ara2-bridge-host/src/services/mod.rs`

- [x] **Step 1: Write failing planar-read boundary tests**

```rust
#[test]
fn read_silences_out_of_range_portions() {
    let source = stereo_source_f32(4, &[1.0, 2.0, 3.0, 4.0]);
    let mut out = planar_f32(2, 6, 9.0);
    assert!(source.reader().read(-1, 6, &mut out));
    assert_eq!(out.channel(0), &[0.0, 1.0, 2.0, 3.0, 4.0, 0.0]);
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-host --test audio_access`  
Expected: FAIL on missing reader implementation.

- [x] **Step 3: Implement f32/f64 readers and access state**

Validate source ownership, sample type, channel count, planar pointer arrays, and lengths. Permit one blocking non-realtime caller per reader; allow distinct readers concurrently. Silence before/after source bounds and silence the entire request before returning false on any failure. Disabling access waits for in-flight reads and tears down affected readers.

- [x] **Step 4: Run concurrency and failure tests**

Run: `cargo test -p ara2-bridge-host --test audio_access && cargo miri test -p ara2-bridge-host --test audio_access`  
Expected: PASS for f32/f64, boundaries, bad buffers, concurrent-reader independence, same-reader exclusion, disable synchronization, and failure silence.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-host/src/services/mod.rs ara2-bridge-host/src/services/audio.rs ara2-bridge-host/src/services/builder.rs ara2-bridge-host/tests/audio_access.rs
git commit -m "feat(host): implement ara audio access"
```

### Task 3: Implement archiving, content, updates, and playback services

**Files:**
- Create: `ara2-bridge-host/src/services/archive.rs`
- Create: `ara2-bridge-host/src/services/content.rs`
- Create: `ara2-bridge-host/src/services/model_update.rs`
- Create: `ara2-bridge-host/src/services/playback.rs`
- Create: `ara2-bridge-host/tests/host_callbacks.rs`
- Modify: `ara2-bridge-host/src/services/mod.rs`

- [x] **Step 1: Write failing callback-manifest join**

```rust
#[test]
fn every_host_slot_has_a_dispatcher_and_contract_class() {
    for slot in ara2_bridge_sys::compatibility::host_slots() {
        assert!(HOST_DISPATCHERS.iter().any(|d| d.c_name == slot.name));
        assert!(HOST_CONTRACT_TESTS.iter().any(|t| t.c_name == slot.name));
    }
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-host --test host_callbacks`  
Expected: FAIL listing unimplemented host slots.

- [x] **Step 3: Implement scoped semantic services**

Archiving owns byte transport, progress, and generation-required document archive IDs. Content readers are callback-scoped snapshots using typed content validation. Model updates record all 2.3 categories and tolerate absent/truncated optional interfaces. Playback forwards start/stop/position/cycle commands through a user executor with explicit thread policy.

- [x] **Step 4: Run every callback's positive and negative class**

Run: `cargo test -p ara2-bridge-host --test host_callbacks`  
Expected: PASS for complete/minimum prefixes, malformed references, reader lifetime, progress bounds, missing tails, user errors, and panics.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-host/src/services/mod.rs ara2-bridge-host/src/services/archive.rs ara2-bridge-host/src/services/content.rs ara2-bridge-host/src/services/model_update.rs ara2-bridge-host/src/services/playback.rs ara2-bridge-host/tests/host_callbacks.rs
git commit -m "feat(host): implement ara service callbacks"
```

### Task 4: Load factories and dispatch every plug-in operation

**Files:**
- Create: `ara2-bridge-host/src/plugin/mod.rs`
- Create: `ara2-bridge-host/src/plugin/factory.rs`
- Create: `ara2-bridge-host/src/plugin/controller.rs`
- Create: `ara2-bridge-host/src/plugin/dispatch.rs`
- Create: `xtask/src/host_dispatch.rs`
- Create: `xtask/tests/host_dispatch.rs`
- Create: `ara2-bridge-host/tests/plugin_dispatch.rs`
- Modify: `ara2-bridge-host/src/lib.rs`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/ara.rs`

- [x] **Step 1: Register and red-test generation, then write failing factory and 54-slot coverage tests**

Export `xtask::host_dispatch`, register `host-dispatch --write|--check`, and test absent output plus a one-byte stale derivative. The command shell must compile before the red run; its deliberate unimplemented result must identify the missing/stale generated file.

```rust
#[test]
fn controller_dispatch_matches_compatibility_manifest() {
    let methods = ara2_bridge_host::plugin::dispatch_manifest();
    assert_eq!(methods.len(), 54);
    for slot in ara2_bridge_sys::compatibility::document_controller_slots() {
        assert!(methods.iter().any(|method| method.c_name == slot.name));
    }
}
```

- [x] **Step 2: Verify generator and runtime failures**

Run: `cargo test -p xtask --test host_dispatch`  
Expected: FAIL on the deliberate absent/stale derivative assertion, not an unresolved command or module.  
Run: `cargo test -p ara2-bridge-host --test plugin_dispatch`  
Expected: FAIL on missing loader/dispatch manifest.

- [x] **Step 3: Implement balanced factory loading**

Validate generation range, factory prefix, metadata pointers, compatible archive IDs, capabilities, factory identity, and controller identity. Coordinate initialize/uninitialize per factory entry across every failure path. Reject null callbacks inside represented prefixes; use the compatibility manifest's semantic fallback only for fields outside a shorter peer prefix.

- [x] **Step 4: Generate typed dispatch methods**

Implement deterministic `host-dispatch --write` and non-mutating `--check`; generate only slot metadata and repetitive call shells. Handwritten wrappers validate lifetime, graph ownership, thread, and copied argument backing before crossing FFI. Poison only the affected controller after asserts, exceptions, impossible results, or escaped provisional state. Generated dispatch sources carry and freshness-check the shared source/tag/commit/generator-version/license/`DO NOT EDIT` provenance banner.

- [x] **Step 5: Run generation and malformed-peer tests**

Run: `cargo xtask ara host-dispatch --write && cargo xtask ara host-dispatch --check && cargo test -p xtask --test host_dispatch && cargo test -p ara2-bridge-host --test plugin_dispatch`  
Expected: PASS for absent/stale-output tests, deterministic regeneration, all 54 methods across generations 1–6 on x86/x86_64 and 4–6 on AArch64, shortened prefixes, initialization failures, bad metadata, and balanced teardown; AArch64 compile tests reject legacy generation paths.

- [ ] **Step 6: Commit**

```bash
git add -- ara2-bridge-host/src/lib.rs ara2-bridge-host/src/plugin/mod.rs ara2-bridge-host/src/plugin/factory.rs ara2-bridge-host/src/plugin/controller.rs ara2-bridge-host/src/plugin/dispatch.rs ara2-bridge-host/tests/plugin_dispatch.rs xtask/src/host_dispatch.rs xtask/tests/host_dispatch.rs xtask/src/lib.rs xtask/src/ara.rs
git commit -m "feat(host): add versioned ara plugin dispatch"
```

### Task 5: Implement the host document graph and edit sessions

**Files:**
- Create: `ara2-bridge-host/src/document/mod.rs`
- Create: `ara2-bridge-host/src/document/records.rs`
- Create: `ara2-bridge-host/src/document/edit.rs`
- Create: `ara2-bridge-host/tests/document_graph.rs`
- Modify: `ara2-bridge-host/src/lib.rs`

- [x] **Step 1: Write failing provisional-record tests**

```rust
#[test]
fn synchronous_callback_can_resolve_provisional_source() {
    let mut session = fixture_session_with_reentrant_plugin();
    let mut edit = session.edit().unwrap();
    let source = edit.create_audio_source(source_properties()).unwrap();
    assert_eq!(plugin_observed_host_source(), source.id());
    edit.finish().unwrap();
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-host --test document_graph`  
Expected: FAIL on missing document session.

- [x] **Step 3: Implement stable records and graph validation**

Own document, context, sequence, source, modification, and region records plus their peer refs. Enforce unique persistent IDs and ARA2 edges; normalize ARA1 synthetic sequences internally. Provision records before create calls, commit non-null plug-in refs, roll back clean null returns, and poison when callbacks escape a failed provisional creation.

- [x] **Step 4a: Implement graph commands and ordering**

Begin/end exactly once; expose document and object property updates, content updates, create/clone, sample-access, deactivate/reactivate, and leaf-first destroy in legal order. Keep call-backing strings, arrays, colors, and channel arrangements alive through each call. Translate host model references to plug-in peer references only in the temporary ABI property record. Sample access remains callable outside editing as required by ARA.

- [x] **Step 4b: Complete provisional-escape quarantine and diagnostics**

Resolve provisional host references during synchronous callbacks. Roll back a rejected creation when the reference did not escape; poison the session when it did. `finish()` reports locally observable failures; `Drop` balances best-effort and records diagnostics without claiming foreign rollback.

- [x] **Step 5: Run graph, ARA1, and poison tests**

Run: `cargo test -p ara2-bridge-host --test document_graph && cargo miri test -p ara2-bridge-host --test document_graph`  
Expected: PASS for legal traces, pre-FFI rejection of bad edges/order, provisional rollback/escape, ARA1 normalization, stale refs, and leaf-first close.

- [ ] **Step 6: Commit**

```bash
git add -- ara2-bridge-host/src/lib.rs ara2-bridge-host/src/document/mod.rs ara2-bridge-host/src/document/records.rs ara2-bridge-host/src/document/edit.rs ara2-bridge-host/tests/document_graph.rs
git commit -m "feat(host): add ara document sessions"
```

### Task 6: Implement restoration and explicit close

**Files:**
- Modify: `ara2-bridge-core/src/archive/filter.rs`
- Modify: `ara2-bridge-host/src/document/edit.rs`
- Create: `ara2-bridge-host/tests/restoration.rs`
- Modify: `ara2-bridge-host/src/document/mod.rs`

- [x] **Step 1: Write failing restore/close balance tests**

Create tests for ARA1 dedicated restoration, ARA2 restore-inside-editing, partial recovery, archive-store rejection during editing, and failures at every begin/end boundary. Include named tests `ara2_restore_accepts_multiple_partial_archives_in_one_edit` and `split_restore_applies_graph_before_document_data`; assert exact traces showing one edit guard spanning all archive scopes and graph objects restored before dependent document data.

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-host --test restoration`  
Expected: FAIL on missing restore and close APIs.

- [x] **Step 3: Implement generation-specific restoration guards**

Compose ARA2 restoration with one edit session spanning one or more partial-archive scopes; use ARA1 begin/end restoration callbacks separately. Apply split archives in dependency order: graph objects first, then their document data. Preserve both transport and decode diagnostics. Reject incompatible archive IDs before mutation and support ARA2 object filters without storing while editing.

- [x] **Step 4: Implement explicit leaf-first close**

Remove extension assignments, readers, regions, modifications, sources, sequences, contexts, then controller. Continue guarded cleanup after an individual failure and return aggregated diagnostics. Keep `Drop` as a no-panic fallback.

- [x] **Step 5: Run balance and allocation-counter tests**

Run: `cargo test -p ara2-bridge-host --test restoration && cargo miri test -p ara2-bridge-host --test restoration`  
Expected: PASS with balanced begin/end and initialize/uninitialize counters on every injected failure.

- [ ] **Step 6: Commit**

```bash
git add -- ara2-bridge-host/src/document/mod.rs ara2-bridge-host/src/document/restore.rs ara2-bridge-host/src/document/close.rs ara2-bridge-host/tests/restoration.rs
git commit -m "feat(host): add restoration and deterministic close"
```

### Task 7: Bind and control all extension roles

**Files:**
- Create: `ara2-bridge-host/src/extension/mod.rs`
- Create: `ara2-bridge-host/src/extension/playback.rs`
- Create: `ara2-bridge-host/src/extension/editor.rs`
- Create: `ara2-bridge-host/src/extension/view.rs`
- Create: `ara2-bridge-host/tests/extensions.rs`
- Modify: `ara2-bridge-host/src/lib.rs`

- [x] **Step 1: Write failing role/binding tests**

Test `known`/`assigned` validation, exact returned ref/interface pairs, all role combinations, ARA1 set/remove mapping, selection backing lifetimes, and both companion/controller destruction orders.

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-host --test extensions`  
Expected: FAIL on missing extension controller.

- [x] **Step 3: Implement role wrappers and RAII assignments**

Bind at most once, validate `assigned & !known == 0`, and enable supported roles using `!known(role) || assigned(role)`. Manage playback-region and sequence assignments without owning graph records. Copy view selections and enforce renderer/editor thread and render-state restrictions.

- [x] **Step 4: Run role, concurrency, and teardown tests**

Run: `cargo test -p ara2-bridge-host --test extensions && cargo miri test -p ara2-bridge-host --test extensions`  
Expected: PASS for role truth tables, concurrent editor updates, illegal render mutation, stale graph refs, and both teardown orders.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-host/src/lib.rs ara2-bridge-host/src/extension/mod.rs ara2-bridge-host/src/extension/playback.rs ara2-bridge-host/src/extension/editor.rs ara2-bridge-host/src/extension/view.rs ara2-bridge-host/tests/extensions.rs
git commit -m "feat(host): control ara extension roles"
```

### Task 8: Build the Rust TestHost and phase gate

**Files:**
- Create: `ara2-bridge-testkit/src/host.rs`
- Create: `ara2-bridge-testkit/src/scenarios/mod.rs`
- Create: `ara2-bridge-testkit/src/scenarios/basic.rs`
- Create: `ara2-bridge-testkit/tests/rust_interop.rs`
- Create: `ara2-bridge-host/README.md`
- Modify: `ara2-bridge-testkit/src/lib.rs`
- Modify: `xtask/tests/workspace.rs`
- Create: `docs/superpowers/handoffs/phase-4-host.md`

- [x] **Step 1: Build a public-API-only TestHost**

Provide deterministic audio, content, archive, update, and playback services; trace every call with generation/state/object identity. It must load the capability-rich Rust TestPlugIn without test-only access to either runtime's internals.

- [x] **Step 2: Run the named basic-document smoke scenario**

Run: `cargo test -p ara2-bridge-testkit --test rust_interop`  
Expected: PASS for `basic_document_smoke`: factory initialization, graph construction, requested analysis with ordered analysis-call/progress trace, one edit cycle, sample access, one content reader, one extension assignment, and both close orders. Complete upstream scenario parity is deliberately added by phase 6 under this same `scenarios/` hierarchy.

- [x] **Step 3: Prove dependency direction**

Add a Cargo metadata assertion to `xtask/tests/workspace.rs` that `ara2-bridge-host` has no normal dependency on `ara2-bridge-plugin` or `ara2-bridge-testkit`.

- [x] **Step 4: Run the complete host phase gate**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo test -p xtask --test workspace && cargo test -p ara2-bridge-testkit --test rust_interop && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps && cargo miri test -p ara2-bridge-host --test services_builder && cargo miri test -p ara2-bridge-host --test audio_access && cargo miri test -p ara2-bridge-host --test host_callbacks && cargo miri test -p ara2-bridge-host --test plugin_dispatch && cargo miri test -p ara2-bridge-host --test document_graph && cargo miri test -p ara2-bridge-host --test restoration && cargo miri test -p ara2-bridge-host --test extensions`  
Expected: PASS; dependency direction is proven, all host and plug-in manifests join, the requested-analysis trace is present, and allocation/reference counters return to zero.

- [x] **Step 5: Write the handoff**

Record public host APIs, basic scenario entry point, gate commands/results, and normative revisions already committed in this phase. The gate fails if any discovered normative revision remains pending.

- [ ] **Step 6: Commit**

```bash
git add -- ara2-bridge-host/README.md ara2-bridge-testkit/src/lib.rs ara2-bridge-testkit/src/host.rs ara2-bridge-testkit/src/scenarios/mod.rs ara2-bridge-testkit/src/scenarios/basic.rs ara2-bridge-testkit/tests/rust_interop.rs xtask/tests/workspace.rs docs/superpowers/handoffs/phase-4-host.md
git commit -m "test(host): gate complete ara host runtime"
```
