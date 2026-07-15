# ARA2 Bridge Plug-in Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a complete safe Rust ARA plug-in runtime covering factory initialization, all 54 document-controller callbacks, host clients, ARA1/ARA2 graph compatibility, persistence, content, and extension roles.

**Architecture:** `PluginBuilder` composes focused semantic capability traits while the runtime owns ABI instances, vtables, typed registries, lifecycle, and host wrappers. Generated callback shims recover validated runtime state and delegate to handwritten trait groups. Each `ARAFactory` has an independent `PluginEntry`; extension instances share tombstoned lifetime state with their bound controller.

**Tech Stack:** `ara2-bridge-sys`, `ara2-bridge-core`, Rust trait composition, generated compatibility metadata, Miri/sanitizers, mock host fixtures.

---

Read first: specs `02`, `03`, `05`, `06` common contract, `07`, `09` and handoffs `phase-0-abi.md`, `phase-1-core.md`, and `phase-2-content.md` under `docs/superpowers/handoffs/`.

### Task 1: Build immutable factories and independent entries

**Files:**
- Create: `ara2-bridge-plugin/src/factory.rs`
- Create: `ara2-bridge-plugin/src/entry.rs`
- Create: `ara2-bridge-plugin/src/builder.rs`
- Create: `ara2-bridge-plugin/tests/factory.rs`
- Modify: `ara2-bridge-plugin/src/lib.rs`

- [x] **Step 1: Write failing per-factory initialization tests**

```rust
#[test]
fn each_factory_entry_has_independent_generation_state() {
    let registry = PluginRegistry::builder()
        .factory(factory("one", ApiGeneration::V1Final..=ApiGeneration::V23Final))
        .factory(factory("two", ApiGeneration::V2Final..=ApiGeneration::V23Final))
        .build().unwrap();
    registry.entry("one").initialize(ApiGeneration::V1Final, assert_addr()).unwrap();
    registry.entry("two").initialize(ApiGeneration::V23Final, assert_addr()).unwrap();
    assert_eq!(registry.entry("one").generation(), Some(ApiGeneration::V1Final));
    assert_eq!(registry.entry("two").generation(), Some(ApiGeneration::V23Final));
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-plugin --test factory`  
Expected: FAIL on missing registry/factory APIs.

- [x] **Step 3: Implement owned factory backing and callbacks**

```rust
pub struct FactoryBuilder {
    factory_id: PersistentId,
    archive_id: PersistentId,
    compatible_archive_ids: Vec<PersistentId>,
    generations: RangeInclusive<ApiGeneration>,
    capabilities: FactoryCapabilities,
    create: Arc<dyn CreateDocumentController>,
}
```

Pin all strings/arrays/factory data for binary lifetime. Validate unique IDs, target generation availability, capability subsets, and factory-ID changes. Install non-null initialize/uninitialize/create callbacks and return stable factory pointers.

- [x] **Step 4: Run lifecycle and invalid-configuration tests**

