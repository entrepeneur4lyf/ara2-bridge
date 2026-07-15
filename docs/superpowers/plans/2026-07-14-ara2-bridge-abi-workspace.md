# ARA2 Bridge ABI and Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish the final crate graph and deterministic, pregenerated, target-correct ARA 2.3 ABI artifacts with no downstream bindgen requirement.

**Architecture:** A maintainer-only `xtask` reads the pinned headers and compatibility TOML, generates raw bindings/layout/access metadata for x86_64, AArch64, and i686, and checks provenance. `ara2-bridge-sys` selects checked-in artifacts by target and exposes no handwritten behavioral API.

**Tech Stack:** Rust 2021, bindgen 0.71 (xtask only), serde/toml, sha2, C11/C++17 probes, Cargo workspaces.

---

Read first: specs `00`, `01`, `07`, `08`, `09`, and `api-compatibility.toml`.

### Task 0: Provision and preflight immutable external SDK inputs

**Files:**
- Create: `ci/reference-sdks.lock.toml`
- Create: `ci/bootstrap-reference-sdks.sh`
- Create: `ci/tests/bootstrap-reference-sdks.sh`
- Modify: `.gitignore`

- [x] **Step 1: Write the failing absent-input preflight**

The shell test runs the non-mutating checker against an empty temporary root and requires a diagnostic naming `reference/ARA_SDK`, the Celemony repository URL, top-level commit, and explicit license flag.

Run: `ci/tests/bootstrap-reference-sdks.sh`  
Expected: FAIL on the deliberately unimplemented checker, not a missing script or network operation.

- [x] **Step 2: Lock every external repository and license decision**

`ci/reference-sdks.lock.toml` records canonical path, repository, tag where applicable, commit, Git tree when already pinned by the ARA superproject, license identifier/URL, and recursive submodule identities. The bootstrap verifies these Git identities; Task 2 separately records and verifies SHA-256 for every consumed file in `sdk-provenance.toml`. Lock these identities:

- ARA SDK: `https://github.com/Celemony/ARA_SDK.git` at `a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c`, tree `305a0dc9ba4759963c1e974353a999c3810b2319`; ARA API `65ec5c43b943a48cb5446f448a0492db6af8534b`, tree `2e3b0455f61314068d34501c5f71407d6ed0051b`; ARA Library `d18a6a5e489816316be84a9de0eaf7307bc1abe4`, tree `a53995463c520ba7aa1015a5cf8d7ae448007800`; ARA Examples `abd7c8aa5854591995e1fbf16f854c65b0998e8d`, tree `34919bd48ed748fc0889f1a0a8e532e37c8d4500`.
- CLAP: `https://github.com/free-audio/clap.git`, tag `1.1.9`, commit `094bb76c85366a13cc6c49292226d8608d6ae50c`.
- VST3: `https://github.com/steinbergmedia/vst3sdk.git`, tag/commit `v3.7.11_build_10` / `7d92338ae922db2d559ac458824a4df40f37e82e`.
- Audio Unit SDK: `https://github.com/apple/AudioUnitSDK.git`, tag/commit `AudioUnitSDK-1.0.0` / `53ea94e5efebf864b70afb673bdd60c977818ec7`.

The bootstrap command never infers license acceptance. It requires a component-specific `--accept-license <SPDX-or-commercial-policy-id>`, prints the locked license URL, refuses mismatches, clones into ignored `reference/` or `.third-party/`, initializes pinned recursive submodules, verifies commit/tree identity, and never updates an existing dirty checkout. Preserve existing ignore rules and add the exact anchored `.third-party/` rule beside the existing `reference/` rule; the shell test asserts both bootstrap roots are ignored and cannot enter a package or commit accidentally.

- [x] **Step 3: Provision and verify the ARA input**

Run: `ci/bootstrap-reference-sdks.sh fetch --component ara --accept-license Apache-2.0 && ci/bootstrap-reference-sdks.sh check --component ara && ci/tests/bootstrap-reference-sdks.sh`  
Expected: PASS from an empty temporary checkout and from the repository checkout; a second fetch is byte/identity stable, wrong commits/licenses and dirty inputs are rejected, and no root `.gitmodules` entry is required.

