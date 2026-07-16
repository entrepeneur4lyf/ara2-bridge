# ARA2 Bridge Companion Adapters

`ara2-bridge-companion` connects the shared ARA 2.3 runtime to an externally owned CLAP, VST3, or Audio Unit v2 processor. It exposes and discovers ARA factories, performs one-shot controller binding, validates role negotiation, and tracks ordering boundaries. It does not implement DSP, companion state, processor creation, or GUI behavior.

## Features

- `clap` uses checked-in direct declarations from CLAP 1.1.9. No SDK path is needed for ordinary builds.
- `vst3` builds an opaque C++17 shim against MIT-licensed VST3 SDK `v3.8.0_build_66`. Set `ARA_VST3_SDK_DIR` to the exact locked checkout.
- `audio-unit-v2` is Apple-only and builds an Objective-C++ property shim against AudioUnitSDK `AudioUnitSDK-1.0.0`. Set `ARA_AUDIO_UNIT_SDK_DIR`.

All features are independent and disabled by default. Build scripts validate configured SDK identity but never download code or accept licenses.

```bash
cargo check -p ara2-bridge-companion --features clap
ARA_VST3_SDK_DIR=$PWD/.third-party/vst3sdk \
  cargo check -p ara2-bridge-companion --features vst3
ARA_AUDIO_UNIT_SDK_DIR=$PWD/.third-party/AudioUnitSDK \
  cargo check -p ara2-bridge-companion --features audio-unit-v2 # macOS only
```

## Processor integration

Create a `CompanionProcessorBinding` from process-lifetime factory associations and the roles your processor implements. Bind before state loading, activation, process-context queries, processing, or view creation. When the host destroys the ARA controller first, call the adapter’s `observe_controller_destruction` before releasing the controller. The adapter retains tombstoned shared state when the companion object dies first.

CLAP processors publish `clap_ara_get_extension`. VST3 processors delegate the three ARA IID queries to `Vst3PluginEntryAdapter::query_interface`; register `Vst3MainFactoryAdapter` under the exact processor class name. Audio Unit subclasses delegate the ARA property IDs from `GetPropertyInfo` and `GetProperty` to `AudioUnitPluginAdapter`.

See [`docs/companion-sdk-setup.md`](../docs/companion-sdk-setup.md) for provisioning and audit commands.
