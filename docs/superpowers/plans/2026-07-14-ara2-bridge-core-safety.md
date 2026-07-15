# ARA2 Bridge Core Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the shared safe types, validation, ownership, diagnostics, generation, thread, lifecycle, and panic-containment primitives used by both runtimes.

**Architecture:** Keep all raw-pointer access in focused `ffi` modules. Convert caller-valid foreign storage into aligned owned values, represent identities with typed generation-stable handles, and drive legal call sequences with scoped guards/state machines. Dispatch adapters catch panics and map typed errors to method-specific ABI sentinels.

**Tech Stack:** Rust 2021/MSRV 1.82, `thiserror`, `bitflags`, `parking_lot`, `trybuild`, `proptest`, Miri, generated `ara2-bridge-sys` metadata.

---

Read first: specs `00`, `01`, `02`, `05` (typed data rules), `07`, `09`, and `docs/superpowers/handoffs/phase-0-abi.md`.

### Task 1: Define errors and structured diagnostics

**Files:**
- Create: `ara2-bridge-core/src/error.rs`
- Create: `ara2-bridge-core/src/diagnostics.rs`
- Create: `ara2-bridge-core/tests/diagnostics.rs`
- Modify: `ara2-bridge-core/src/lib.rs`
- Modify: `ara2-bridge-core/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

- [x] **Step 1: Add workspace `thiserror = "2"` and write failing error-context tests**

```rust
#[test]
fn diagnostic_retains_interface_method_and_identity() {
    let d = Diagnostic::new(AraError::InvalidState("not editing"))
        .at("ARADocumentControllerInterface", "createAudioSource")
        .with_instance(InstanceId::new(7));
    assert_eq!(d.interface(), Some("ARADocumentControllerInterface"));
    assert_eq!(d.method(), Some("createAudioSource"));
    assert_eq!(d.instance(), Some(InstanceId::new(7)));
}
```

- [x] **Step 2: Verify failure on missing types**

Run: `cargo test -p ara2-bridge-core --test diagnostics`  
Expected: FAIL because `AraError`, `Diagnostic`, and `InstanceId` do not exist.

- [x] **Step 3: Implement the non-exhaustive error and sink APIs**

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AraError {
    #[error("invalid ABI: {0}")] Abi(&'static str),
    #[error("invalid argument: {0}")] InvalidArgument(&'static str),
    #[error("invalid state: {0}")] InvalidState(&'static str),
    #[error("invalid thread: {0}")] InvalidThread(&'static str),
      #[error("unsupported capability: {0}")] Unsupported(&'static str),
      #[error(transparent)] Archive(#[from] ArchiveError),
      #[error(transparent)] Companion(#[from] CompanionError),
      #[error("peer failure: {0}")] Peer(&'static str),
      #[error("instance poisoned")]
      Poisoned,
      #[error("archive too large for target")]
      ArchiveTooLargeForTarget,
  }
```

Define general archive transport/decode/filter and companion discovery/binding/lifecycle error categories now, before later crates depend on them. Add `DiagnosticSink: Send + Sync`, a bounded default ring sink, and static/owned messages without synchronous logging as a correctness dependency.

- [x] **Step 4: Run tests**

