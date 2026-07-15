# Phase 1 Handoff: Core Safety

Status: complete locally; CI and target-native jobs remain the merge authority  
Baseline: Phase 0 ABI handoff plus ARA API generation manifest `api-compatibility.toml`

## Public boundary

`ara2-bridge-core` now provides the shared safe substrate consumed by later host and plug-in phases:

- `error` and `diagnostics`: non-exhaustive `AraError`, archive/companion errors, contextual diagnostics, pluggable sinks, and a bounded poison-tolerant sink.
- `generation` and `assertions`: all six API generations, target availability checks, and process-wide reference-counted ARA assert cells with balanced factory initialization guards.
- `handles` and `registry`: typed `Handle<K>`/`ModelRef<K>` values and bounded, append-only registries with stable opaque addresses and permanent tombstones. A Phase 2 filter audit added an explicit `RegistrySession` shared by sibling typed registries in one document.
- `ffi`: canonical `AraBool`, sealed generated `SizedRecord` metadata, unaligned sized-structure readers, checked copied foreign slices, and bounded owned strings/persistent IDs.
- `properties`: owned document/model/selection property mirrors with stable backing, exact generation prefixes, checked references, strings, colors, numeric values, arrays, and channel layouts.
- `threading`, `poison`, and `lifecycle`: model-thread identity, first-failure poisoning, and scoped editing, restoration, sample-access, content-call, render-activation, and teardown guards.
- `realtime`: immutable sorted head/tail snapshots and preallocated bounded failure queues.
- `dispatch`: common panic containment, diagnostic context, poisoning, and signature-specific sentinel mapping for future generated callbacks.

## Safety invariants

Foreign-pointer APIs require caller-valid readable/writable storage for the documented extent. They then validate nullability, counts, overflow, minimum and complete-field sizes, alignment where required, UTF-8/ASCII rules, references, enums, and finite numbers before dependent reads. Packed structure fields use generated extents and unaligned copies.

Outbound properties own all transitive backing, keep exposed pointers stable, initialize every byte, and advertise only an implemented generation prefix. Companion-defined variable channel layouts are rejected until a companion adapter supplies a validated extent; core never guesses or retains their borrowed storage.

Handles are intentionally neither `Send` nor `Sync`. Registries reject stale, foreign, wrong-kind, duplicate-destroy, and over-capacity operations and never reuse a cell within a session. No panic crosses `extern "C"`; panics record the first diagnostic, poison the runtime, and return the method sentinel. Realtime queries allocate zero memory and touch no mutable model-thread state.

## Generated inputs

Core consumes `ara2-bridge-sys` raw target bindings plus generated `access`, `layout`, and `compatibility` metadata. Phase 1 extended generation with explicit `kARAFalse`/`kARATrue` declarations and ordered field-extent slices for all 25 versioned records. A Phase 2 decoder audit additionally restored `kARAInvalidPitchNumber` and the C `float` types of both pitch-frequency constants. The C/C++ ABI envelopes now prove 33 structures and 74/71/74 constants for x86_64/AArch64/i686. Generated files remain maintainer-only derivatives; package builds require neither Clang nor an SDK checkout.

## Gate evidence

The following passed on local x86_64 Linux on 2026-07-15:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                              # 55 passed; 37 suites
cargo test -p ara2-bridge-core --test clippy_ui    # 2 passed
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo +1.82 check --workspace --all-targets --locked
for suite in diagnostics generation registry ffi_validation properties lifecycle realtime dispatch; do cargo +nightly miri test -p ara2-bridge-core --test "$suite"; done
cargo xtask ara generate --check
cargo xtask ara probe-core --check-all
env -u LIBCLANG_PATH cargo check --workspace
git diff --check
```

The eight Miri suites passed 33 focused tests. Trybuild proves model handles are not `Send`; isolated negative Clippy fixtures prove missing safety docs and undocumented unsafe blocks fail policy. The native ABI test passed after x86_64, Wine/i686, and QEMU/AArch64 C/C++ probe execution.

## Closed revisions and next-phase constraints

- `proptest = 1.5.0` and `trybuild = 1.0.101` are exact development pins compatible with Cargo/rustc 1.82; the Phase 0 lock constraints remain in force.
- Specification `02` and the active plan now explicitly defer companion-sized CoreAudio and CLAP ambisonic layout copying rather than guessing an extent. The focused spec and plan re-audits are `CLEAR`.
- Phase 2 must build typed content and persistence on these owned property mirrors and lifecycle guards. It must not duplicate raw ABI facts, bypass `DispatchRuntime`, make model handles cross-thread, or weaken the caller-valid foreign-storage contract.

No discovered Phase 1 normative revision is pending.
