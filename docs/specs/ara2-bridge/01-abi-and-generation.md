# ABI and Binding Generation

Status: Normative component specification  
Depends on: [System Overview](00-overview.md)  
Last revised: 2026-07-15

## Scope

This spec owns the exact Rust representation of the released ARA 2.3 C ABI. It does not define safe ownership or application behavior.

## Source provenance

The canonical input is `https://github.com/Celemony/ARA_SDK.git`, installed inside the consuming project by `scripts/install-ara-sdk.sh`. The script checks out top-level commit `a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c`, runs `git submodule update --init --recursive`, and records relocatable project-local SDK paths. `.third-party/ARA_SDK/ARA_API/` commit `65ec5c43b943a48cb5446f448a0492db6af8534b` (`releases/2.3.0`) is the canonical ABI input. The top-level commit is one README-only commit after the release tag; it does not alter normative code. Regeneration fails if the GitHub checkout is absent, dirty, at the wrong identity, or hashes differ unless the explicit SDK-update workflow is invoked.

Consumed headers are `ARAInterface.h`, `ARAAudioFileChunks.h`, `ARACLAP.h`, `ARAVST3.h`, and `ARAAudioUnit.h`. License and NOTICE files must accompany distributed source and generated artifacts.

## Generated artifacts

`ara2-bridge-sys` shall ship pregenerated bindings so downstream builds do not require clang or libclang. A pinned maintainer-only generator shall:

1. run bindgen with explicit target, language mode, allowlists, raw-integer enum policy, and ARA packing configuration;
2. normalize only deterministic formatting and header paths;
3. generate compile-time layout assertions, minimum-size constants, ordered function-slot metadata, safe unaligned field accessors, and interface-prefix constructors;
4. write a packaged machine-readable coverage manifest mapping every public declaration from all five ARA headers to its generated Rust symbol, audited companion shim symbol, explicit target/SDK-gated classification, or companion-deferred declaration that the final companion manifests must close; core headers are preprocessed in Phase 0, while companion headers are lexically inventoried without resolving their external includes and are compiled/preprocessed only after the corresponding pinned SDK is provisioned; and
5. compare output against checked-in artifacts in CI.

Coverage discovery is a source-inventory operation, not a host-ABI probe. Its preprocessing and AST passes therefore use the canonical x86_64 Linux target on every maintainer host; the separate per-family binding and C/C++ probe stages remain authoritative for target layout.

Handwritten code may wrap generated symbols but must not edit generated files. The generator itself is tested and versioned.

## Required ABI surface

The core output must contain all ARA basic types, raw integer enum aliases/constants, opaque reference types, property/event/filter structs, five host controller interfaces, document-controller host and plug-in instances, the ordered document-controller slot manifest, factory/configuration types, the deprecated plug-in extension plus playback renderer, editor renderer, and editor view interfaces, view selection, and extension instance. The 2.3 baseline has 54 callable document-controller slots plus `structSize`; the generated manifest, not that handwritten count, is authoritative.

All audio-file chunk XML constants are required. The generator synthesizes the Rust `createDistinctAudioModification` constant from the released C++ declaration because the released C branch accidentally omits its macro; checked-in generation metadata records that exception and a Rust-to-C++ probe proves exact value parity. Companion-specific symbols are generated or represented through audited shims as follows:

- CLAP: C ABI bindings generated directly from the pinned CLAP headers; no third-party Rust sys crate is part of the ABI chain.
- VST3: audited Rust COM/interface declarations or a thin C++ shim, because `ARAVST3.h` uses C++ VST3 types.
- Audio Unit: audited Core Audio property structures and constants, compiled only on Apple targets.

No companion header may silently disappear because an external SDK is unavailable; the corresponding feature must fail with an actionable build-time message or remain disabled.

## Versioned structs and generations

Every versioned struct exposes released minimum sizes and generated implemented-prefix sizes. A field is present only when `structSize >= offset_of(field) + size_of(field)`; impossible partial-field sizes are rejected. Providers set `structSize` to the last consecutively implemented field, never blindly to `size_of`. Consumers never assume the peer was compiled against 2.3 and apply the method-specific fallbacks in the compatibility spec.

ARA uses pack 1 on x86/x86_64 and pack 8 on AArch64. Generated artifacts are per ABI family. Packed fields are never accessed through Rust references; accessors use `read_unaligned`/`write_unaligned` or copy into aligned mirrors. `ARABool` is an integer with nonzero-as-true semantics. Raw enums remain integer aliases so unknown peer values cannot create invalid Rust discriminants; safe conversions validate known closed sets or preserve unknown values for non-exhaustive sets.

Cast- or macro-defined scalar constants that bindgen omits or widens are normalized by the generator to their declared C type. In particular, `kARAInvalidPitchNumber` is emitted as `ARAPitchNumber`, while `kARAInvalidFrequency` and `kARADefaultConcertPitchFrequency` remain `float`/Rust `f32`; compile-time type tests and C value probes cover these corrections.

Exact first-class targets are `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, and `aarch64-apple-darwin`. `i686-pc-windows-msvc` is tier 2 for generation-1 and 32-bit archive compatibility. ARM32 is unsupported by the released header. Deprecation attributes may be added to wrappers but may not remove ABI symbols available on that target.

## Verification

Required generated tests:

- `size_of`, `align_of`, and field-offset parity against a C/C++ probe compiled from the same headers;
- function-pointer signature checks for every interface slot;
- constant and discriminant parity;
- minimum-size and per-field availability tests;
- header-manifest and regeneration-diff checks;
- native checks on Linux x86_64, Windows x86_64, macOS x86_64, and macOS AArch64; Linux AArch64 runs the Rust conformance suite natively or under system emulation and executes the matching C probe; Windows i686 may use compile-plus-C-probe coverage with the limitation recorded in the release report.

Cross compilation may compile layout assertions, but at least one runner per pack/alignment family must execute the C probe. Generated tests include deliberately unaligned property and content-event buffers.

## Acceptance criteria

Downstream `cargo build` works without libclang. Regeneration from the pinned SDK is deterministic. The coverage manifest has no unclassified public ARA symbol. All layout probes pass, and the safe crates access raw declarations only through reviewed modules.

## Decisions and revisions

- 2026-07-14: Replace consumer-time bindgen with checked-in generated bindings and a maintainer regeneration tool.
- 2026-07-14: Permit thin audited companion shims where headers are not directly bindgen-compatible.
- 2026-07-14: Audit requires per-ABI packing and unaligned access; ordinary references to packed fields are forbidden.
- 2026-07-15: Audit made external SDK bootstrap, exhaustive all-symbol coverage, and the synthetic chunk constant explicit release artifacts.
- 2026-07-15: Implementation evidence requires generator normalization and compile-time type tests for bindgen-omitted or widened scalar macros.
- 2026-07-15: Coverage discovery uses one canonical preprocessing target so regeneration is byte-identical on Linux and macOS; target ABI evidence remains per-family.
- 2026-07-16: Install the official recursive ARA SDK checkout inside each consuming project and discover it through project-local Cargo configuration.
