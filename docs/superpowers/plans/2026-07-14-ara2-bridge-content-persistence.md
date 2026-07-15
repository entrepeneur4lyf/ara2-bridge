# ARA2 Bridge Content and Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement typed ARA content, archive/filter abstractions, audio-file chunk XML/container support, processing metadata, licensing inputs, and the reusable ARA utility algorithms.

**Architecture:** Semantic types live in `ara2-bridge-core`; archive and audio-file transport use owned, bounded Rust APIs with no plug-in-specific payload format. Content readers expose owned iteration by default and a lending callback for zero-copy access. Container mutation is streaming and atomic at the path helper boundary.

**Tech Stack:** Rust 2021, `bitflags`, `base64`, `quick-xml`, `proptest`, golden WAVE/AIFF fixtures, fuzz targets.

---

Read first: specs `02`, `05`, `07`, `09` and `docs/superpowers/handoffs/phase-1-core.md`; keep the host/plugin runtime specs available only for integration signatures.

### Task 1: Model all six typed content kinds

**Files:**
- Create: `ara2-bridge-core/src/content/mod.rs`
- Create: `ara2-bridge-core/src/content/events.rs`
- Create: `ara2-bridge-core/src/content/kind.rs`
- Create: `ara2-bridge-core/src/content/raw.rs`
- Create: `ara2-bridge-core/src/content/validate.rs`
- Create: `ara2-bridge-core/tests/content_events.rs`
- Modify: `ara2-bridge-core/src/lib.rs`
- Modify: `ara2-bridge-core/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `xtask/src/bindings.rs`
- Modify: `ara2-bridge-sys/src/generated/{x86_64,aarch64,i686}.rs`
- Modify: `ara2-bridge-sys/tests/generated/core_abi_assertions.rs`
- Modify: `ara2-bridge-sys/tests/generated/{x86_64,aarch64,i686}-core-abi.json`
- Modify: `ara2-bridge-sys/tests/scalar_constants.rs`

- [x] **Step 1: Add workspace `bitflags = "2"` and write failing kind/event round-trip tests**

```rust
#[test]
fn every_released_kind_has_a_typed_event() {
    assert_kind::<Tempo>(kARAContentTypeTempoEntries);
    assert_kind::<BarSignatures>(kARAContentTypeBarSignatures);
    assert_kind::<Notes>(kARAContentTypeNotes);
    assert_kind::<StaticTuning>(kARAContentTypeStaticTuning);
    assert_kind::<KeySignatures>(kARAContentTypeKeySignatures);
    assert_kind::<SheetChords>(kARAContentTypeSheetChords);
}
```

- [x] **Step 2: Run and verify the decoder red state**

Run: `cargo test -p ara2-bridge-core --test content_events`  
Expected: FAIL on the deliberately unsupported placeholder decoder after the complete public module shape compiles.

- [x] **Step 3: Implement sealed markers and aligned owned events**

```rust
pub trait ContentKind: sealed::Sealed + 'static {
    type Event: Clone + Send + Sync + 'static;
    const RAW_TYPE: ARAContentType;
}
```

The private sealed raw backend implements `unsafe fn copy_event(storage: EventStorage<'_>)`; only the FFI validator can construct `EventStorage` after proving caller-valid readable storage, content-kind identity, and the complete event extent. No public safe method accepts a raw pointer. Port exact field validation for finite/order/range/pitch/enums and copy C strings immediately. Represent grades and update flags non-exhaustively so unknown values/bits survive where ARA permits them.

Decoder implementation exposed three scalar macro defects in the pregenerated boundary: bindgen omitted `kARAInvalidPitchNumber` and widened `kARAInvalidFrequency`/`kARADefaultConcertPitchFrequency` from C `float` to Rust `f64`. Normalize these in `xtask`, add compile-time type tests, regenerate all target bindings/assertions, and recompile/import all three C/C++ probe envelopes before declaring the event decoder green.

- [x] **Step 4: Run event and unaligned-input tests**

Run: `cargo test -p ara2-bridge-core --test content_events`  
Expected: PASS for all six kinds, invalid numeric boundaries, and packed input.

- [ ] **Step 5: Commit**

```bash
git add -- Cargo.toml Cargo.lock ara2-bridge-core/Cargo.toml ara2-bridge-core/src/lib.rs ara2-bridge-core/src/content/mod.rs ara2-bridge-core/src/content/events.rs ara2-bridge-core/src/content/kind.rs ara2-bridge-core/src/content/raw.rs ara2-bridge-core/src/content/validate.rs ara2-bridge-core/tests/content_events.rs
git commit -m "feat(core): add typed ara content events"
```

### Task 2: Implement owned and lending reader semantics

**Files:**
- Create: `ara2-bridge-core/src/content/reader.rs`
- Create: `ara2-bridge-core/tests/content_reader.rs`
- Create: `ara2-bridge-core/tests/ui/content_event_escape.rs`
- Create: `ara2-bridge-core/tests/ui/content_event_escape.stderr`
- Modify: `ara2-bridge-core/tests/ui.rs`
- Modify: `ara2-bridge-core/src/content/mod.rs`

- [x] **Step 1: Write failing invalidation and destruction tests**

```rust
#[test]
fn owned_iteration_survives_next_peer_data_call() {
    let peer = RotatingScratchPeer::new(two_note_events());
    let mut reader = ContentReader::<Notes>::new(peer);
    let first = reader.event(0).unwrap();
    let _second = reader.event(1).unwrap();
    assert_eq!(first.pitch_number(), 60);
    drop(reader);
    assert_eq!(destroy_count(), 1);
}

#[test]
fn reader_guard_blocks_conflicting_controller_operations_until_drop() {
    let session = fixture_session();
    let reader = session.notes_reader().unwrap();
    assert!(session.begin_editing().is_err());
    assert!(session.begin_restoring().is_err());
    assert!(session.create_content_reader(ContentType::TempoEntries).is_err());
    assert!(session.close_controller().is_err());
    drop(reader);
    assert!(session.begin_editing().is_ok());
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-core --test content_reader`  
Expected: FAIL on missing reader.

- [x] **Step 3: Implement reader backends and lending closure**

```rust
pub fn with_event<R>(&mut self, index: usize, f: impl for<'event> FnOnce(EventRef<'event, K>) -> R) -> Result<R, AraError>;
pub fn event(&mut self, index: usize) -> Result<K::Event, AraError>;
```

Every typed or dynamic reader owns a controller/session reader guard from successful creation through `Drop`. The guard rejects editing, restoration, conflicting content calls, and controller destruction while any reader is alive. Check count conversion and bounds, also forbid reentrant controller calls during `with_event`, and destroy exactly once across success, error, and panic. Dynamic readers retain raw content type and downcast only after exact validation.

- [x] **Step 4: Run behavior and lock the compile-fail snapshot**

Run: `cargo test -p ara2-bridge-core --test content_reader && TRYBUILD=overwrite cargo test -p ara2-bridge-core --test ui`  
Expected: PASS while creating `content_event_escape.stderr`; review it and require the intended higher-ranked closure lifetime error with no unrelated diagnostic.  
Run: `cargo test -p ara2-bridge-core --test ui`  
Expected: PASS with both reviewed UI snapshots unchanged; `EventRef` cannot escape the closure.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-core/src/content/mod.rs ara2-bridge-core/src/content/reader.rs ara2-bridge-core/tests/content_reader.rs ara2-bridge-core/tests/ui.rs ara2-bridge-core/tests/ui/content_event_escape.rs ara2-bridge-core/tests/ui/content_event_escape.stderr
git commit -m "feat(core): enforce ara content reader lifetimes"
```

### Task 3: Add random-access archive transport and filters

**Files:**
- Create: `ara2-bridge-core/src/archive/mod.rs`
- Create: `ara2-bridge-core/src/archive/io.rs`
- Create: `ara2-bridge-core/src/archive/filter.rs`
- Create: `ara2-bridge-core/tests/archive.rs`
- Modify: `ara2-bridge-core/src/lib.rs`
- Modify: `ara2-bridge-core/src/error.rs`
- Modify: `ara2-bridge-core/src/registry.rs`
- Modify: `ara2-bridge-core/src/properties/model.rs`
- Modify: `ara2-bridge-core/src/properties/mod.rs`

- [x] **Step 1: Write failing random-access and overflow tests**

```rust
#[test]
fn archive_io_is_position_based_and_checked() {
    let archive = MemoryArchive::from(vec![1, 2, 3, 4]);
    let mut out = [0_u8; 2];
    archive.read_at(1, &mut out).unwrap();
    assert_eq!(out, [2, 3]);
    assert!(matches!(archive.read_at(u64::MAX, &mut out), Err(AraError::Archive(ArchiveError::RangeOverflow))));
}

#[test]
fn restore_filter_rejects_duplicate_archive_ids() {
    assert!(RestoreFilter::builder().audio_source("a", "x").audio_source("a", "y").build().is_err());
}
```

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-core --test archive`  
Expected: FAIL on missing archive/filter APIs.

- [x] **Step 3: Implement `ReadAt`, `WriteAt`, owned filters, and progress**

```rust
pub trait ReadAt { fn len(&self) -> Result<u64, AraError>; fn read_at(&self, pos: u64, out: &mut [u8]) -> Result<(), AraError>; }
pub trait WriteAt { fn write_at(&mut self, pos: u64, data: &[u8]) -> Result<(), AraError>; }
```

Validate position+length, 32-bit refusal, count/pointer pairs, unique references/IDs, mapping lengths, ownership, and document-data ordering. A null FFI filter maps to `FilterSelection::All`. Progress accepts finite monotonic values only.

Store-filter ownership evidence requires one document identity shared by its typed registries. Add `RegistrySession` plus `Registry::in_session`; keep `Registry::new` as a standalone-session convenience. Add the missing `AudioModificationKind` handle marker and make store filters accept only session-matching typed audio-source and audio-modification handles.

- [x] **Step 4: Run archive property tests**

Run: `cargo test -p ara2-bridge-core --test archive`  
Expected: PASS for sparse writes, overflow, and partial restore ordering. The phase gate separately runs the 32-bit-only refusal path.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-core/src/lib.rs ara2-bridge-core/src/archive/mod.rs ara2-bridge-core/src/archive/io.rs ara2-bridge-core/src/archive/filter.rs ara2-bridge-core/tests/archive.rs
git commit -m "feat(core): add checked ara archive transport"
```

### Task 4: Parse and emit the exact ARA iXML schema

**Files:**
- Create: `ara2-bridge-core/src/audio_file/mod.rs`
- Create: `ara2-bridge-core/src/audio_file/xml.rs`
- Create: `ara2-bridge-core/tests/audio_file_xml.rs`
- Modify: `ara2-bridge-core/src/lib.rs`
- Create: `ara2-bridge-testkit/fixtures/chunks/legacy-missing-distinct.xml`
- Create: `ara2-bridge-testkit/fixtures/chunks/full-2.3.xml`
- Create: `ara2-bridge-testkit/fixtures/chunks/namespace-qualified.xml`
- Create: `ara2-bridge-testkit/fixtures/chunks/unrelated-ordering.xml`
- Create: `ara2-bridge-testkit/fixtures/chunks/multi-entry-order.xml`
- Modify: `sdk-provenance.toml`
- Modify: `ara2-bridge-core/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `xtask/src/fixtures.rs`
- Create: `xtask/tests/fixtures.rs`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/ara.rs`

- [x] **Step 1: Add dependencies, register the fixture command, and write failing generator/schema tests**

Add workspace `base64 = "0.22"` and `quick-xml = "0.37"`. Export `xtask::fixtures`, register `fixtures --write|--check --set <name>`, and make `xtask/tests/fixtures.rs` test an absent XML output plus a one-byte stale output before adding the schema tests below.

```rust
#[test]
fn legacy_chunk_defaults_optional_booleans_to_false() {
    let set = AraChunkSet::parse(include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/legacy-missing-distinct.xml")).unwrap();
    let entry = set.get("com.example.archive").unwrap();
    assert!(!entry.open_automatically());
    assert!(!entry.create_distinct_audio_modification());
}

#[test]
fn duplicate_singletons_are_errors() {
    assert!(matches!(AraChunkSet::parse(DUPLICATE_ARCHIVE_DATA), Err(ChunkError::DuplicateElement("archiveData"))));
}

#[test]
fn namespace_and_unrelated_ixml_order_survive_round_trip() {
    let input = fixture("namespace-qualified.xml");
    let reparsed = AraChunkSet::parse(&AraChunkSet::parse(input).unwrap().emit()).unwrap();
    assert_eq!(reparsed.archive_ids(), ["first", "second"]);
    assert_unrelated_nodes_and_attributes_preserved(input, reparsed.document());
}
```

- [x] **Step 2: Verify the deterministic generator failure**

Run: `cargo test -p xtask --test fixtures`  
Expected: FAIL on the deliberately absent/stale `chunk-xml` output, not an unresolved module or command.

- [x] **Step 3: Implement and run deterministic XML fixture generation**

Implement `chunk-xml` generation for the five exact task paths. Each output is derived from a reviewed structured recipe; the generator atomically updates `sdk-provenance.toml` with recipe/source repository, Apache-2.0 license, input hash, and output SHA-256. `--check` is non-mutating and rejects missing, extra, empty, or stale outputs and provenance.

Run: `cargo xtask ara fixtures --write --set chunk-xml && cargo xtask ara fixtures --check --set chunk-xml && cargo test -p xtask --test fixtures && cargo xtask ara provenance --check && cargo test -p ara2-bridge-core --test audio_file_xml`  
Expected: generator/freshness checks PASS and the core test FAILS on the missing parser.

- [x] **Step 4: Implement bounded XML parsing and canonical emission**

Model every table row from spec `05`: exact cardinality, ASCII IDs, default false booleans, optional suggested metadata, MIME Base64 input, zero-byte archive acceptance, duplicate archive-ID rejection, unknown-node/attribute retention, namespace-tolerant matching, and configurable XML/decoded-size limits. Canonical output writes both booleans and unwrapped Base64, preserves unrelated iXML nodes and attributes in relative order, and preserves multi-entry dictionary order across parse–emit–parse.

- [x] **Step 5: Run golden and entity-expansion tests**

Run: `cargo xtask ara fixtures --check --set chunk-xml && cargo xtask ara provenance --check && cargo test -p xtask --test fixtures && cargo test -p ara2-bridge-core --test audio_file_xml`  
Expected: PASS for namespace-qualified input, unrelated nodes/attributes before and after `ARA`, multi-entry dictionary order, parse–emit–parse preservation, and entity rejection without external resolution.

- [ ] **Step 6: Commit**

```bash
git add -- Cargo.toml Cargo.lock ara2-bridge-core/Cargo.toml ara2-bridge-core/src/lib.rs ara2-bridge-core/src/audio_file/mod.rs ara2-bridge-core/src/audio_file/xml.rs ara2-bridge-core/tests/audio_file_xml.rs ara2-bridge-testkit/fixtures/chunks/legacy-missing-distinct.xml ara2-bridge-testkit/fixtures/chunks/full-2.3.xml ara2-bridge-testkit/fixtures/chunks/namespace-qualified.xml ara2-bridge-testkit/fixtures/chunks/unrelated-ordering.xml ara2-bridge-testkit/fixtures/chunks/multi-entry-order.xml sdk-provenance.toml xtask/src/fixtures.rs xtask/tests/fixtures.rs xtask/src/lib.rs xtask/src/ara.rs
git commit -m "feat(core): parse ara audio file chunk xml"
```

### Task 5: Implement WAVE/RF64/BW64/AIFF/AIFC streaming containers

**Files:**
- Create: `ara2-bridge-core/src/audio_file/container.rs`
- Create: `ara2-bridge-core/src/audio_file/path.rs`
- Create: `ara2-bridge-core/tests/audio_file_container.rs`
- Modify: `ara2-bridge-core/src/audio_file/mod.rs`
- Create: `ara2-bridge-testkit/fixtures/audio/wave-unknown-odd.wav`
- Create: `ara2-bridge-testkit/fixtures/audio/rf64-ds64.wav`
- Create: `ara2-bridge-testkit/fixtures/audio/bw64-ds64.wav`
- Create: `ara2-bridge-testkit/fixtures/audio/aiff-unknown-odd.aiff`
- Create: `ara2-bridge-testkit/fixtures/audio/aifc-unknown-odd.aifc`
- Modify: `sdk-provenance.toml`
- Modify: `xtask/src/fixtures.rs`
- Modify: `xtask/tests/fixtures.rs`

- [x] **Step 1: Red-test container-fixture freshness and write failing unknown-chunk/padding round trips**

Extend the fixture integration test with absent and one-byte stale cases for the `audio-containers` set before adding the runtime test below.

```rust
#[test]
fn wave_rewrite_preserves_unknown_chunks_and_padding() {
    let input = fixture("wave-unknown-odd.wav");
    let output = rewrite_ixml(&input, full_chunk_set()).unwrap();
    assert_eq!(chunk(&output, *b"JUNK"), chunk(&input, *b"JUNK"));
    assert_eq!(AraChunkSet::from_audio(&output).unwrap(), full_chunk_set());
}
```

- [x] **Step 2: Verify generator failure**

Run: `cargo test -p xtask --test fixtures`  
Expected: FAIL on the absent/stale `audio-containers` outputs.

- [x] **Step 3: Extend deterministic generation and verify the runtime red test**

Generate the five minimal binary fixtures from structured recipes, atomically update their source/license/input/output hashes in `sdk-provenance.toml`, and make `--check` reject missing, extra, empty, or stale bytes.

Run: `cargo xtask ara fixtures --write --set audio-containers && cargo xtask ara fixtures --check --set audio-containers && cargo test -p xtask --test fixtures && cargo xtask ara provenance --check && cargo test -p ara2-bridge-core --test audio_file_container`  
Expected: fixture checks PASS and the runtime test FAILS on missing container rewrite.

- [x] **Step 4: Implement streaming parse/rewrite**

Support RIFF/WAVE, RF64/BW64 `ds64`, AIFF, and AIFC with correct endian lengths and even-byte padding. Preserve unknown chunks and order, require at most one unambiguous iXML chunk, reject Wave64 and conflicting iXML, and write to caller-provided `Read + Seek`/`Write + Seek` streams without mutating input.

- [x] **Step 5: Implement atomic path replacement**

Use same-directory temporary files, copied permissions, file+directory fsync where supported, symlink refusal by default, validation before rename, and diagnostic retention of a Windows sharing-violation temporary path.

- [x] **Step 6: Run container tests**

Run: `cargo xtask ara fixtures --check --set audio-containers && cargo xtask ara provenance --check && cargo test -p xtask --test fixtures && cargo test -p ara2-bridge-core --test audio_file_container`  
Expected: PASS for all four container families, large-size tables, odd chunks, ambiguity, symlink, and injected-write failures.

- [ ] **Step 7: Commit**

```bash
git add -- ara2-bridge-core/src/audio_file/mod.rs ara2-bridge-core/src/audio_file/container.rs ara2-bridge-core/src/audio_file/path.rs ara2-bridge-core/tests/audio_file_container.rs ara2-bridge-testkit/fixtures/audio/wave-unknown-odd.wav ara2-bridge-testkit/fixtures/audio/rf64-ds64.wav ara2-bridge-testkit/fixtures/audio/bw64-ds64.wav ara2-bridge-testkit/fixtures/audio/aiff-unknown-odd.aiff ara2-bridge-testkit/fixtures/audio/aifc-unknown-odd.aifc sdk-provenance.toml xtask/src/fixtures.rs xtask/tests/fixtures.rs
git commit -m "feat(core): edit ara chunks in audio containers"
```

### Task 6: Port reusable ARA utilities and processing metadata

**Files:**
- Create: `ara2-bridge-core/src/util/mod.rs`
- Create: `ara2-bridge-core/src/util/time.rs`
- Create: `ara2-bridge-core/src/util/tempo.rs`
- Create: `ara2-bridge-core/src/util/pitch.rs`
- Create: `ara2-bridge-core/src/util/harmony.rs`
- Create: `ara2-bridge-core/src/channel.rs`
- Create: `ara2-bridge-core/src/processing.rs`
- Create: `ara2-bridge-core/tests/utilities.rs`
- Modify: `ara2-bridge-core/src/lib.rs`
- Modify: `sdk-provenance.toml`

- [x] **Step 1: Add upstream-derived vector tests**

```rust
#[test]
fn sample_time_rounding_matches_ara_library() {
    assert_eq!(time_to_sample(0.5, 44_100.0), 22_050);
    assert_eq!(time_to_sample(-0.5 / 44_100.0, 44_100.0), 0);
}
```

The negative half-sample expectation was corrected during the source-level port: upstream uses `floor(x + 0.5)`, so exactly `-0.5` maps to `0` and `-1.5` maps to `-1`.

- [x] **Step 2: Verify failure**

Run: `cargo test -p ara2-bridge-core --test utilities`  
Expected: FAIL on missing utility functions.

- [x] **Step 3: Port only listed library algorithms**

Implement sample/time conversion with ARA rounding; tempo/bar mapping; pitch interpretation; chord and key-signature interpretation; international, German, and ASCII naming; content-range intersection; flag helpers; and channel-arrangement inspection. Channel arrangements have owned safe variants for undefined, VST3 speaker arrangement, Core Audio layout, AAX stem format, CLAP channel map, and CLAP ambisonic data; unknown tags require an explicitly unsafe opaque representation or are rejected. Add `ProcessingAlgorithmProperties` with controller-lifetime backing, stable index validation, and `LicenseRequest` subset validation. Record upstream source/hash in provenance for every ported vector/algorithm.

- [x] **Step 4: Run utility tests**

Run: `cargo test -p ara2-bridge-core --test utilities`  
Expected: PASS for boundary, negative-time, overflow, pitch/harmony naming vectors, every channel arrangement, unknown flag/tag, and capability-subset cases.

- [ ] **Step 5: Commit**

```bash
git add -- ara2-bridge-core/src/lib.rs ara2-bridge-core/src/util/mod.rs ara2-bridge-core/src/util/time.rs ara2-bridge-core/src/util/tempo.rs ara2-bridge-core/src/util/pitch.rs ara2-bridge-core/src/util/harmony.rs ara2-bridge-core/src/channel.rs ara2-bridge-core/src/processing.rs ara2-bridge-core/tests/utilities.rs sdk-provenance.toml
git commit -m "feat(core): port ara content and time utilities"
```

### Task 7: Content/persistence phase gate

**Files:**
- Modify: `ara2-bridge-core/src/lib.rs`
- Modify: `ara2-bridge-core/README.md`
- Create: `ara2-bridge-core/examples/content-reader.rs`
- Create: `ara2-bridge-core/examples/archive-roundtrip.rs`
- Create: `ara2-bridge-core/examples/audio-file-chunk.rs`
- Create: `fuzz/Cargo.toml`
- Create: `fuzz/Cargo.lock`
- Create: `fuzz/fuzz_targets/audio_file_xml.rs`
- Create: `fuzz/fuzz_targets/audio_file_container.rs`
- Create: `docs/superpowers/handoffs/phase-2-content.md`

- [x] **Step 1: Add public examples for content, archives, and chunk editing**

Examples must compile and show owned event iteration, random-access archive use, a legacy chunk parse, and atomic file rewrite without raw pointers.

- [x] **Step 2: Add bounded fuzz targets**

```rust
fuzz_target!(|data: &[u8]| {
    let _ = AraChunkSet::parse_with_limits(data, Limits::fuzz());
});
```

- [x] **Step 3: Run the phase gate**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps && cargo +nightly fuzz run audio_file_xml -- -runs=10000 && cargo +nightly fuzz run audio_file_container -- -runs=10000`  
Expected: PASS with no panic, timeout, or oversized allocation.

- [x] **Step 4: Execute the 32-bit archive refusal path**

Run on 32-bit Windows: `cargo test --target i686-pc-windows-msvc -p ara2-bridge-core --test archive archive_larger_than_address_space_is_rejected`  
Expected: PASS using a sparse fixture declaring more than 4 GiB and asserting exactly `AraError::ArchiveTooLargeForTarget` before allocation or I/O.

Local Linux execution uses the behavior-equivalent `i686-pc-windows-gnu` target under Wine with target-local MinGW tools; target-native CI retains the exact MSVC command above.

- [x] **Step 5: Write the compact phase handoff**

Record only public modules/types, generated artifacts, exact gate commands/results, target caveats, and normative revisions already committed in this phase. The gate fails if any discovered normative revision remains pending. Do not copy task history.

- [ ] **Step 6: Commit**

```bash
git add -- ara2-bridge-core/src/lib.rs ara2-bridge-core/README.md ara2-bridge-core/examples/content-reader.rs ara2-bridge-core/examples/archive-roundtrip.rs ara2-bridge-core/examples/audio-file-chunk.rs fuzz/Cargo.toml fuzz/Cargo.lock fuzz/fuzz_targets/audio_file_xml.rs fuzz/fuzz_targets/audio_file_container.rs docs/superpowers/handoffs/phase-2-content.md
git commit -m "test(core): gate content persistence and chunk utilities"
```