Run: `cargo test -p ara2-bridge-plugin --test factory`  
Expected: PASS for two simultaneous generations, reinitialize-after-uninitialize, dirty reference input, and stable pointer equality.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-plugin/src/lib.rs ara2-bridge-plugin/src/factory.rs ara2-bridge-plugin/src/entry.rs ara2-bridge-plugin/src/builder.rs ara2-bridge-plugin/tests/factory.rs
git commit -m "feat(plugin): add ara factory entries"
```

### Task 2: Implement host service clients with exact call scopes

**Files:**
- Create: `ara2-bridge-plugin/src/host/mod.rs`
- Create: `ara2-bridge-plugin/src/host/audio.rs`
- Create: `ara2-bridge-plugin/src/host/archive.rs`
- Create: `ara2-bridge-plugin/src/host/content.rs`
- Create: `ara2-bridge-plugin/src/host/model_update.rs`
- Create: `ara2-bridge-plugin/src/host/playback.rs`
- Create: `ara2-bridge-plugin/tests/host_clients.rs`
- Modify: `ara2-bridge-plugin/src/lib.rs`

- [x] **Step 1: Write failing content/audio scope tests**

```rust
#[test]
fn content_reader_cannot_be_created_outside_callback_scope() {
    let host = MockHost::with_content();
    let clients = HostClients::new(host.instance(), ApiGeneration::V23Final).unwrap();
    assert!(clients.content().is_none());
    clients.with_audio_source_update(source_host_ref(), |scope| {
        assert!(scope.content().audio_source::<Tempo>(source_host_ref(), None).is_ok());
        assert!(scope.content().audio_source::<Tempo>(other_source_host_ref(), None).is_err());
    });
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-plugin --test host_clients`  
Expected: FAIL on missing host clients.

- [x] **Step 3: Implement validated clients**

Validate required audio/archiving interfaces and 2.0 Final `getDocumentArchiveID`; represent optional interfaces by `Option`. Create `HostContentScope<'call>` only during relevant current-object callbacks or `endEditing`. Audio-reader creation uses the same gate but returns a long-lived reader; reads enforce one caller per reader and non-realtime threads. Archive clients exist only during archive calls.

- [x] **Step 4: Test absent, truncated, and failure-reporting host services**

Run: `cargo test -p ara2-bridge-plugin --test host_clients`  
Expected: PASS for absent optional services, truncated model-update tails, wrong current object, reader teardown, host-reported failures, and archive-ID requirements. A host panic cannot be caught after entering a non-unwinding C callback; foreign peers must report failure through the ARA return contract instead.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-plugin/src/lib.rs ara2-bridge-plugin/src/host/mod.rs ara2-bridge-plugin/src/host/audio.rs ara2-bridge-plugin/src/host/archive.rs ara2-bridge-plugin/src/host/content.rs ara2-bridge-plugin/src/host/model_update.rs ara2-bridge-plugin/src/host/playback.rs ara2-bridge-plugin/tests/host_clients.rs
git commit -m "feat(plugin): add scoped ara host clients"
```

### Task 3: Define focused authoring traits and runtime graph

**Files:**
- Create: `ara2-bridge-plugin/src/traits/mod.rs`
- Create: `ara2-bridge-plugin/src/traits/model.rs`
- Create: `ara2-bridge-plugin/src/traits/content.rs`
- Create: `ara2-bridge-plugin/src/traits/persistence.rs`
- Create: `ara2-bridge-plugin/src/model.rs`
- Create: `ara2-bridge-plugin/src/runtime.rs`
- Create: `ara2-bridge-plugin/tests/model_graph.rs`
- Modify: `ara2-bridge-plugin/src/lib.rs`

- [x] **Step 1: Write failing graph-order tests**

```rust
#[test]
fn playback_region_requires_live_modification_and_sequence() {
    let mut runtime = fixture_runtime(ApiGeneration::V23Final);
    let edit = runtime.begin_editing().unwrap();
    let err = edit.create_playback_region(missing_mod(), missing_seq(), props()).unwrap_err();
    assert!(matches!(err, AraError::InvalidArgument(_)));
    assert_eq!(fixture_delegate_calls(), 0);
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-plugin --test model_graph`  
Expected: FAIL on missing runtime.

- [x] **Step 3: Implement capability traits and typed registries**

```rust
pub trait AudioSources: Send {
    type Source: Send + 'static;
    fn create_audio_source(&mut self, ctx: &mut CreateContext<'_>, properties: AudioSourceProperties) -> Result<Self::Source, AraError>;
    fn update_audio_source(&mut self, source: &mut Self::Source, properties: AudioSourceProperties) -> Result<(), AraError>;
}
```

Define separate required traits for document/model object groups and optional traits for analysis, persistence, algorithms, licensing, chunk writing, and signal queries. Runtime registries own application objects, enforce parent/child edges, provision before user hooks, invalidate before destruction hooks, and distinguish deactivation.

- [x] **Step 4: Run graph and provisional-creation tests**

Run: `cargo test -p ara2-bridge-plugin --test model_graph && cargo miri test -p ara2-bridge-plugin --test model_graph`  
Expected: PASS for create/update/deactivate/destroy ordering, stale refs, rollbacks, and leaf-first teardown.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-plugin/src/lib.rs ara2-bridge-plugin/src/traits/mod.rs ara2-bridge-plugin/src/traits/model.rs ara2-bridge-plugin/src/traits/content.rs ara2-bridge-plugin/src/traits/persistence.rs ara2-bridge-plugin/src/model.rs ara2-bridge-plugin/src/runtime.rs ara2-bridge-plugin/tests/model_graph.rs
git commit -m "feat(plugin): add safe ara document model"
```

### Task 4: Generate and wire all 54 document-controller callbacks

**Files:**
- Create: `ara2-bridge-plugin/src/ffi/mod.rs`
- Create: `ara2-bridge-plugin/src/ffi/callbacks.rs`
- Create: `ara2-bridge-plugin/src/ffi/vtable.rs`
- Create: `xtask/src/plugin_dispatch.rs`
- Create: `xtask/tests/plugin_dispatch.rs`
- Create: `ara2-bridge-plugin/tests/callback_manifest.rs`
- Modify: `ara2-bridge-plugin/src/lib.rs`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/main.rs`
- Modify: `xtask/src/ara.rs`

- [x] **Step 1: Register and red-test generation, then write the failing coverage join**

Export `xtask::plugin_dispatch`, register `plugin-dispatch --write|--check`, and test absent output plus a one-byte stale derivative. The command shell must compile before the red run; its deliberate unimplemented result must identify the missing/stale generated file.

```rust
#[test]
fn every_document_controller_slot_has_delegate_and_test_class() {
    for slot in ara2_bridge_sys::compatibility::document_controller_slots() {
        assert!(PLUGIN_DELEGATES.iter().any(|d| d.c_name == slot.name));
        assert!(PLUGIN_CONTRACT_TESTS.iter().any(|t| t.c_name == slot.name));
    }
    assert_eq!(PLUGIN_DELEGATES.len(), 54);
}
```

- [x] **Step 2: Verify missing generation and the 54 missing delegates**

Run: `cargo test -p xtask --test plugin_dispatch`  
Expected: FAIL on the deliberate absent/stale derivative assertion, not an unresolved command or module.  
Run: `cargo test -p ara2-bridge-plugin --test callback_manifest`  
Expected: FAIL listing all 54 unimplemented slots in the new plug-in crate skeleton.

- [x] **Step 3: Generate mechanical shims and prefix constructors**

Implement deterministic `plugin-dispatch --write` and non-mutating `--check` from the compatibility manifest. Each shim only recovers the runtime, validates/copies arguments, selects the scoped context, calls `dispatch_*`, and delegates to a named safe method. `vtable.rs` constructs the generation-required prefix and extends through registered capabilities; every represented callback is non-null and intervening optional groups install count-zero/true/false/defensive-false defaults from the compatibility manifest. Generated shim sources carry and freshness-check the shared source/tag/commit/generator-version/license/`DO NOT EDIT` provenance banner.

```rust
pub static PLUGIN_DELEGATES: &[Delegate] = &[
    Delegate::new("destroyDocumentController", destroy_document_controller),
    // generated in exact manifest order through isAudioModificationPreservingAudioSourceSignal
];
```

- [x] **Step 4: Run generated callback tests**

Run: `cargo xtask ara plugin-dispatch --write && cargo xtask ara plugin-dispatch --check && cargo test -p xtask --test plugin_dispatch && cargo test -p ara2-bridge-plugin --test callback_manifest`  
Expected: PASS with absent/stale-output tests, deterministic regeneration, 54 unique delegates, and correct terminal fields for generations 1–6 on x86/x86_64 and generations 4–6 on AArch64. AArch64 cross-compilation plus runtime construction tests prove generations 1–3 are rejected; shared generation constants remain available so portable code can parse metadata without conditionally compiling enum variants.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-plugin/src/lib.rs ara2-bridge-plugin/src/ffi/mod.rs ara2-bridge-plugin/src/ffi/callbacks.rs ara2-bridge-plugin/src/ffi/vtable.rs ara2-bridge-plugin/tests/callback_manifest.rs xtask/src/plugin_dispatch.rs xtask/tests/plugin_dispatch.rs xtask/src/lib.rs xtask/src/main.rs xtask/src/ara.rs
git commit -m "feat(plugin): dispatch all ara controller callbacks"
```

### Task 5: Implement content, analysis, algorithms, licensing, and persistence behavior

**Files:**
- Create: `ara2-bridge-plugin/src/content.rs`
- Create: `ara2-bridge-plugin/src/analysis.rs`
- Create: `ara2-bridge-plugin/src/persistence.rs`
- Create: `ara2-bridge-plugin/src/processing.rs`
- Create: `ara2-bridge-plugin/tests/capabilities.rs`
- Create: `ara2-bridge-plugin/tests/realtime_head_tail.rs`
- Modify: `ara2-bridge-plugin/src/lib.rs`

- [x] **Step 1: Write failing capability/default tests**

```rust
#[test]
fn later_signal_capability_populates_intervening_defaults() {
    let plugin = PluginBuilder::new(required_model()).signal_preservation(|_| false).build().unwrap();
    let v = plugin.document_controller_interface(ApiGeneration::V23Final);
    assert!(v.getProcessingAlgorithmsCount.is_some());
    assert!(v.isLicensedForCapabilities.is_some());
    assert!(v.storeAudioSourceToAudioFileChunk.is_some());
    assert_eq!(unsafe { (v.getProcessingAlgorithmsCount.unwrap())(plugin.ref_()) }, 0);
}
```

In `tests/realtime_head_tail.rs`, call the unimplemented head/tail adapter through its ABI shell and instrument allocation, blocking-lock acquisition, file I/O, and synchronous logging. The red test must fail because the adapter/snapshot route is absent, before any realtime behavior is implemented.

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-plugin --test capabilities && cargo test -p ara2-bridge-plugin --test realtime_head_tail`  
Expected: FAIL until the optional groups and realtime snapshot adapter are wired.

- [x] **Step 3: Implement semantic capability adapters**

Snapshot typed content readers; cancel analysis on access disable/destruction; order progress; validate algorithm indices/stable properties; gate modal licensing; implement ARA1 whole-document and ARA2 partial persistence with exact call scopes; implement chunk output only when the factory flag and capability agree. Route `getPlaybackRegionHeadAndTailTime` exclusively through `RealtimeHeadTailView`, never through a mutable application trait or general diagnostic path.

- [x] **Step 4: Run capability tests**

Run: `cargo test -p ara2-bridge-plugin --test capabilities && cargo test -p ara2-bridge-plugin --test realtime_head_tail`  
Expected: PASS for each zero/one capability and all later-tail combinations without null callbacks; head/tail queries pass allocation, blocking-lock, file-I/O, and synchronous-log instrumentation, with failures deferred through preallocated diagnostic codes.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-plugin/src/lib.rs ara2-bridge-plugin/src/content.rs ara2-bridge-plugin/src/analysis.rs ara2-bridge-plugin/src/persistence.rs ara2-bridge-plugin/src/processing.rs ara2-bridge-plugin/tests/capabilities.rs ara2-bridge-plugin/tests/realtime_head_tail.rs
git commit -m "feat(plugin): implement ara semantic capabilities"
```

### Task 6: Track and flush reliable 2.3 state notifications

**Files:**
- Create: `ara2-bridge-plugin/src/updates.rs`
- Create: `ara2-bridge-plugin/tests/updates.rs`
- Modify: `ara2-bridge-plugin/src/lib.rs`

- [x] **Step 1: Write failing category/coalescing tests**

```rust
#[test]
fn every_persistent_category_flushes_only_during_notify_model_updates() {
    let mut runtime = fixture_23_runtime();
    runtime.mark_source_changed(source(), None, flags());
    runtime.mark_modification_changed(modification(), None, flags());
    runtime.mark_region_changed(region(), None, flags());
    runtime.mark_document_changed();
    assert!(host_notifications().is_empty());
    runtime.notify_model_updates().unwrap();
    assert_eq!(host_notification_kinds(), ["source", "modification", "region", "document"]);
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-plugin --test updates`  
Expected: FAIL on missing update tracker.

- [x] **Step 3: Implement per-object/category pending state**

Coalesce ranges/flags, suppress host-originated/restoration echoes, retain recovery/conversion changes, flush only inside `notifyModelUpdates`, and retain changes raised during delivery for the next pass. If the interface/tail is absent, keep tracking and suppress the call without lowering factory generation.

- [x] **Step 4: Run full/truncated/absent interface tests**

Run: `cargo test -p ara2-bridge-plugin --test updates`  
Expected: PASS for all categories and reentrant-change behavior.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-plugin/src/lib.rs ara2-bridge-plugin/src/updates.rs ara2-bridge-plugin/tests/updates.rs
git commit -m "feat(plugin): deliver reliable ara 2.3 updates"
```

### Task 7: Implement ARA1 normalization and all extension roles

**Files:**
- Create: `ara2-bridge-plugin/src/compatibility.rs`
- Create: `ara2-bridge-plugin/src/extension/mod.rs`
- Create: `ara2-bridge-plugin/src/extension/playback.rs`
- Create: `ara2-bridge-plugin/src/extension/editor.rs`
- Create: `ara2-bridge-plugin/src/extension/view.rs`
- Create: `ara2-bridge-plugin/tests/extensions.rs`
- Modify: `ara2-bridge-plugin/src/lib.rs`

- [x] **Step 1: Write failing role truth-table tests**

```rust
#[test]
fn role_enablement_matches_sdk_formula() {
    assert_roles(roles(0), roles(0), all_supported(), roles(0b111));
    assert_roles(roles(0b111), roles(0), all_supported(), roles(0));
    assert_roles(roles(0b111), roles(0b101), all_supported(), roles(0b101));
    assert!(bind(roles(0b001), roles(0b010)).is_err());
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-plugin --test extensions`  
Expected: FAIL on missing binding/role interfaces.

- [x] **Step 3: Implement role and lifetime state**

Enable each supported role with `!known(role) || assigned(role)`, reject `assigned & !known`, expose exact ref/interface pairs, and keep interface storage alive across either controller-first or companion-first destruction. Enforce render-state/thread rules on playback/editor assignments and copy view selections.

- [x] **Step 4: Implement ARA1 graph and extension adapters**

Map playback-region musical contexts to internal synthetic sequences, suppress sequence wire callbacks, use dedicated ARA1 restore/store calls, and map legacy set/remove to playback+editor render assignments plus UI-selection semantics.

- [x] **Step 5: Run generation and teardown tests**

Run: `cargo test -p ara2-bridge-plugin --test extensions && cargo miri test -p ara2-bridge-plugin --test extensions`  
Expected: PASS for all role combinations, ARA1 traces, repeated invalid bind, and both destruction orders.

- [ ] **Step 6: Commit**

```bash
git add -- ara2-bridge-plugin/src/lib.rs ara2-bridge-plugin/src/compatibility.rs ara2-bridge-plugin/src/extension/mod.rs ara2-bridge-plugin/src/extension/playback.rs ara2-bridge-plugin/src/extension/editor.rs ara2-bridge-plugin/src/extension/view.rs ara2-bridge-plugin/tests/extensions.rs
git commit -m "feat(plugin): add ara generations and extension roles"
```

### Task 8: Build the capability-rich Rust TestPlugIn and phase gate

**Files:**
- Create: `ara2-bridge-testkit/src/plugin.rs`
- Create: `ara2-bridge-testkit/tests/plugin_contract.rs`
- Modify: `ara2-bridge-testkit/src/lib.rs`
- Modify: `ara2-bridge-plugin/src/lib.rs`
- Create: `ara2-bridge-plugin/README.md`
- Create: `docs/superpowers/handoffs/phase-3-plugin.md`

- [x] **Step 1: Implement a fixture with every optional capability enabled**

The fixture provides all six content kinds, analysis, partial persistence, processing algorithms, licensing, chunk writing, signal preservation, head/tail, and all extension roles. It exposes counters and wire traces but uses only public plug-in APIs. Declare and re-export the fixture from `ara2-bridge-testkit/src/lib.rs` before running its contract test.

- [x] **Step 2: Drive every manifest callback positively and negatively**

Run: `cargo test -p ara2-bridge-testkit --test plugin_contract`  
Expected: PASS with 54 positive callback records and zero capability skips.

- [x] **Step 3: Run the plug-in phase gate**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo test -p ara2-bridge-testkit --test plugin_contract && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps && cargo miri test -p ara2-bridge-plugin --test factory && cargo miri test -p ara2-bridge-plugin --test host_clients && cargo miri test -p ara2-bridge-plugin --test model_graph && cargo miri test -p ara2-bridge-plugin --test callback_manifest && cargo miri test -p ara2-bridge-plugin --test capabilities && cargo miri test -p ara2-bridge-plugin --test realtime_head_tail && cargo miri test -p ara2-bridge-plugin --test updates && cargo miri test -p ara2-bridge-plugin --test extensions`  
Expected: PASS; callback coverage joins the compatibility manifest without gaps.

- [x] **Step 4: Write the compact phase handoff**

Record public builder/trait surfaces, generated dispatch artifacts, supported target/generation sets, gate commands/results, and normative revisions already committed in this phase. The gate fails if any discovered normative revision remains pending.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-plugin/src/lib.rs ara2-bridge-plugin/README.md ara2-bridge-testkit/src/lib.rs ara2-bridge-testkit/src/plugin.rs ara2-bridge-testkit/tests/plugin_contract.rs docs/superpowers/handoffs/phase-3-plugin.md
git commit -m "test(plugin): gate complete ara plugin runtime"
```
