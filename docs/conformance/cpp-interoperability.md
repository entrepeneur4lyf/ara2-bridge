# C++ Interoperability

## Scope and provenance

The `cpp-interop` testkit feature builds directly against the pinned Celemony ARA SDK 2.3 checkout (`ARA_SDK` `a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c`, API `65ec5c43b943a48cb5446f448a0492db6af8534b`, Library `d18a6a5e489816316be84a9de0eaf7307bc1abe4`, Examples `abd7c8aa5854591995e1fbf16f854c65b0998e8d`). It is disabled by default and does not affect portable builds or published runtime crates.

Run the gate with:

```bash
ci/bootstrap-reference-sdks.sh fetch --component ara --accept-license Apache-2.0
cargo test -p ara2-bridge-testkit --features cpp-interop --test cpp_interop -- --nocapture
```

## Pairings and scenarios

Both directions run at ARA 2.3 Final:

- Rust `TestHost` → Celemony C++ `ARATestDocumentController`.
- Celemony C++ `TestHost`/`TestCases` → the capability-rich Rust `TestPlugIn`.

The direct-factory matrix runs property updates, content updates, content reading and analysis, modification cloning, full archives, split/partial archives, drag/drop import, processing algorithms, audio-file chunk saving, and basic document creation. Each result records the selected generation, stable scenario name, observed callback count, bounded assertion/exception diagnostics, and post-teardown live-object count.

Playback rendering, time-stretch rendering, and editor-view scenarios require a companion-format processor instance rather than only an ARA factory; they are exercised by the CLAP, VST3, and Audio Unit v2 interoperability suites. Audio-file chunk loading is a container/XML decoder operation with no plug-in ABI call and remains in the deterministic Rust scenario suite. These exclusions are explicit and are not capability skips.

## Native boundary and cleanup

The shim exposes only C-compatible configuration/result records and factory pointers defined by ARA. C++ objects, STL types, and exceptions never cross into Rust. Every C entry catches all exceptions, ARA assertions are translated into bounded diagnostics, and a process-wide lock serializes the SDK's external assertion hook. Initialization is RAII-balanced; Rust sessions close leaf-first before factory uninitialization. A zero live-object result is reported only after those teardown checks complete.

## Implementation findings

Upstream execution corrected three interoperability rules: analysis-start notifications are now deferred to `notifyModelUpdates`; audio-file chunk callbacks return the exact factory-published archive-ID pointer; and note volume validation follows ARA's nonnegative (not unit-bounded) rule. The TestPlugIn fixture also publishes the two tempo points required by Celemony's content validator.
