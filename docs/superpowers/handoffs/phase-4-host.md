# Phase 4 Handoff — Host Runtime

Status: implementation gate complete  
Baseline: ARA API `releases/2.3.0`, normative commit `65ec5c43b943a48cb5446f448a0492db6af8534b`

## Implemented surface

`ara2-bridge-host` now provides stable audio, archive, content, model-update, and playback host services; checked factory loading and complete 54-slot controller dispatch; ARA1/ARA2 document graphs; edit, restoration, partial persistence, and explicit-close orchestration; typed source/modification/region content; analysis; processing algorithms; licensing; signal preservation; head/tail and audio-file chunk operations; and all renderer/editor/view extension roles.

The public `DocumentSession` owns graph handles and peer references. `EditSession` contains mutation and algorithm-selection operations. `ExtensionController` validates known/assigned roles and creates RAII assignments. `PluginContentReaderBackend` connects checked plug-in readers to core typed readers. `CloseError` preserves every teardown failure.

## Safety and lifecycle invariants

- Provisional records exist before create callbacks; failed creation poisons only if a reentrant host callback observed the provisional reference.
- Host and plug-in references remain separate identity domains. Dependent outbound properties contain translated peer references.
- Readers retain exclusive controller access and are revoked before graph teardown.
- ARA2 archive filters are pinned, archive IDs are preflighted, and multiple partial restores may share one edit.
- Extension assignments are removed before graph objects; later guard drops are inert.
- Processing catalogs and chunk metadata are copied. Indices, capability subsets, event extents, archive IDs, and realtime results are validated before exposure.

## Public TestHost and scenario

`ara2_bridge_testkit::TestHost` supplies deterministic capability-complete services and a sequence-numbered `TestHostTrace`. `scenarios::basic_document_smoke` uses only published APIs to verify factory initialization, two graph edit cycles, a real stereo sample read, requested analysis and three ordered progress callbacks, a typed note reader, processing selection, extension assignments, and controller-first plus companion-first teardown.

Workspace metadata proves `ara2-bridge-host` has no normal dependency on `ara2-bridge-plugin` or `ara2-bridge-testkit`; its testkit edge remains development-only.

## Completed gate evidence

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` — 165 tests across 67 suites
- `cargo test -p xtask --test workspace`
- `cargo test -p ara2-bridge-testkit --test rust_interop`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
- strict-provenance Miri: services builder, audio access, host callback manifest, plug-in dispatch, document graph, restoration, extensions, content/processing, and Rust interoperability scenario

All host phase checks pass. No discovered normative revision remains undocumented.

## Revisions discovered during implementation

- Stable host service owners and callback backing must remain at fixed addresses after publication under strict provenance.
- Provisional creation needs explicit reentrant-observation tracking rather than unconditional rollback assumptions.
- Typed content readers require an exclusive document-controller borrow for their full foreign reader lifetime.
- Successful audio-file chunk storage must copy and validate the returned archive ID against factory metadata.
- Extension teardown retained typed raw pointers; integer-to-pointer reconstruction was rejected by strict-provenance Miri.
