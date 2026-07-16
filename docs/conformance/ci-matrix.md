# Automated Validation Matrix

The pull-request workflow carries fast, portable checks. Native SDK interoperability is isolated so its exact SDK identities and licenses are visible. Scheduled safety work covers the slower dynamic-analysis and supply-chain gates. These workflows validate commits; they do not construct or publish releases.

Every job writes one schema-validated JSON fragment and uploads it independently. A fragment records the repository, head SHA, workflow/run/job identity, target, toolchain, exact command summary, conclusion, and SHA-256 maps for existing inputs and outputs. The local manual release procedure may inspect these fragments as additional evidence, but CI never packages, attests, signs, uploads, or publishes a release artifact.

| Workflow | Job | Coverage |
| --- | --- | --- |
| `ci.yml` | `quality` | Format, stable build/test/lint/docs, generated ABI freshness, C/C++ probes |
| `ci.yml` | `msrv` | Rust 1.82 locked workspace |
| `ci.yml` | `runtime-matrix` | Linux x86_64/AArch64, Windows x86_64, macOS x86_64/AArch64 |
| `ci.yml` | `i686-archive` | Executed 32-bit oversized-archive refusal |
| `ci.yml` | `phase0-core-probe` | Regression lock for provenance and native core-probe artifacts |
| `native-conformance.yml` | four native jobs | C++ ARA, CLAP, configured VST3, and Apple AUv2 |
| `safety.yml` | four safety jobs | Miri, sanitizers, eight fuzz targets, dependency/minimum/feature checks |

The block below is parsed by `cargo xtask ci validate`; keep it synchronized with the table and workflows.

<!-- ci-matrix
enforce_policy = true

[[job]]
workflow = "ci.yml"
id = "quality"
required = ["cargo xtask ara generate --check", "cargo xtask ara probe-core --check-all", "cargo xtask ara companion-probe clap --check-all", "cargo fmt --all --check", "cargo check -p ara2-bridge --no-default-features", "--features clap,testkit", "cargo clippy --workspace --all-targets -- -D warnings", "cargo test --workspace", "cargo doc --workspace --no-deps"]

[[job]]
workflow = "ci.yml"
id = "msrv"
required = ["1.82.0", "cargo check --workspace --all-targets --locked"]

[[job]]
workflow = "ci.yml"
id = "runtime-matrix"
evidence_count = 5
required = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "x86_64-apple-darwin", "aarch64-apple-darwin", "cargo test --workspace --target"]

[[job]]
workflow = "ci.yml"
id = "i686-archive"
required = ["i686-pc-windows-msvc", "cargo test --target i686-pc-windows-msvc -p ara2-bridge-core --test archive archive_larger_than_address_space_is_rejected"]

[[job]]
workflow = "ci.yml"
id = "phase0-core-probe"
required = ["bootstrap-reference-sdks.sh fetch --component ara --accept-license Apache-2.0", "cargo xtask ara provenance --check", "cargo xtask ara probe-core --check-all", "cargo xtask ara probe-core --emit"]

[[job]]
workflow = "native-conformance.yml"
id = "cpp-interop"
evidence_count = 5
required = ["cpp-interop", "https://github.com/Celemony/ARA_SDK.git", ".third-party/ARA_SDK", "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "x86_64-apple-darwin", "aarch64-apple-darwin"]

[[job]]
workflow = "native-conformance.yml"
id = "clap-conformance"
evidence_count = 5
required = ["--component clap --accept-license MIT", "companion-probe clap --check-target", "clap_abi", "clap_interop", "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "x86_64-apple-darwin", "aarch64-apple-darwin"]

[[job]]
workflow = "native-conformance.yml"
id = "vst3-conformance"
evidence_count = 5
required = ["--component vst3 --accept-license MIT", "companion-probe vst3 --check-target", "vst3_abi", "vst3_interop", "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "x86_64-apple-darwin", "aarch64-apple-darwin"]

[[job]]
workflow = "native-conformance.yml"
id = "audio-unit-conformance"
evidence_count = 2
required = ["--component audio-unit --accept-license Apache-2.0", "companion-probe audio-unit-v2 --check-target", "audio_unit_interop", "x86_64-apple-darwin", "aarch64-apple-darwin"]

[[job]]
workflow = "safety.yml"
id = "miri"
required = ["cargo miri test", "state_models", "model_graph", "audio_access", "restoration"]

[[job]]
workflow = "safety.yml"
id = "sanitizers"
evidence_count = 4
required = ["asan-invalid-pointer", "ubsan-invalid-pointer", "tsan-state-models", "tsan-production", "ci/run-sanitizers.sh"]

[[job]]
workflow = "safety.yml"
id = "fuzz"
evidence_count = 8
required = ["cargo-fuzz --version 0.13.2", "versioned_structs", "references", "content_events", "archive_filters", "audio_file_chunks", "audio_file_xml", "audio_file_container", "dispatch", "-max_total_time=30"]

[[job]]
workflow = "safety.yml"
id = "supply-chain-and-features"
required = ["cargo-audit --version 0.22.1", "cargo-deny --version 0.20.2", "cargo audit", "cargo deny check licenses sources", "-Z minimal-versions", "cargo check -p ara2-bridge --no-default-features", "--features clap,testkit"]

-->

## Local validation

Run `cargo xtask ci validate` to enforce the exact 13-job validation matrix, immutable Action revisions, explicit SDK license acceptance, evidence emission/upload, required command tokens, and absence of CI release authority. Run `cargo xtask ci list-jobs` for the stable canonical job list. The validator performs structural checks; GitHub runner availability and native SDK compilation remain execution evidence, not inferred local results.