- [ ] **Step 4: Commit the bootstrap boundary**

```bash
git add -- .gitignore ci/reference-sdks.lock.toml ci/bootstrap-reference-sdks.sh ci/tests/bootstrap-reference-sdks.sh
git commit -m "build: lock external ara sdk inputs"
```

### Task 1: Lock the workspace dependency graph

**Files:**
- Create: `.cargo/config.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `ara2-bridge/Cargo.toml`
- Create: `ara2-bridge-core/Cargo.toml`
- Create: `ara2-bridge-core/src/lib.rs`
- Create: `ara2-bridge-plugin/Cargo.toml`
- Create: `ara2-bridge-plugin/src/lib.rs`
- Create: `ara2-bridge-host/Cargo.toml`
- Create: `ara2-bridge-host/src/lib.rs`
- Create: `ara2-bridge-companion/Cargo.toml`
- Create: `ara2-bridge-companion/src/lib.rs`
- Create: `ara2-bridge-testkit/Cargo.toml`
- Create: `ara2-bridge-testkit/src/lib.rs`
- Create: `ara2-bridge-testkit/build.rs`
- Create: `xtask/Cargo.toml`
- Create: `xtask/src/lib.rs`
- Create: `xtask/src/main.rs`
- Create: `xtask/src/ara.rs`
- Create: `xtask/tests/workspace.rs`

- [x] **Step 1: Seed the runnable `xtask` and add a failing workspace test**

Keep the repository's existing `ara2-bridge-sys` and `ara2-bridge` members, add only `xtask` as a new workspace member, install the Cargo alias, declare the exact dependencies listed below, create `xtask/src/lib.rs` and `main.rs` with the `ara` command router, update the root lockfile, then add this test. The five new `core`, `plugin`, `host`, `companion`, and `testkit` package skeletons remain absent for this red run. This bootstrap is intentionally non-behavioral; all subsequent red tests run through the real maintainer binary.

```rust
// xtask/tests/workspace.rs
#[test]
fn expected_packages_are_workspace_members() {
    let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();
    let members: std::collections::BTreeSet<_> = metadata.workspace_members.iter().collect();
    for expected in [
        "ara2-bridge-sys", "ara2-bridge-core", "ara2-bridge-plugin",
        "ara2-bridge-host", "ara2-bridge-companion", "ara2-bridge-testkit",
        "ara2-bridge", "xtask",
    ] {
        let package = metadata.packages.iter().find(|p| p.name == expected)
            .unwrap_or_else(|| panic!("missing workspace package {expected}"));
        assert!(members.contains(&package.id), "missing workspace member {expected}");
    }
}
```

- [x] **Step 2: Run the metadata test and verify the missing-package failure**

Run: `cargo test -p xtask --test workspace`  
Expected: the existing sys/facade checks pass, then the test binary FAILS with `missing workspace package ara2-bridge-core`; it must not fail with “package xtask not found” or an unlabelled `Option::unwrap()` panic.

- [x] **Step 3: Add all members, shared package metadata, and one-way dependencies**

```toml
# Cargo.toml
[workspace]
members = [
  "ara2-bridge-sys", "ara2-bridge-core", "ara2-bridge-plugin",
  "ara2-bridge-host", "ara2-bridge-companion", "ara2-bridge-testkit",
  "ara2-bridge", "xtask",
]
resolver = "2"

[workspace.package]
version = "0.2.0-alpha.1"
edition = "2021"
rust-version = "1.82"
license = "MIT OR Apache-2.0"
repository = "https://github.com/entrepeneur4lyf/ara2-bridge"
```

```toml
# .cargo/config.toml
[alias]
xtask = "run --package xtask --"
```

Each new library root initially enables `#![deny(missing_docs)]` and `#![deny(unsafe_op_in_unsafe_fn)]` plus crate-role rustdoc. The testkit gets a no-op `build.rs` so later cross-language tasks modify an existing explicit boundary. `xtask/src/lib.rs` exports its command modules so integration tests exercise the same implementation as the binary. Wire the exact acyclic DAG from spec `00`: `sys <- core <- plugin/host`; companion feature bundles depend on core plus the relevant runtime; testkit depends on focused crates but never the facade; the facade depends on focused crates and may optionally re-export testkit. Add metadata assertions for every allowed and forbidden edge.

