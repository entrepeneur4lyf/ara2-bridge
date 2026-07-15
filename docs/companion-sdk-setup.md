# Companion SDK Setup

Companion features never download SDKs during Cargo builds. Maintainers provision exact ignored checkouts through `ci/bootstrap-reference-sdks.sh`; downstream users may point the same environment variables at independently obtained, identity-matching checkouts.

## CLAP 1.1.9

The `clap` feature uses commit `094bb76c85366a13cc6c49292226d8608d6ae50c` under MIT. Maintainer setup:

```bash
ci/bootstrap-reference-sdks.sh fetch --component clap --accept-license MIT
cargo xtask ara provenance --check --component clap
cargo xtask ara companion-probe clap --check-all
```

## VST3 v3.7.11_build_10

The exact commit is `7d92338ae922db2d559ac458824a4df40f37e82e`. Its locked release offers GPL-3.0-only or Steinberg VST3 license-policy paths; the operator must deliberately select the policy they are entitled to use. Do not copy a policy value from an example without reviewing the linked SDK terms.

```bash
export ARA_VST3_LICENSE_POLICY='<GPL-3.0-only-or-LicenseRef-Steinberg-VST3>'
ci/bootstrap-reference-sdks.sh fetch --component vst3 \
  --accept-license "$ARA_VST3_LICENSE_POLICY"
export ARA_VST3_SDK_DIR="$PWD/.third-party/vst3sdk"
cargo xtask ara provenance --check --component vst3
```

## AudioUnitSDK 1.0.0

Audio Unit v2 is built only on Apple targets. The SDK commit is `53ea94e5efebf864b70afb673bdd60c977818ec7` under Apache-2.0; platform Core Audio headers come from Xcode.

```bash
ci/bootstrap-reference-sdks.sh fetch --component audio-unit --accept-license Apache-2.0
export ARA_AUDIO_UNIT_SDK_DIR="$PWD/.third-party/AudioUnitSDK"
cargo xtask ara provenance --check --component audio-unit
```

Canonical probe envelopes are produced on each named native runner, imported without renaming, then checked with `cargo xtask ara companion-probe <component> --check-all`. Any commit, tree, transitive input hash, target, payload, or probe hash mismatch fails closed.