Run: `cargo test -p ara2-bridge-core --test diagnostics`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -- Cargo.toml Cargo.lock ara2-bridge-core/Cargo.toml ara2-bridge-core/src/lib.rs ara2-bridge-core/src/error.rs ara2-bridge-core/src/diagnostics.rs ara2-bridge-core/tests/diagnostics.rs
git commit -m "feat(core): add ara errors and diagnostics"
```

### Task 2: Implement API generations and assert-address coordination

**Files:**
- Create: `ara2-bridge-core/src/generation.rs`
- Create: `ara2-bridge-core/src/assertions.rs`
- Create: `ara2-bridge-core/tests/generation.rs`
- Modify: `ara2-bridge-core/src/lib.rs`

- [x] **Step 1: Write failing per-factory generation tests**

```rust
#[test]
fn factories_keep_independent_generations_but_share_generation_cell() {
    let coordinator = AssertCoordinator::default();
    let a = FactoryInitialization::begin(ApiGeneration::V1Final, &coordinator).unwrap();
    let b = FactoryInitialization::begin(ApiGeneration::V23Final, &coordinator).unwrap();
    let c = FactoryInitialization::begin(ApiGeneration::V23Final, &coordinator).unwrap();
    assert_ne!(a.generation(), b.generation());
    assert_eq!(b.assert_address(), c.assert_address());
}
```

- [x] **Step 2: Verify the failure**

Run: `cargo test -p ara2-bridge-core --test generation`  
Expected: FAIL on undefined generation/coordinator types.

- [x] **Step 3: Implement target-aware generation validation**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ApiGeneration { V1Draft = 1, V1Final = 2, V2Draft = 3, V2Final = 4, V2xDraft = 5, V23Final = 6 }

impl ApiGeneration {
    pub fn supported_on_target(self) -> bool {
        cfg!(not(target_arch = "aarch64")) || (self as u32) >= Self::V2Final as u32
    }
}
```

`AssertCoordinator` owns pinned function-pointer cells keyed by generation and reference-counts active initializations. `FactoryInitialization` is non-cloneable, balances one begin/end, and stores generation per factory entry.

- [x] **Step 4: Run tests including duplicate/misordered teardown cases**

Run: `cargo test -p ara2-bridge-core --test generation`  
Expected: PASS; AArch64 cfg tests reject generations 1–3.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-core/src/lib.rs ara2-bridge-core/src/generation.rs ara2-bridge-core/src/assertions.rs ara2-bridge-core/tests/generation.rs
git commit -m "feat(core): coordinate factory generations and assertions"
```

### Task 3: Add typed opaque handles and bounded registries

**Files:**
- Create: `ara2-bridge-core/src/handles.rs`
- Create: `ara2-bridge-core/src/registry.rs`
- Create: `ara2-bridge-core/tests/registry.rs`
- Create: `ara2-bridge-core/tests/ui/handle_not_send.rs`
- Create: `ara2-bridge-core/tests/ui/handle_not_send.stderr`
- Create: `ara2-bridge-core/tests/ui.rs`
- Modify: `ara2-bridge-core/src/lib.rs`
- Modify: `ara2-bridge-core/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

- [x] **Step 1: Add workspace `trybuild = "=1.0.101"` as a dev-dependency and write failing stale/wrong-kind tests**

```rust
enum AudioSourceKind {}
enum PlaybackRegionKind {}

#[test]
fn registry_rejects_stale_and_wrong_kind_handles() {
    let mut registry = Registry::<AudioSourceKind, String>::new(2);
    let handle = registry.insert("source".into()).unwrap();
    assert_eq!(registry.remove(handle).unwrap(), "source");
    assert!(matches!(registry.get(handle), Err(AraError::InvalidArgument(_))));
    let raw = handle.into_raw();
    assert!(Handle::<PlaybackRegionKind>::try_from_raw(raw).is_err());
}
```

`trybuild` is pinned because newer releases pull an Edition 2024 TOML parser that Cargo 1.82
cannot load. Snapshot generation and review use the pinned normalizer.

- [x] **Step 2: Verify failure before implementation**

Run: `cargo test -p ara2-bridge-core --test registry`  
Expected: FAIL on missing `Registry` and `Handle`.

- [x] **Step 3: Implement stable handle cells and tombstones**

```rust
pub struct Handle<K> {
    index: NonZeroU32,
    session: NonZeroU64,
    _kind: PhantomData<fn(K) -> K>,
    _not_send_sync: PhantomData<Rc<()>>,
}
```

Store pinned cells in append-only chunks, never reuse indices within a session, tombstone before invoking user destruction, reject foreign session IDs/kind tags/double removal, and fail insertion at the configurable cap (default 1,048,576).

- [x] **Step 4: Generate, review, and lock the compile-fail snapshot**

Run: `cargo test -p ara2-bridge-core --test registry && TRYBUILD=overwrite cargo test -p ara2-bridge-core --test ui`  
Expected: PASS while creating `handle_not_send.stderr`; review it and require the failure to be the intended `Send`/`Rc<()>` bound with no unrelated diagnostic.  
Run: `cargo test -p ara2-bridge-core --test ui && cargo miri test -p ara2-bridge-core --test registry`  
Expected: PASS with the reviewed snapshot unchanged.

