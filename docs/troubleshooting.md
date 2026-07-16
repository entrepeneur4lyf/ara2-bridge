# Troubleshooting

This index is the manual-ready failure guide for development and conformance work. Start with the first failing command; do not bypass generation, ABI, provenance, or safety checks.

## General Diagnostics

Run `cargo check --workspace --all-targets`, then repeat the failing command without `--quiet`. Record the target triple, Rust version, enabled features, SDK commit, and complete bounded ARA diagnostic. A poisoned document or controller must be torn down; it cannot be recovered by retrying the same callback.

## SDK Configuration

Cargo builds never download SDKs. Run `bash scripts/install-ara-sdk.sh` from the consuming project, or download that script from the repository URL shown in the README. It installs into the project's `.third-party/` directory and writes relative `ARA_SDK_DIR`, `ARA_CLAP_DIR`, `ARA_VST3_SDK_DIR`, and, on macOS, `ARA_AUDIO_UNIT_SDK_DIR` entries to `.cargo/config.toml`. A conflicting entry, dirty checkout, wrong commit, missing compiler, or non-Apple Audio Unit feature is an intentional error.

## Generation Mismatch

Run `cargo xtask ara generate --check`, `cargo xtask ara probe --check-all`, and the applicable companion provenance/probe check. Regenerate only from the commits in `ci/reference-sdks.lock.toml`; never edit generated Rust, C, C++, JSON, or TOML by hand.

## Lifecycle and Ownership

Create and destroy model objects inside their document session, end edit/restore/render guards, revoke readers before source teardown, and close leaf objects before controllers. A stale, cross-document, or wrong-generation handle is rejected before FFI.

## Realtime Callbacks

The processing head/tail path must not allocate, block, access files, or log synchronously. Consume `RealtimeFailureQueue` from a non-realtime thread. Reproduce failures with `cargo test -p ara2-bridge-testkit --test realtime -- --nocapture`.

## Content and Persistence

Validate event ordering and field ranges before exposing a reader. For archives, preserve IDs and partial-store mappings. For audio-file chunks, verify the file container, chunk-size limits, XML namespaces, Base64 bounds, and the fixture SHA-256 before mutation.

## Companion Discovery

ARA binding must occur before activation, state load, processing-dependent extension use, or GUI creation. CLAP IDs, VST3 class names, and Audio Unit properties must resolve to the same factory pointer. The bridge supplies ARA adapters, not DSP, plug-in-format entry points, bundle metadata, registration, cache management, signing, or notarization.

## Native Conformance

Use the matching native runner; VST3 and Audio Unit runtime probes cannot be inferred by cross-compilation. The in-process C++ harness runs ARA 2.3 Final scenarios with a 30-second scenario timeout and requires zero capability skips for the release fixture. Preserve assertion text and teardown counters when reporting failures.

## Migration

Do not recreate the 0.1 raw-pointer `DocumentController` or vtable builders. Follow `docs/migration-0.1-to-0.2.md`, select focused features, and move ownership into `PluginBuilder`, `FactoryBuilder`, `HostServicesBuilder`, and session guards.
