# Phase 0 Handoff: ABI and Workspace

Status: complete locally; target-native CI remains the merge authority  
Baseline: ARA SDK `releases/2.3.0` / ARA API
`65ec5c43b943a48cb5446f448a0492db6af8534b`

## Boundary

The workspace dependency direction is:

```text
sys <- core <- plugin / host <- companion
                  \         /
                   testkit

facade -> sys, core, plugin, host, companion, optional testkit
```

`testkit` never depends on the facade, and no focused production crate depends on `testkit`.
The facade is aggregation-only. `ara2-bridge-sys` contains raw generated ABI, generated packed
access/compatibility metadata, and source provenance constants; it contains no behavioral vtable
builders, build script, copied SDK headers, or build dependency on bindgen.

## Generated and Audited Inputs

- `ci/reference-sdks.lock.toml` and `ci/bootstrap-reference-sdks.sh` provision immutable ignored
  SDK inputs with explicit license acceptance.
- `sdk-provenance.toml` hashes every consumed ARA source and verifies clean Git identities.
- `ara2-bridge-sys/src/generated/{x86_64,aarch64,i686}.rs` are the target-family raw bindings.
- `access.rs`, `layout.rs`, and `compatibility.rs` encode unaligned access, 232 field extents, and
  the reviewed generation-prefix/callback rules.
- `ara2-bridge-sys/generated/symbol-coverage.json` classifies 547 declarations: 498 core ABI and
  49 companion-deferred declarations. Companion phases must close the latter against their pinned
  SDKs before making support claims.
- `ara2-bridge-sys/tests/generated/*-core-abi.json` records independently compiled C11/C++17
  evidence for 33 complete structs per family and 74/71/74 constants for
  x86_64/AArch64/i686 respectively.

All generated derivatives carry repository, release tag, API commit, generator version, SPDX
license, and `DO NOT EDIT` metadata. `cargo xtask ara generate --check` is non-mutating.

## Target and Generation Set

Raw bindings are selected for x86_64, AArch64, and 32-bit x86; ARM32 and unknown architectures
fail at compile time. CI covers Linux x86_64/AArch64, Windows x86_64/i686, and macOS
x86_64/AArch64. Generation availability remains defined by the compatibility manifest:

- x86/i686 and x86_64: 1.0 Draft, 1.0 Final, 2.0 Draft, 2.0 Final, 2.X Draft, 2.3 Final.
- AArch64: 2.0 Final, 2.X Draft, and 2.3 Final; legacy constants are not synthesized.

Phase 0 proves representation and compatibility metadata only. Safe generation policy and runtime
behavior are Phase 1 onward responsibilities.

## Gate Evidence

The following commands passed on the local x86_64 Linux checkout on 2026-07-15:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                         # 15 passed; 26 suites
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo xtask ara generate --check
cargo xtask ara probe-core --check-all
cargo +1.82.0 check --workspace --all-targets --locked
env -u LIBCLANG_PATH cargo check --workspace
git diff --exit-code -- ara2-bridge-sys/src/generated sdk-provenance.toml
```

The x86_64 C/C++ probe executed natively. AArch64 C/C++ and Rust evidence executed under QEMU;
i686 C/C++ evidence executed under Wine, and the exact MSVC Rust target type-checked locally.
The workflow executes AArch64 probes on a native Linux runner and the i686 Rust ABI assertions on
Windows, which is the authoritative runtime evidence for those target families.

## Normative Revisions Closed

- Probe envelopes use workspace-pinned `tar 0.4` and `zstd 0.13` for deterministic transport.
- `tempfile` is pinned to `3.14.0`; `indexmap 2.7.0` and `jobserver 0.1.32` remain locked so Cargo
  and rustc 1.82 can resolve and build the maintainer workspace.
- The stale 0.1 facade implementation was removed because it depended on deleted build-time sys
  helpers; the facade now matches the already-audited aggregation-only architecture.
- Raw nullable callback types are preserved as C representation. Every callback inside an exposed
  safe interface prefix must be non-null; optionality is expressed by a shorter prefix or the
  manifest's explicit semantic fallback.
- The generator emits the cast-style C macros `kARAFalse`/`kARATrue` explicitly and exposes each
  record's field extents as a declaration-ordered slice so safe validation does not duplicate ABI
  facts by hand.
- The three canonical C/C++ probe envelopes were regenerated after adding those boolean constants;
  their inventories and hashes now cover the synthetic Rust declarations on every ABI family.
- The generator also restores the omitted `kARAInvalidPitchNumber` macro and corrects bindgen's
  widened `float` pitch-frequency constants to Rust `f32`; compile-time type tests cover all three.

No discovered Phase 0 normative revision is pending.