- [ ] **Step 5: Commit**

```bash
git add -- Cargo.toml Cargo.lock ara2-bridge-core/Cargo.toml ara2-bridge-core/src/lib.rs ara2-bridge-core/src/handles.rs ara2-bridge-core/src/registry.rs ara2-bridge-core/tests/registry.rs ara2-bridge-core/tests/ui.rs ara2-bridge-core/tests/ui/handle_not_send.rs ara2-bridge-core/tests/ui/handle_not_send.stderr
git commit -m "feat(core): add typed bounded handle registries"
```

### Task 4: Validate sized structs and foreign arrays safely

**Files:**
- Create: `ara2-bridge-core/src/ffi/mod.rs`
- Create: `ara2-bridge-core/src/ffi/sized.rs`
- Create: `ara2-bridge-core/src/ffi/slice.rs`
- Create: `ara2-bridge-core/src/ffi/string.rs`
- Create: `ara2-bridge-core/src/ffi/scalar.rs`
- Create: `ara2-bridge-core/tests/ffi_validation.rs`
- Modify: `xtask/src/bindings.rs`
- Modify: `xtask/src/compatibility.rs`
- Modify: `ara2-bridge-sys/src/generated/{x86_64,aarch64,i686}.rs`
- Modify: `ara2-bridge-sys/src/generated/layout.rs`
- Create: `ara2-bridge-sys/tests/scalar_constants.rs`
- Modify: `ara2-bridge-core/src/lib.rs`
- Modify: `ara2-bridge-core/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

- [x] **Step 1: Add workspace `proptest = "=1.5.0"` as a dev-dependency and write tests using valid allocations with malformed contents**

```rust
#[test]
fn rejects_partial_tail_without_reading_it() {
    let mut bytes = vec![0u8; size_of::<ARASize>() + 1];
    bytes[..size_of::<ARASize>()].copy_from_slice(&(bytes.len() as ARASize).to_ne_bytes());
    let result = unsafe { SizedInput::<ARAAudioSourceProperties>::from_ptr(bytes.as_ptr().cast()) };
    assert!(matches!(result, Err(AraError::Abi(_))));
}

#[test]
fn rejects_nonzero_count_with_null_pointer() {
    assert!(unsafe { ForeignSlice::<u32>::copy_from_raw(std::ptr::null(), 1) }.is_err());
}

