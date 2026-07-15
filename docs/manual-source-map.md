# Manual Source Map

This file is the durable input inventory for the future user manual. The prose headings describe chapter intent; the embedded TOML is normative and is checked by `cargo xtask docs verify-manual-map`. `not-applicable:` values are explicit boundaries, not missing work.

```toml manual-source-map
schema = 1

[[chapter]]
number = 1
title = "ARA concepts and the host, plug-in, and model graph"
normative_specs = ["docs/specs/ara2-bridge/00-overview.md", "docs/specs/ara2-bridge/02-core-safety-and-dispatch.md"]
public_apis = ["ara2_bridge::core::ApiGeneration", "ara2_bridge::core::Handle", "ara2_bridge::plugin::PluginBuilder", "ara2_bridge::host::DocumentSession"]
examples = ["ara2-bridge/examples/minimal-plugin.rs", "ara2-bridge/examples/minimal-host.rs"]
conformance_commands = ["cargo test -p ara2-bridge-testkit --test rust_interop -- --nocapture"]
testhost_args = ["in-process: generation=V23Final; scenario=basic_document_smoke; --nocapture"]
companion_binaries = ["not-applicable: concepts use the direct ARA factory path"]
sdk_environment = ["not-applicable: pregenerated core ARA bindings need no SDK variable"]
required_capabilities = ["ARA generation 2.3 Final", "audio access", "archiving", "model updates", "content access", "playback"]
expected_skips = 0
fixture_hashes = ["not-applicable: the basic graph example creates its model in memory"]
platform_steps = ["not-applicable: direct ARA factory construction is portable"]
gui_main_loop = ["not-applicable: the concepts scenario has no editor view"]
timeouts = ["30 seconds per TestHost scenario"]
troubleshooting = ["docs/troubleshooting.md#general-diagnostics", "docs/troubleshooting.md#lifecycle-and-ownership"]

[[chapter]]
number = 2
title = "Installation, features, targets, and SDK licensing"
normative_specs = ["docs/specs/ara2-bridge/06-companion-integrations.md", "docs/specs/ara2-bridge/08-packaging-versioning-and-manual.md"]
public_apis = ["ara2_bridge::core", "ara2_bridge::plugin", "ara2_bridge::host", "ara2_bridge::companion", "ara2_bridge::testkit"]
examples = ["ara2-bridge/examples/minimal-plugin.rs"]
conformance_commands = ["cargo test -p ara2-bridge --test features", "cargo +1.82.0 check --workspace --all-targets --locked", "ci/bootstrap-reference-sdks.sh verify --component ara"]
testhost_args = ["not-applicable: feature compilation does not launch TestHost"]
companion_binaries = ["target/debug/examples/clap-binding", "target/debug/examples/vst3-binding", "target/debug/examples/audio-unit-v2-binding"]
sdk_environment = ["ARA_SDK_DIR=$PWD/reference/ARA_SDK", "ARA_CLAP_DIR=$PWD/.third-party/clap", "ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk", "ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK"]
required_capabilities = ["Rust 1.82 or newer", "C++17 compiler for VST3", "Apple SDK and Objective-C++ compiler for Audio Unit v2"]
expected_skips = 0
fixture_hashes = ["not-applicable: feature checks consume locked source commits, not media fixtures"]
platform_steps = ["CLAP and VST3 builds run on Linux, Windows, and macOS; Audio Unit v2 builds only on macOS", "accept GPL-3.0-only or a documented proprietary policy before provisioning VST3"]
gui_main_loop = ["not-applicable: compilation does not create a format GUI"]
timeouts = ["10 minutes per clean SDK-backed feature build"]
troubleshooting = ["docs/troubleshooting.md#sdk-configuration", "docs/troubleshooting.md#generation-mismatch"]

[[chapter]]
number = 3
title = "Plug-in quick start and factory configuration"
normative_specs = ["docs/specs/ara2-bridge/03-plugin-runtime.md"]
public_apis = ["ara2_bridge::plugin::PluginBuilder", "ara2_bridge::plugin::FactoryBuilder", "ara2_bridge::plugin::PluginRegistryBuilder"]
examples = ["ara2-bridge/examples/minimal-plugin.rs"]
conformance_commands = ["cargo run -p ara2-bridge --example minimal-plugin --features plugin", "cargo test -p ara2-bridge-plugin --test factory"]
testhost_args = ["in-process: generation=V23Final; scenario=create_basic_document; --nocapture"]
companion_binaries = ["not-applicable: quick start publishes a direct ARAFactory"]
sdk_environment = ["not-applicable: plug-in authoring uses pregenerated ARA bindings"]
required_capabilities = ["factory ID", "document archive ID", "plug-in display metadata", "document-controller constructor for production use"]
expected_skips = 0
fixture_hashes = ["not-applicable: factory construction has no external fixture"]
platform_steps = ["retain the PluginRegistry and every published ARAFactory for dynamic-library lifetime"]
gui_main_loop = ["not-applicable: factory construction occurs before editor creation"]
timeouts = ["30 seconds for the basic document scenario"]
troubleshooting = ["docs/troubleshooting.md#lifecycle-and-ownership", "docs/troubleshooting.md#general-diagnostics"]

[[chapter]]
number = 4
title = "Document editing, analysis, content, and rendering"
normative_specs = ["docs/specs/ara2-bridge/03-plugin-runtime.md", "docs/specs/ara2-bridge/05-content-persistence-and-utilities.md"]
public_apis = ["ara2_bridge::plugin::PluginRuntime", "ara2_bridge::plugin::EditSession", "ara2_bridge::core::ContentReader", "ara2_bridge::plugin::RenderAssignment"]
examples = ["ara2-bridge/examples/content-reader.rs"]
conformance_commands = ["cargo run -p ara2-bridge --example content-reader", "cargo test -p ara2-bridge-testkit --test upstream_scenarios -- --nocapture"]
testhost_args = ["in-process: generation=V23Final; scenarios=update_content,read_content_analysis,playback_renderer,playback_renderer_transformations; --nocapture"]
companion_binaries = ["target/debug/examples/clap-binding"]
sdk_environment = ["ARA_CLAP_DIR=$PWD/.third-party/clap"]
required_capabilities = ["content provider", "analysis progress", "playback renderer role", "time-stretch transformations for transformed rendering"]
expected_skips = 0
fixture_hashes = ["ara2-bridge-testkit/fixtures/scenarios/ara2-full.archive@3c0cfb45fc5dab202d26a16bfb5788af87d88d5c6569621e78c4434542f50c9f"]
platform_steps = ["not-applicable: CLAP adapter tests run without installing a loadable bundle"]
gui_main_loop = ["editor-view scenarios require the integrating format's main-thread event loop; content and playback scenarios do not"]
timeouts = ["30 seconds per scenario", "5 seconds for analysis completion in the deterministic fixture"]
troubleshooting = ["docs/troubleshooting.md#content-and-persistence", "docs/troubleshooting.md#realtime-callbacks"]

[[chapter]]
number = 5
title = "Persistence, partial archives, and audio-file chunks"
normative_specs = ["docs/specs/ara2-bridge/05-content-persistence-and-utilities.md"]
public_apis = ["ara2_bridge::core::MemoryArchive", "ara2_bridge::core::StoreFilterBuilder", "ara2_bridge::core::RestoreFilterBuilder", "ara2_bridge::core::AraChunkSet", "ara2_bridge::core::replace_ara_in_path"]
examples = ["ara2-bridge/examples/archive-roundtrip.rs", "ara2-bridge/examples/audio-file-chunk.rs"]
conformance_commands = ["cargo run -p ara2-bridge --example archive-roundtrip", "cargo run -p ara2-bridge --example audio-file-chunk", "cargo test -p ara2-bridge-testkit --test upstream_scenarios -- --nocapture"]
testhost_args = ["in-process: generation=V23Final; scenarios=store_complete_document,store_restore_objects,import_audio_source,save_audio_file_chunk; --nocapture"]
companion_binaries = ["not-applicable: persistence and chunk storage are core ARA paths"]
sdk_environment = ["not-applicable: fixtures and pregenerated bindings are self-contained"]
required_capabilities = ["persistence", "partial persistence", "audio-file chunk storage"]
expected_skips = 0
fixture_hashes = ["ara2-bridge-testkit/fixtures/scenarios/ara2-partial-a.archive@bc89ff51343e8c335496648d7736cf2cd4ab1910320fe1cb6a0197b86b3a5a16", "ara2-bridge-testkit/fixtures/scenarios/chunk-wave.wav@88dbe314538135405b47393dfac1d0bee801ffb463e658031f6d17fefecdbe53", "ara2-bridge-testkit/fixtures/scenarios/chunk-aiff.aiff@090db617e82f530df694d889da5ea084b4a52235dc39ebabd4d4fae0fa401cfa"]
platform_steps = ["copy media before running the path-mutating example; replacement is atomic but intentionally changes the selected file"]
gui_main_loop = ["not-applicable: persistence and chunk operations have no GUI requirement"]
timeouts = ["30 seconds per TestHost scenario", "5 seconds per bounded chunk parse"]
troubleshooting = ["docs/troubleshooting.md#content-and-persistence", "docs/troubleshooting.md#generation-mismatch"]

[[chapter]]
number = 6
title = "Host quick start, discovery, model graph, and services"
normative_specs = ["docs/specs/ara2-bridge/04-host-runtime.md"]
public_apis = ["ara2_bridge::host::HostServicesBuilder", "ara2_bridge::host::LoadedFactory", "ara2_bridge::host::DocumentSession", "ara2_bridge::host::EditSession"]
examples = ["ara2-bridge/examples/minimal-host.rs"]
conformance_commands = ["cargo run -p ara2-bridge --example minimal-host --no-default-features --features host", "cargo test -p ara2-bridge-host --test document_graph"]
testhost_args = ["in-process: generation=V23Final; scenario=basic_document_smoke; services=all; --nocapture"]
companion_binaries = ["not-applicable: the quick start loads a direct ARA factory"]
sdk_environment = ["not-applicable: host runtime uses pregenerated ARA bindings"]
required_capabilities = ["audio access", "archiving", "optional content access", "optional model update", "optional playback"]
expected_skips = 0
fixture_hashes = ["not-applicable: minimal host advertises empty in-memory services"]
platform_steps = ["discover and load the integrating plug-in format before constructing LoadedFactory"]
gui_main_loop = ["host service callbacks follow their documented model or realtime thread; quick start has no GUI"]
timeouts = ["30 seconds for the basic document scenario"]
troubleshooting = ["docs/troubleshooting.md#lifecycle-and-ownership", "docs/troubleshooting.md#native-conformance"]

[[chapter]]
number = 7
title = "CLAP, VST3, Audio Unit v2, and format boundaries"
normative_specs = ["docs/specs/ara2-bridge/06-companion-integrations.md"]
public_apis = ["ara2_bridge::companion::CompanionProcessorBinding", "ara2_bridge::companion::clap::ClapAraEntry", "ara2_bridge::companion::vst3::Vst3MainFactoryAdapter", "ara2_bridge::companion::audio_unit::AudioUnitPluginAdapter"]
examples = ["ara2-bridge/examples/clap-binding.rs", "ara2-bridge/examples/vst3-binding.rs", "ara2-bridge/examples/audio-unit-v2-binding.rs"]
conformance_commands = ["cargo run -p ara2-bridge --example clap-binding --features plugin,clap", "ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk cargo run -p ara2-bridge --example vst3-binding --features plugin,vst3", "ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK cargo run -p ara2-bridge --example audio-unit-v2-binding --features plugin,audio-unit-v2"]
testhost_args = ["in-process: generation=V23Final; scenarios=playback_renderer,playback_renderer_transformations,editor_renderer_view; known_roles=all; assigned_roles=fixture-selected; --nocapture"]
companion_binaries = ["target/debug/examples/clap-binding", "target/debug/examples/vst3-binding", "target/debug/examples/audio-unit-v2-binding"]
sdk_environment = ["ARA_CLAP_DIR=$PWD/.third-party/clap", "ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk", "ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK"]
required_capabilities = ["stable factory identity", "one-shot role-aware binding", "processor-owned DSP and state", "format-owned GUI"]
expected_skips = 0
fixture_hashes = ["not-applicable: companion ABI fixtures are canonical probe JSON rather than audio media"]
platform_steps = ["CLAP: package the integrating format's loadable bundle and trigger its host-specific rescan", "VST3: package the integrating module, then apply platform signing/notarization and host cache refresh required by its distributor", "Audio Unit v2: embed the three ARA properties in the AUBase subclass, register the component bundle, sign it, and validate it with the target host", "AAX and Audio Unit v3 have no supplied public companion adapter and remain outside this release"]
gui_main_loop = ["binding precedes view creation; the integrating format owns the platform main loop and GUI thread"]
timeouts = ["30 seconds per companion interoperability test", "10 minutes for a clean native SDK-backed compile"]
troubleshooting = ["docs/troubleshooting.md#companion-discovery", "docs/troubleshooting.md#sdk-configuration"]

[[chapter]]
number = 8
title = "Threading, realtime safety, ownership, and teardown"
normative_specs = ["docs/specs/ara2-bridge/02-core-safety-and-dispatch.md", "docs/specs/ara2-bridge/03-plugin-runtime.md", "docs/specs/ara2-bridge/04-host-runtime.md"]
public_apis = ["ara2_bridge::core::ModelThread", "ara2_bridge::core::Lifecycle", "ara2_bridge::core::RealtimeFailureQueue", "ara2_bridge::plugin::RealtimeProcessContext"]
examples = ["ara2-bridge/examples/minimal-plugin.rs", "ara2-bridge/examples/minimal-host.rs"]
conformance_commands = ["cargo test -p ara2-bridge-testkit --test realtime -- --nocapture", "cargo test -p ara2-bridge-testkit --test analysis_concurrency -- --nocapture", "cargo test -p ara2-bridge-testkit --test sample_access_concurrency -- --nocapture"]
testhost_args = ["in-process: generation=V23Final; scenario=basic_document_smoke; teardown=controller-first-and-companion-first; --nocapture"]
companion_binaries = ["not-applicable: safety tests instrument the runtime and callback paths directly"]
sdk_environment = ["not-applicable: portable safety tests use pregenerated bindings"]
required_capabilities = ["model-thread token", "bounded registries", "nonblocking realtime failure queue", "explicit teardown guards"]
expected_skips = 0
fixture_hashes = ["not-applicable: state models generate deterministic operation traces"]
platform_steps = ["run TSan on its dedicated Linux runner; do not infer race freedom from a normal test run"]
gui_main_loop = ["all model mutation remains on the model thread; realtime callbacks never marshal synchronously to a GUI loop"]
timeouts = ["30 seconds per concurrency/realtime suite", "120 seconds per sanitizer case"]
troubleshooting = ["docs/troubleshooting.md#realtime-callbacks", "docs/troubleshooting.md#lifecycle-and-ownership"]

[[chapter]]
number = 9
title = "Errors, assertions, diagnostics, validation, and troubleshooting"
normative_specs = ["docs/specs/ara2-bridge/02-core-safety-and-dispatch.md", "docs/specs/ara2-bridge/07-conformance-and-quality.md"]
public_apis = ["ara2_bridge::core::AraError", "ara2_bridge::core::Diagnostic", "ara2_bridge::core::BoundedDiagnosticSink", "ara2_bridge::core::PoisonState"]
examples = ["ara2-bridge/examples/minimal-host.rs"]
conformance_commands = ["cargo test -p ara2-bridge-core --test dispatch", "cargo test -p ara2-bridge-core --test ffi_validation -- --nocapture"]
testhost_args = ["in-process: generation=V23Final; diagnostics=bounded; assertions=captured; teardown_required=true; --nocapture"]
companion_binaries = ["not-applicable: diagnostic behavior is shared by all format adapters"]
sdk_environment = ["not-applicable: Rust validation runs without an SDK checkout"]
required_capabilities = ["bounded diagnostic sink", "panic containment", "per-document poison state", "deterministic fallback return"]
expected_skips = 0
fixture_hashes = ["not-applicable: invalid-input cases construct bytes in test storage"]
platform_steps = ["capture the first bounded failure and target/toolchain metadata before teardown"]
gui_main_loop = ["report diagnostics asynchronously; never synchronously log or display UI from realtime callbacks"]
timeouts = ["30 seconds per invalid-input suite"]
troubleshooting = ["docs/troubleshooting.md#general-diagnostics", "docs/troubleshooting.md#native-conformance"]

[[chapter]]
number = 10
title = "Testing with the conformance kit"
normative_specs = ["docs/specs/ara2-bridge/07-conformance-and-quality.md"]
public_apis = ["ara2_bridge::testkit::TestHost", "ara2_bridge::testkit::TestPluginTrace", "ara2_bridge::testkit::scenarios"]
examples = ["ara2-bridge/examples/content-reader.rs"]
conformance_commands = ["cargo test -p ara2-bridge-testkit --test upstream_scenarios -- --nocapture", "ARA_SDK_DIR=$PWD/reference/ARA_SDK cargo test -p ara2-bridge-testkit --features cpp-interop --test cpp_interop -- --nocapture"]
testhost_args = ["Rust TestHost: generation=V23Final; scenario=<docs/conformance/upstream-scenarios.toml name>; expected_skips=0; --nocapture", "C++ TestHost: pairing=cpp-host-rust-plugin; generation=V23Final; scenario=<buildable direct-factory scenario>; --nocapture"]
companion_binaries = ["target/debug/examples/clap-binding", "target/debug/examples/vst3-binding", "target/debug/examples/audio-unit-v2-binding"]
sdk_environment = ["ARA_SDK_DIR=$PWD/reference/ARA_SDK", "ARA_CLAP_DIR=$PWD/.third-party/clap", "ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk", "ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK"]
required_capabilities = ["capability-rich release fixture", "ARA generation 2.3 Final", "all applicable host services", "all applicable plug-in capabilities"]
expected_skips = 0
fixture_hashes = ["ara2-bridge-testkit/fixtures/scenarios/ara1-full.archive@3071f1d6bd332cec8e56112c0c60f79ebc399030459671ab7e11a08ece132dfc", "ara2-bridge-testkit/fixtures/scenarios/ara2-full.archive@3c0cfb45fc5dab202d26a16bfb5788af87d88d5c6569621e78c4434542f50c9f", "ara2-bridge-testkit/fixtures/scenarios/chunk-wave.wav@88dbe314538135405b47393dfac1d0bee801ffb463e658031f6d17fefecdbe53"]
platform_steps = ["run native companion jobs on their matching OS/architecture runner and import only envelopes that pass source/tree/hash validation"]
gui_main_loop = ["editor-view conformance uses the integrating companion format's native main loop; direct-factory C++ scenarios exclude it explicitly"]
timeouts = ["30 seconds per scenario", "10 minutes per native C++ build and pairing"]
troubleshooting = ["docs/troubleshooting.md#native-conformance", "docs/troubleshooting.md#generation-mismatch"]

[[chapter]]
number = 11
title = "API-generation compatibility and migration from 0.1"
normative_specs = ["docs/specs/ara2-bridge/09-generation-compatibility.md", "docs/specs/ara2-bridge/08-packaging-versioning-and-manual.md"]
public_apis = ["ara2_bridge::core::ApiGeneration", "ara2_bridge::plugin::FactoryBuilder", "ara2_bridge::host::LoadedFactory"]
examples = ["ara2-bridge/examples/minimal-plugin.rs"]
conformance_commands = ["cargo test -p ara2-bridge-core --test generation", "cargo test -p xtask --test compatibility", "cargo test -p ara2-bridge --test features"]
testhost_args = ["in-process: generation=V1Final or V20Draft or V20Final or V23Draft or V23Final according to compatibility vector; --nocapture"]
companion_binaries = ["not-applicable: migration starts at the direct facade/runtime boundary"]
sdk_environment = ["not-applicable: generation compatibility is encoded in pregenerated manifests"]
required_capabilities = ["generation negotiation", "represented struct-size prefix", "safe replacement for 0.1 raw pointer ownership"]
expected_skips = 0
fixture_hashes = ["not-applicable: compatibility vectors are checked-in TOML and ABI probe envelopes"]
platform_steps = ["generation 1 is unavailable on targets where the released ARA ABI excludes it; use the reported supported range"]
gui_main_loop = ["not-applicable: migration and negotiation have no GUI requirement"]
timeouts = ["30 seconds per compatibility suite"]
troubleshooting = ["docs/troubleshooting.md#migration", "docs/troubleshooting.md#generation-mismatch"]

[[chapter]]
number = 12
title = "Complete interface and feature reference"
normative_specs = ["docs/specs/ara2-bridge/00-overview.md", "docs/specs/ara2-bridge/07-conformance-and-quality.md", "docs/specs/ara2-bridge/08-packaging-versioning-and-manual.md"]
public_apis = ["ara2_bridge::sys", "ara2_bridge::core", "ara2_bridge::plugin", "ara2_bridge::host", "ara2_bridge::companion", "ara2_bridge::testkit"]
examples = ["ara2-bridge/examples/minimal-plugin.rs", "ara2-bridge/examples/minimal-host.rs", "ara2-bridge/examples/clap-binding.rs"]
conformance_commands = ["cargo xtask ara coverage --check", "cargo xtask ci validate", "cargo xtask docs verify-manual-map", "RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps"]
testhost_args = ["in-process: generation=V23Final; scenario=all named scenarios; expected_skips=0; --nocapture"]
companion_binaries = ["target/debug/examples/clap-binding", "target/debug/examples/vst3-binding", "target/debug/examples/audio-unit-v2-binding"]
sdk_environment = ["ARA_SDK_DIR=$PWD/reference/ARA_SDK", "ARA_CLAP_DIR=$PWD/.third-party/clap", "ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk", "ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK"]
required_capabilities = ["every coverage-manifest entry implemented or explicitly ARA-unsupported", "all additive facade features", "all named conformance scenarios"]
expected_skips = 0
fixture_hashes = ["ara2-bridge-testkit/fixtures/chunks/full-2.3.xml@ab9149beb163d4a5b49b768e4e51e9c32acf378fe40eb0e25ce76ead6647ec62", "ara2-bridge-testkit/fixtures/audio/rf64-ds64.wav@96321f5b8a751443f6a21229b41128fe16c8baf7697d7ae87cd54c85170c8a24"]
platform_steps = ["consult docs/conformance/ci-matrix.md for the authoritative target/feature runner ownership"]
gui_main_loop = ["only editor/view companion paths require the integrating format's native GUI main loop"]
timeouts = ["30 seconds per scenario", "10 minutes per native build", "30 seconds per fuzz smoke target"]
troubleshooting = ["docs/troubleshooting.md#general-diagnostics", "docs/troubleshooting.md#companion-discovery", "docs/troubleshooting.md#native-conformance"]
```