Before the Step 2 red command, declare these exact workspace dependencies and opt `xtask` into them: `cargo_metadata = "0.19"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `toml = "0.8"`, `sha2 = "0.10"`, and maintainer-only `bindgen = "0.71"`; add `tempfile = "=3.14.0"` as the exact xtask dev-dependency. The exact pin prevents newer `tempfile` releases from pulling an Edition 2024 `getrandom` manifest that Cargo 1.82 cannot parse. Generate and retain the root `Cargo.lock`. Later plans opt crates into centrally pinned versions only in the task that first uses them and list every `Cargo.toml`/`Cargo.lock` mutation; no red command may rely on an unstated manifest edit.

The Rust 1.82 phase gate also requires the retained lockfile resolutions `indexmap 2.7.0` and
`jobserver 0.1.32`. Newer compatible-range releases either use an Edition 2024 manifest that
Cargo 1.82 cannot parse or declare Rust 1.85. The locked MSRV job is authoritative and must be
rerun whenever dependencies are updated.

- [x] **Step 4: Run workspace metadata and dependency checks**

Run: `cargo test -p xtask --test workspace && cargo check --workspace`  
Expected: PASS; no focused production crate (`sys`, `core`, `plugin`, `host`, or `companion`) depends on `ara2-bridge-testkit`. The aggregation-only facade's optional, off-by-default `testkit` edge is explicitly accepted, and testkit never depends on the facade.

- [ ] **Step 5: Commit the workspace skeleton**

```bash
git add -- .cargo/config.toml Cargo.toml Cargo.lock ara2-bridge/Cargo.toml ara2-bridge-core/Cargo.toml ara2-bridge-core/src/lib.rs ara2-bridge-plugin/Cargo.toml ara2-bridge-plugin/src/lib.rs ara2-bridge-host/Cargo.toml ara2-bridge-host/src/lib.rs ara2-bridge-companion/Cargo.toml ara2-bridge-companion/src/lib.rs ara2-bridge-testkit/Cargo.toml ara2-bridge-testkit/src/lib.rs ara2-bridge-testkit/build.rs xtask/Cargo.toml xtask/src/lib.rs xtask/src/main.rs xtask/src/ara.rs xtask/tests/workspace.rs
git commit -m "build: establish ara2 bridge workspace architecture"
```

### Task 2: Capture immutable SDK provenance

**Files:**
- Create: `sdk-provenance.toml`
- Create: `xtask/src/provenance.rs`
- Modify: `xtask/src/lib.rs`
- Create: `xtask/tests/provenance.rs`
- Modify: `xtask/src/main.rs`
- Modify: `xtask/src/ara.rs`

- [x] **Step 1: Export a minimal provenance module and write its failing manifest test**

Create the verifier API with manifest loading first; leave the manifest absent so the red condition is deterministic.

```rust
#[test]
fn pinned_ara_api_matches_manifest() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    xtask::provenance::verify(root, root.join("sdk-provenance.toml")).unwrap();
}
```

- [x] **Step 2: Run it and verify the missing-manifest failure**

Run: `cargo test -p xtask --test provenance`  
Expected: the test compiles and FAILS with `sdk-provenance.toml: No such file`.

- [x] **Step 3: Implement the manifest and verifier**

```toml
schema = 1
sdk_commit = "a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c"
ara_api_commit = "65ec5c43b943a48cb5446f448a0492db6af8534b"
ara_library_commit = "d18a6a5e489816316be84a9de0eaf7307bc1abe4"
ara_examples_commit = "abd7c8aa5854591995e1fbf16f854c65b0998e8d"
```

`verify()` must run `git -C <submodule> status --porcelain`, compare submodule HEADs, and SHA-256 every consumed header/license/behavioral source listed in `[[file]]` entries. `cargo xtask ara provenance --refresh` is the only command allowed to rewrite hashes; ordinary generation uses `--check` and rejects dirtiness.

- [x] **Step 4: Populate hashes and verify determinism**

Run: `cargo xtask ara provenance --refresh && FIRST=$(sha256sum sdk-provenance.toml) && cargo xtask ara provenance --check && cargo xtask ara provenance --refresh && test "$FIRST" = "$(sha256sum sdk-provenance.toml)" && cargo test -p xtask --test provenance`  
Expected: PASS; the second `--refresh` preserves the exact manifest bytes and the original integration test is green.

- [ ] **Step 5: Commit provenance**

```bash
git add -- sdk-provenance.toml xtask/src/provenance.rs xtask/src/lib.rs xtask/src/main.rs xtask/src/ara.rs xtask/tests/provenance.rs
git commit -m "build: pin ara sdk provenance"
```

### Task 3: Generate checked-in raw bindings per ABI

**Files:**
- Modify: `ara2-bridge-sys/Cargo.toml`
- Delete: `ara2-bridge-sys/build.rs`
- Delete: copied headers under `ara2-bridge-sys/*.h`
- Create: `ara2-bridge-sys/src/generated/x86_64.rs`
- Create: `ara2-bridge-sys/src/generated/aarch64.rs`
- Create: `ara2-bridge-sys/src/generated/i686.rs`
- Create: `ara2-bridge-sys/src/generated/mod.rs`
- Create: `ara2-bridge-sys/src/generated/audio_file_chunks.rs`
- Create: `ara2-bridge-sys/generated/symbol-coverage.json`
- Create: `ara2-bridge-sys/tests/audio_file_chunk_constants.rs`
- Modify: `ara2-bridge-sys/src/lib.rs`
- Create: `xtask/src/bindings.rs`
- Modify: `xtask/src/lib.rs`
- Create: `xtask/tests/bindings.rs`
- Modify: `xtask/src/ara.rs`

- [x] **Step 1: Register a minimal checker and add a failing pregenerated-artifact freshness test**

```rust
// First create `xtask::bindings` and export it from `xtask/src/lib.rs`. Its initial
// `generate(Check)` only checks the canonical output paths and reports a missing artifact.
#[test]
fn generated_bindings_are_current() {
    xtask::bindings::generate(xtask::Mode::Check).unwrap();
}
```

- [x] **Step 2: Confirm the test fails on absent generated files**

Run: `cargo test -p xtask --test bindings`  
Expected: the test compiles and FAILS naming `ara2-bridge-sys/src/generated/x86_64.rs`, not an unresolved `xtask::bindings` import.

- [x] **Step 3: Implement deterministic generation**

Configure bindgen with `EnumVariation::Consts`, `size_t_is_usize(true)`, explicit `--target`, C11 mode, allowlists `ARA.*`/`kARA.*`, and header inputs from `reference/ARA_SDK/ARA_API`. Normalize only absolute paths. Every generated derivative uses the shared provenance encoder: Rust/C/C++ gets a comment banner and JSON/TOML gets a top-level metadata object containing source repository, source tag `releases/2.3.0`, normative ARA API commit `65ec5c43b943a48cb5446f448a0492db6af8534b`, `ara2-bridge` generator crate/version, SPDX license `Apache-2.0`, and `DO NOT EDIT`. Markdown uses an equivalent HTML comment. Freshness tests remove or alter each field in turn and require a field-specific failure. Emit raw integer aliases for enums and retain `Option<unsafe extern "C" fn(...)>` callback types. Generate `audio_file_chunks.rs` from the C constants plus one reviewed synthetic `kARAXMLName_CreateDistinctAudioModification` entry sourced from the released C++ branch. Generate `symbol-coverage.json` by preprocessing only the self-contained core headers `ARAInterface.h` and `ARAAudioFileChunks.h`. Lexically inventory every ARA-owned macro, constant, typedef, struct/field, callback, and exported declaration in `ARACLAP.h`, `ARAVST3.h`, and `ARAAudioUnit.h` without resolving or preprocessing their CLAP, VST3, or platform includes; classify those records as companion-deferred with exact source spans and required SDKs. Tests reject missing/duplicate declarations, unknown classifications, or companion records incorrectly claimed ABI-proven in Phase 0. Companion Tasks 2/4/6 provision their pinned inputs, compile/preprocess each companion header, and close these records with complete symbol manifests. Register the target-selected generated modules from `ara2-bridge-sys/src/lib.rs` before the clang-free package check; Task 6 later narrows and documents that public boundary.

```rust
pub const TARGETS: &[Target] = &[
    Target::new("x86_64-unknown-linux-gnu", "x86_64.rs"),
    Target::new("aarch64-unknown-linux-gnu", "aarch64.rs"),
    Target::new("i686-pc-windows-msvc", "i686.rs"),
];
```

- [x] **Step 4: Generate, remove downstream bindgen, and prove a clang-free build**

Run: `cargo xtask ara bindings --write && cargo xtask ara bindings --check && cargo test -p xtask --test bindings && cargo test -p ara2-bridge-sys --test audio_file_chunk_constants && env -u LIBCLANG_PATH cargo check -p ara2-bridge-sys`  
Expected: PASS without executing bindgen in the package build; every public declaration is classified and the synthetic chunk constant is present.

- [ ] **Step 5: Commit pregenerated bindings**

```bash
git add -- ara2-bridge-sys/Cargo.toml ara2-bridge-sys/build.rs ara2-bridge-sys/ARAInterface.h ara2-bridge-sys/ARAAudioFileChunks.h ara2-bridge-sys/ARACLAP.h ara2-bridge-sys/ARAVST3.h ara2-bridge-sys/ARAAudioUnit.h ara2-bridge-sys/src/lib.rs ara2-bridge-sys/src/generated/mod.rs ara2-bridge-sys/src/generated/x86_64.rs ara2-bridge-sys/src/generated/aarch64.rs ara2-bridge-sys/src/generated/i686.rs ara2-bridge-sys/src/generated/audio_file_chunks.rs ara2-bridge-sys/generated/symbol-coverage.json ara2-bridge-sys/tests/audio_file_chunk_constants.rs xtask/src/bindings.rs xtask/src/lib.rs xtask/src/ara.rs xtask/tests/bindings.rs Cargo.lock
git commit -m "feat(sys): ship pregenerated ara bindings"
```

### Task 4: Generate layouts, unaligned accessors, and interface metadata

**Files:**
- Create: `ara2-bridge-sys/src/generated/layout.rs`
- Create: `ara2-bridge-sys/src/generated/access.rs`
- Create: `ara2-bridge-sys/src/generated/compatibility.rs`
- Create: `xtask/src/compatibility.rs`
- Modify: `xtask/src/lib.rs`
- Create: `xtask/tests/compatibility.rs`
- Modify: `ara2-bridge-sys/src/generated/mod.rs`
- Modify: `xtask/src/ara.rs`

- [x] **Step 1: Register a minimal checker and write failing freshness/manifest-join tests**

```rust
#[test]
fn generated_compatibility_metadata_is_current() {
    xtask::compatibility::generate(xtask::Mode::Check).unwrap();
}
```

First export `xtask::compatibility` from `xtask/src/lib.rs`; its initial check implementation only verifies the canonical generated path and reports its absence. After the red run, add the separate 54-slot/header-order semantic join test while implementing full generation.

- [x] **Step 2: Run and verify missing generated metadata**

Run: `cargo test -p xtask --test compatibility`  
Expected: the test compiles and FAILS naming `ara2-bridge-sys/src/generated/compatibility.rs`, not an unresolved module or function.

- [x] **Step 3: Generate field extents and safe packed-field access primitives**

Generate `implemented_size::<T>(field)` constants, target layout assertions, and accessors shaped as:

```rust
pub unsafe fn read_field<T: Copy>(base: *const u8, offset: usize) -> T {
    // SAFETY: caller guarantees `base..base+offset+size_of::<T>()` is readable.
    unsafe { std::ptr::read_unaligned(base.add(offset).cast::<T>()) }
}
```

Generate compatibility records with generation, terminal field, callback order, dependency group, and fallback enum. Do not infer behavior in `sys`; serialize the reviewed TOML values. Apply and validate the same complete generated-derivative provenance metadata used by raw bindings to `layout.rs`, `access.rs`, and `compatibility.rs`.

- [x] **Step 4: Run freshness and compile-time layout checks**

Run: `cargo xtask ara generate --write && cargo test -p xtask --test compatibility && cargo test -p ara2-bridge-sys`  
Expected: PASS with all 54 slots and every versioned surface represented.

- [ ] **Step 5: Commit generated metadata**

```bash
git add -- ara2-bridge-sys/src/generated/mod.rs ara2-bridge-sys/src/generated/layout.rs ara2-bridge-sys/src/generated/access.rs ara2-bridge-sys/src/generated/compatibility.rs xtask/src/compatibility.rs xtask/src/lib.rs xtask/src/ara.rs xtask/tests/compatibility.rs
git commit -m "feat(sys): generate ara layout and compatibility metadata"
```

### Task 5: Add C/C++ ABI probes

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `xtask/Cargo.toml`
- Modify: `ara2-bridge-sys/Cargo.toml`
- Create: `ara2-bridge-sys/tests/abi.rs`
- Create: `ara2-bridge-sys/tests/probe/ara_layout.c`
- Create: `ara2-bridge-sys/tests/probe/ara_core.cpp`
- Create: `ara2-bridge-sys/tests/generated/x86_64-core-abi.json`
- Create: `ara2-bridge-sys/tests/generated/aarch64-core-abi.json`
- Create: `ara2-bridge-sys/tests/generated/i686-core-abi.json`
- Create: `xtask/src/core_probe.rs`
- Modify: `xtask/src/lib.rs`
- Create: `xtask/tests/core_probe.rs`
- Modify: `xtask/src/ara.rs`

This task opts the workspace into exact maintainer envelope dependencies `tar = "0.4"` and
`zstd = "0.13"`; `ara2-bridge-sys` uses the existing workspace `serde_json = "1"` only as a
dev-dependency for cross-language assertions. The generated assertion table is checked in at
`ara2-bridge-sys/tests/generated/core_abi_assertions.rs`.

- [x] **Step 1: Register the probe API and write failing probe tests**

```rust
#[test]
fn document_controller_layout_matches_c() {
    assert_eq!(size_of::<ARADocumentControllerInterface>(), probe_size("ARADocumentControllerInterface"));
    assert_eq!(offset_of!(ARADocumentControllerInterface, storeObjectsToArchive), probe_offset("ARADocumentControllerInterface.storeObjectsToArchive"));
}
```

Before any probe generation, export a minimal `xtask::core_probe` API that checks the three canonical family paths, plus `--emit`, `--import-dir`, and `--check-all` command shells. `xtask/tests/core_probe.rs` must compile and report absent family artifacts rather than an unresolved module or command.

- [x] **Step 2: Verify both integration targets fail on absent artifacts**

Run: `cargo test -p xtask --test core_probe`  
Expected: FAIL listing the absent x86_64, AArch64, and i686 artifacts.  
Run: `cargo test -p ara2-bridge-sys --test abi`  
Expected: FAIL because the current target-family file under `tests/generated/{x86_64,aarch64,i686}-core-abi.json` is absent or empty.

- [x] **Step 3: Implement probes for every generated record**

`cargo xtask ara probe-core --emit <envelope> --target-family <family>` compiles and runs the C/C++ sources against only `ARAInterface.h` and `ARAAudioFileChunks.h` on that exact ABI, then emits an artifact envelope containing the deterministic JSON payload plus target triple, family, source/probe hashes, and payload hash. `--import-dir` rejects duplicate/missing families or any hash/family/source mismatch before atomically extracting all three canonical JSON files. `--check-all` validates their complete provenance metadata and deterministic contents. The probes export `sizeof`, alignment, and offsets for every versioned struct plus core constants/discriminants and the C++-only chunk constant. The Rust ABI test compares that C++ value to `audio_file_chunks::kARAXMLName_CreateDistinctAudioModification`, not merely to another probe field. Generate the probe table from the compatibility and symbol-coverage manifests so missing symbols fail compilation. VST3, Audio Unit, and CLAP probes belong to the companion phase after their pinned SDK inputs exist.

- [ ] **Step 4: Produce, collect, and validate every ABI-family artifact**

Run on a Linux x86_64 runner: `cargo xtask ara probe-core --emit target/abi-artifacts/x86_64.probe.tar.zst --target-family x86_64`  
Run on a native or system-emulated Linux AArch64 runner: `cargo xtask ara probe-core --emit target/abi-artifacts/aarch64.probe.tar.zst --target-family aarch64`  
Run on a Windows i686 runner: `cargo xtask ara probe-core --emit target/abi-artifacts/i686.probe.tar.zst --target-family i686`  
Collect those three runner artifacts without renaming them, then run in the task worktree: `cargo xtask ara probe-core --import-dir target/abi-artifacts && cargo xtask ara probe-core --check-all && cargo test -p xtask --test core_probe`  
Finally, run `cargo test -p ara2-bridge-sys --test abi` on each of the same three ABI families against the imported canonical files.  
Expected: PASS; all three payload/source hashes and family identities validate before import, every canonical file exists before commit, freshness is clean, provenance fields are complete, and unaligned property buffers are read only through generated accessors.

- [ ] **Step 5: Commit probes**

```bash
git add -- ara2-bridge-sys/tests/abi.rs ara2-bridge-sys/tests/probe/ara_layout.c ara2-bridge-sys/tests/probe/ara_core.cpp ara2-bridge-sys/tests/generated/x86_64-core-abi.json ara2-bridge-sys/tests/generated/aarch64-core-abi.json ara2-bridge-sys/tests/generated/i686-core-abi.json xtask/src/core_probe.rs xtask/src/lib.rs xtask/src/ara.rs xtask/tests/core_probe.rs
git commit -m "test(sys): verify ara abi with c and cpp probes"
```

### Task 6: Finish the sys public boundary and phase gate

**Files:**
- Modify: `ara2-bridge-sys/src/lib.rs`
- Modify: `ara2-bridge/src/lib.rs`
- Create: `xtask/tests/sys_boundary.rs`
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`
- Create: `docs/superpowers/handoffs/phase-0-abi.md`

- [x] **Step 1: Add compile checks for target selection and forbidden build dependencies**

```rust
#[test]
fn sys_package_has_no_build_time_bindgen() {
    let text = std::fs::read_to_string("ara2-bridge-sys/Cargo.toml").unwrap();
    assert!(!text.contains("[build-dependencies]"));
    assert!(!text.contains("bindgen"));
}
```

- [x] **Step 2: Expose only generated raw modules and provenance constants**

`lib.rs` selects the exact generated target family with `cfg`, re-exports raw symbols, documents packed access rules, and emits `compile_error!` for ARM32/unknown targets. Remove the old host-vtable builders and all claims that null callbacks inside a full struct are supported.

The existing facade still imported five build-time-generated host vtable helpers removed with
`ara2-bridge-sys/build.rs`. Replace that unused `0.1.x` implementation with the specified
aggregation-only facade so the Phase 0 workspace gate tests the final crate boundary. This is a
plan clarification, not a behavioral spec change: spec `00` already permits the unused API break
and defines the facade as aggregation-only.

- [x] **Step 3: Add CI jobs for generation freshness and ABI families**

Add `cargo xtask ara generate --check`, core probe execution, Linux x86_64, Linux AArch64 under a native runner or QEMU, Windows x86_64, macOS x86_64/AArch64, and i686 compile/probe coverage. Do not reference companion SDKs in this phase.

- [x] **Step 4: Run the phase gate**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps && cargo xtask ara generate --check && cargo xtask ara probe-core --check-all && cargo +1.82.0 check --workspace --all-targets --locked && env -u LIBCLANG_PATH cargo check --workspace && git diff --exit-code -- ara2-bridge-sys/src/generated sdk-provenance.toml`  
Expected: all commands PASS and `git diff --exit-code -- ara2-bridge-sys/src/generated sdk-provenance.toml` is clean.

- [x] **Step 5: Write the compact phase handoff**

Record final crate edges, generated artifact paths, supported target/generation sets, exact gate commands/results, and normative revisions already committed in this phase. The gate fails if any discovered normative revision remains pending; omit task history.

- [ ] **Step 6: Commit the phase gate**

```bash
git add -- ara2-bridge-sys/src/lib.rs ara2-bridge/src/lib.rs xtask/tests/sys_boundary.rs .github/workflows/ci.yml README.md docs/superpowers/handoffs/phase-0-abi.md
git commit -m "ci: gate generated ara abi across targets"
```