#[test]
fn ara_bool_uses_nonzero_input_and_canonical_output() {
    assert!(!AraBool::from_raw(0).get());
    assert!(AraBool::from_raw(1).get());
    assert!(AraBool::from_raw(2).get());
    assert_eq!(AraBool::new(false).into_raw(), kARAFalse);
    assert_eq!(AraBool::new(true).into_raw(), kARATrue);
}
```

- [x] **Step 2: Run and verify failure**

Run: `cargo test -p ara2-bridge-core --test ffi_validation`  
Expected: FAIL on missing validation types.

- [x] **Step 3: Implement the narrow unsafe boundary**

`SizedInput::from_ptr` documents the caller precondition that advertised storage is readable, reads packed fields with generated unaligned accessors, validates min/complete-field extents, and never attempts OS-level readability probing. `ForeignSlice` checks null/count, `usize` conversion, multiplication/addition overflow, then copies. `ForeignStr` validates NUL termination within configured bounds, UTF-8 for display strings, and seven-bit non-empty ASCII for persistent IDs. `AraBool` is the only safe scalar conversion for `ARABool`: every nonzero inbound value becomes true and outbound values are exactly `kARAFalse` or `kARATrue`. Callback/property adapters must use this helper; a source-level boundary test rejects direct `raw != 0` or ad hoc outbound conversions outside `ffi::scalar`.

Implementation evidence showed that bindgen omits the released cast-style `kARAFalse` and
`kARATrue` C macros, and that individual generated field extents were not machine-iterable. Extend
the existing Phase 0 generator to emit those two audited constants and a declaration-ordered
field-extent slice for every generated record. `core` consumes those generated facts; it must not
duplicate ABI layouts by hand.

- [x] **Step 4: Add property-based malformed-buffer coverage**

Run: `cargo test -p ara2-bridge-core --test ffi_validation && cargo miri test -p ara2-bridge-core --test ffi_validation`  
Expected: PASS for pointer validation and `ARABool` values 0/1/2 plus canonical output; arbitrary unreadable pointer tests are absent from in-process Miri and reserved for later sanitizer subprocesses.

- [ ] **Step 5: Commit**

```bash
git add -- Cargo.toml Cargo.lock ara2-bridge-core/Cargo.toml ara2-bridge-core/src/lib.rs ara2-bridge-core/src/ffi/mod.rs ara2-bridge-core/src/ffi/sized.rs ara2-bridge-core/src/ffi/slice.rs ara2-bridge-core/src/ffi/string.rs ara2-bridge-core/src/ffi/scalar.rs ara2-bridge-core/tests/ffi_validation.rs
git commit -m "feat(core): validate ara foreign storage"
```

### Task 5: Create owned aligned property mirrors and builders

**Files:**
- Create: `ara2-bridge-core/src/properties/mod.rs`
- Create: `ara2-bridge-core/src/properties/document.rs`
- Create: `ara2-bridge-core/src/properties/model.rs`
- Create: `ara2-bridge-core/src/properties/selection.rs`
- Create: `ara2-bridge-core/tests/properties.rs`
- Modify: `ara2-bridge-core/src/lib.rs`

- [x] **Step 1: Write failing packed-input ownership tests**

```rust
#[test]
fn audio_source_properties_copy_ephemeral_strings() {
    let input = fixture_audio_source_properties("take.wav", "source-1");
    let owned = unsafe { AudioSourceProperties::copy_from_ffi(&input).unwrap() };
    drop(input);
    assert_eq!(owned.name(), Some("take.wav"));
    assert_eq!(owned.persistent_id(), "source-1");
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-core --test properties`  
Expected: FAIL on missing owned property types.

- [x] **Step 3: Implement one owned type per property family**

Builders own `CString`s, colors, arrays, and validated `RawChannelArrangement` storage that phase 2 lifts into safe named variants; `as_ffi()` returns a pinned call guard whose pointers remain stable. The core implementation copies fixed-size channel layouts and core-visible variable layouts whose extent is derivable from the ARA header. CoreAudio and CLAP ambisonic payloads return `Unsupported` until phase 5 companion adapters validate their SDK-specific extent; core never guesses a size or retains a borrowed pointer. Playback regions require a region sequence for generation 2+, musical contexts and colors remain optional, numeric values are finite, and persistent IDs are document-valid ASCII. Poison padding before construction, prove every byte in each emitted prefix is initialized, set the exact generation-specific `structSize`, and reject `usize::MAX` count/length multiplication before allocating.

```rust
pub struct FfiProperties<'a, T> {
    raw: T,
    _backing: Pin<&'a PropertyBacking>,
}
```

- [x] **Step 4: Run full property tests**

Run: `cargo test -p ara2-bridge-core --test properties`  
Expected: PASS for minimum/full/future `structSize`, unaligned input, retained output backing, poisoned padding, exact outbound prefixes, and arithmetic overflow.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-core/src/lib.rs ara2-bridge-core/src/properties/mod.rs ara2-bridge-core/src/properties/document.rs ara2-bridge-core/src/properties/model.rs ara2-bridge-core/src/properties/selection.rs ara2-bridge-core/tests/properties.rs
git commit -m "feat(core): own ara model properties"
```

### Task 6: Encode threads, lifecycles, and poisoning

