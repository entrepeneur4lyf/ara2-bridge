# Safety and Concurrency Conformance

This bridge treats foreign ARA storage as caller-valid memory whose contents still require strict validation. Safe Rust cannot determine whether an arbitrary address is readable. `SizedInput`, content-event decoders, archive filters, and reference registries therefore reject malformed values only after the caller has supplied readable storage for the documented extent.

## Realtime Callback Contract

`getPlaybackRegionHeadAndTailTime` reads an immutable, atomically selected snapshot and reports failures through a preallocated bounded queue. The conformance test instruments allocations and audits the designated callback implementation for blocking locks, file I/O, and synchronous logging. Model-thread snapshot replacement may allocate and lock; the realtime query may not.

Run:

```bash
cargo test -p ara2-bridge-testkit --test realtime
```

## Concurrency and Teardown

Deterministic models cover reader revocation, analysis cancellation, render activation, editor updates, and both controller/companion teardown orders. Production integrations additionally exercise concurrent analysis cancellation, an in-flight host audio read during synchronous revocation, and editor/view assignment plus teardown scenarios. Every integration asserts real callbacks and cleanup behavior.

```bash
ci/run-sanitizers.sh tsan-state-models
ci/run-sanitizers.sh tsan-production
```

The TSan lanes use nightly `-Zbuild-std` so the standard library and all dependencies share the instrumented ABI.

## Invalid Foreign Pointers

Caller-valid malformed records run in the ordinary test suite and must return a typed error. Null-adjacent, inaccessible-page, and guard-page addresses run only in isolated sanitizer children after `ARA2_BRIDGE_ALLOW_INVALID_POINTER_CASE=1` is set by the harness.

```bash
cargo test -p ara2-bridge-testkit --test invalid_pointer_subprocess
ci/run-sanitizers.sh asan-invalid-pointer
ci/run-sanitizers.sh ubsan-invalid-pointer
```

Rust nightly has no UndefinedBehaviorSanitizer mode. The UBSan lane therefore instruments the C foreign-caller side of the pointer-readability contract; the Rust subprocess separately verifies safe rejection of readable malformed contents. A sanitizer report or signal-classified nonzero exit is required for every deliberately unreadable case.

## Fuzzing and Miri

`fuzz/corpus-manifest.toml` binds every reviewed seed to its target, semantic class, source path/repository, license, and source/output SHA-256. `cargo xtask ara fuzz-corpus --check` rejects missing, stale, empty, unlicensed, or unexpected named seeds. SHA-named files emitted by libFuzzer are transient discoveries, ignored by Git, and are not reviewed corpus inputs.

All eight fuzz targets bound input or parser allocation and call production validators. Execute smoke runs with nightly, for example:

```bash
cargo +nightly fuzz run content_events -- -max_total_time=30
```

Miri covers the explicit core, plug-in, and host suites listed in the conformance delivery plan; subprocess and native-SDK tests remain outside Miri.