**Files:**
- Create: `ara2-bridge-core/src/threading.rs`
- Create: `ara2-bridge-core/src/realtime.rs`
- Create: `ara2-bridge-core/src/lifecycle.rs`
- Create: `ara2-bridge-core/src/poison.rs`
- Create: `ara2-bridge-core/tests/lifecycle.rs`
- Create: `ara2-bridge-core/tests/realtime.rs`
- Modify: `ara2-bridge-core/src/lib.rs`
- Modify: `ara2-bridge-core/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

- [x] **Step 1: Add workspace `parking_lot = "0.12"` and `crossbeam-queue = "0.3"`, then write failing transition and realtime instrumentation tests**

```rust
#[test]
fn edit_restore_and_poison_transitions_are_checked() {
    let state = Lifecycle::new_on_current_thread();
    let edit = state.begin_editing().unwrap();
    assert!(state.begin_editing().is_err());
    drop(edit.finish().unwrap());
    state.poison(Diagnostic::new(AraError::Poisoned));
    assert!(state.begin_editing().is_err());
    assert!(state.begin_teardown().is_ok());
}
```

In `tests/realtime.rs`, assert that a missing `RealtimeHeadTailView` cannot answer a query and instrument allocation, blocking-lock acquisition, file I/O, and synchronous logging so each forbidden operation fails the test.

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-core --test lifecycle && cargo test -p ara2-bridge-core --test realtime`  
Expected: FAIL on the missing lifecycle state and missing realtime snapshot API before either is implemented.

- [x] **Step 3: Implement scoped states**

Add model-thread identity, `EditGuard`, ARA1 restore guard, ARA2 edit-plus-restore guard, sample-access state, content-call exclusivity, render activation, and poison state. `Drop` balances best-effort but explicit `finish()` returns observable errors. Only teardown/diagnostics are legal after poison. Add `RealtimeHeadTailView`, an immutable bounded snapshot for `getPlaybackRegionHeadAndTailTime`; reads are allocation-free, lock-free, nonblocking, and logging-free. Realtime failures enqueue fixed-size codes in a preallocated lock-free buffer for later model-thread diagnostic expansion.

- [x] **Step 4: Run deterministic transition tests**

Run: `cargo test -p ara2-bridge-core --test lifecycle && cargo test -p ara2-bridge-core --test realtime`  
Expected: PASS for every legal/illegal transition, wrong-thread call, and head/tail query under allocation/lock/I/O/log instrumentation.

- [ ] **Step 5: Commit**

```bash
git add -- Cargo.toml Cargo.lock ara2-bridge-core/Cargo.toml ara2-bridge-core/src/lib.rs ara2-bridge-core/src/threading.rs ara2-bridge-core/src/realtime.rs ara2-bridge-core/src/lifecycle.rs ara2-bridge-core/src/poison.rs ara2-bridge-core/tests/lifecycle.rs ara2-bridge-core/tests/realtime.rs
git commit -m "feat(core): encode ara lifecycle and thread rules"
```

### Task 7: Add panic-safe callback dispatch

**Files:**
- Create: `ara2-bridge-core/src/dispatch.rs`
- Create: `ara2-bridge-core/tests/dispatch.rs`
- Modify: `ara2-bridge-core/src/lib.rs`

- [x] **Step 1: Write failing panic/sentinel tests**

```rust
#[test]
fn panic_is_recorded_poisoned_and_mapped_to_false() {
    let runtime = fixture_runtime(|| panic!("boom"));
    let result = dispatch_bool(&runtime, "Interface", "method", || runtime.call());
    assert_eq!(result, kARAFalse);
    assert!(runtime.is_poisoned());
    assert!(runtime.diagnostics().iter().any(|d| d.method() == Some("method")));
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-core --test dispatch`  
Expected: FAIL on missing dispatch adapters.

- [x] **Step 3: Implement method-specific adapters**

Provide `dispatch_void`, `dispatch_bool`, `dispatch_ref`, `dispatch_i32`, and `dispatch_time_pair`. Each performs validated state recovery, `catch_unwind(AssertUnwindSafe(...))`, diagnostic recording, poisoning, and exact sentinel mapping. No panic payload formatting occurs on realtime-designated paths.

- [x] **Step 4: Run panic and nested-callback tests**

Run: `cargo test -p ara2-bridge-core --test dispatch && cargo miri test -p ara2-bridge-core --test dispatch`  
Expected: PASS and no unwind crosses an `extern "C"` test boundary.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-core/src/lib.rs ara2-bridge-core/src/dispatch.rs ara2-bridge-core/tests/dispatch.rs
git commit -m "feat(core): contain panics in ara dispatch"
```

### Task 8: Core phase gate and documentation

**Files:**
- Modify: `ara2-bridge-core/src/lib.rs`
- Modify: `ara2-bridge-plugin/src/lib.rs`
- Modify: `ara2-bridge-host/src/lib.rs`
- Modify: `ara2-bridge-companion/src/lib.rs`
- Modify: `ara2-bridge-testkit/src/lib.rs`
- Modify: `ara2-bridge/src/lib.rs`
- Create: `ara2-bridge-core/README.md`
- Modify: `.github/workflows/ci.yml`
- Create: `ara2-bridge-core/tests/clippy_ui.rs`
- Create: `ara2-bridge-core/tests/clippy-fixtures/missing-safety-doc/Cargo.toml`
- Create: `ara2-bridge-core/tests/clippy-fixtures/missing-safety-doc/src/lib.rs`
- Create: `ara2-bridge-core/tests/clippy-fixtures/undocumented-unsafe-block/Cargo.toml`
- Create: `ara2-bridge-core/tests/clippy-fixtures/undocumented-unsafe-block/src/lib.rs`
- Create: `docs/superpowers/handoffs/phase-1-core.md`

- [x] **Step 1: Add crate-root example and safety documentation checks**

Document the core boundary, handle ownership, caller-valid foreign-pointer precondition, generation coordination, and why raw callback authors should use higher-level runtimes. Enable `#![deny(missing_docs)]`, `#![deny(unsafe_op_in_unsafe_fn)]`, `#![deny(clippy::missing_safety_doc)]`, and `#![deny(clippy::undocumented_unsafe_blocks)]` in every safe crate. Keep trybuild for rustc lifetime/thread errors. Give each Clippy fixture manifest its own empty `[workspace]` so it is isolated from the repository workspace. A dedicated integration test invokes `cargo clippy --manifest-path <fixture>/Cargo.toml -- -D clippy::missing-safety-doc -D clippy::undocumented-unsafe-blocks` for the two fixture crates and asserts the exact missing-`# Safety` and undocumented-unsafe-block diagnostics.

- [x] **Step 2: Run the core gate**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo test -p ara2-bridge-core --test clippy_ui && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps && cargo miri test -p ara2-bridge-core --test diagnostics && cargo miri test -p ara2-bridge-core --test generation && cargo miri test -p ara2-bridge-core --test registry && cargo miri test -p ara2-bridge-core --test ffi_validation && cargo miri test -p ara2-bridge-core --test properties && cargo miri test -p ara2-bridge-core --test lifecycle && cargo miri test -p ara2-bridge-core --test realtime && cargo miri test -p ara2-bridge-core --test dispatch`  
Expected: PASS with zero undocumented unsafe items. The explicit Miri list covers validation, registries, owned properties, lifecycle/realtime state, dispatch, RAII, and destruction while intentionally excluding trybuild and subprocess-driven Clippy tests.

- [x] **Step 3: Write the compact phase handoff**

Record public types/modules, safety invariants, generated inputs, exact gate commands/results, and normative revisions already committed in this phase. The gate fails if any discovered normative revision remains pending; omit completed task narration.

- [ ] **Step 4: Commit the phase gate**

```bash
git add -- ara2-bridge-core/src/lib.rs ara2-bridge-plugin/src/lib.rs ara2-bridge-host/src/lib.rs ara2-bridge-companion/src/lib.rs ara2-bridge-testkit/src/lib.rs ara2-bridge/src/lib.rs ara2-bridge-core/README.md .github/workflows/ci.yml ara2-bridge-core/tests/clippy_ui.rs ara2-bridge-core/tests/clippy-fixtures/missing-safety-doc/Cargo.toml ara2-bridge-core/tests/clippy-fixtures/missing-safety-doc/src/lib.rs ara2-bridge-core/tests/clippy-fixtures/undocumented-unsafe-block/Cargo.toml ara2-bridge-core/tests/clippy-fixtures/undocumented-unsafe-block/src/lib.rs docs/superpowers/handoffs/phase-1-core.md
git commit -m "docs(core): complete safety boundary documentation"
```
