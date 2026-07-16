# Companion SDK Setup

For the complete Rust toolchain, operating-system prerequisites, feature matrix, ordinary build,
and maintainer quality gate, start with [`building.md`](building.md). This page focuses on the
locked native SDK inputs and their conformance commands.

Cargo builds never download SDKs implicitly. Run the installer once from the consuming project; it clones the official repositories, initializes ARA recursively, builds Celemony's libraries and examples, and writes relocatable SDK paths to `.cargo/config.toml`:

```bash
curl -fsSLO https://raw.githubusercontent.com/entrepeneur4lyf/ara2-bridge/v0.3.0/scripts/install-ara-sdk.sh
bash install-ara-sdk.sh
```

The default installation is `.third-party/` in the consuming project, with build output in `target/ara-sdk-build`. Existing checkouts must be clean and match the locked origins and commits. The installer preserves unrelated Cargo configuration and rejects conflicting SDK entries. On Linux it supplies implicit `<limits>` and `<cstdint>` includes required by the immutable ARA 2.3 examples under GCC 15.

Maintainers may also provision individual ignored checkouts from `ci/reference-sdks.lock.toml` through `ci/bootstrap-reference-sdks.sh`. The ARA source is always `https://github.com/Celemony/ARA_SDK.git`, cached at `.third-party/ARA_SDK`; `reference/` is never a build input. When using the lower-level bootstrap directly, set `ARA_SDK_DIR` and the applicable companion variable to those project-local paths.

The bootstrap command configures every checkout and submodule with `core.autocrlf=false` and
`core.filemode=false` before materializing files. This is required on Windows: converted CRLF files
do not match the byte-level provenance recorded from the upstream repositories.

## CLAP 1.1.9

The `clap` feature uses commit `094bb76c85366a13cc6c49292226d8608d6ae50c` under MIT. Maintainer setup:

```bash
ci/bootstrap-reference-sdks.sh fetch --component clap --accept-license MIT
cargo xtask ara provenance --check --component clap
cargo xtask ara companion-probe clap --check-all
```

## VST3 v3.8.0_build_66

The exact commit is `9fad9770f2ae8542ab1a548a68c1ad1ac690abe0`. Steinberg licenses VST3 3.8 under MIT; the former GPL/proprietary policy paths do not apply to this pin.

On Windows, install LLVM's `clang-cl` driver and set `CXX=clang-cl` for provenance and native-probe commands. The crate build also accepts MSVC; both build paths enable `/EHsc` because the native shim must catch every C++ exception before returning through its `extern "C"` boundary.

```powershell
$env:CXX = "clang-cl"
$env:ARA_VST3_SDK_DIR = "$PWD\.third-party\vst3sdk"
cargo xtask ara provenance --check --component vst3
cargo xtask ara companion-probe vst3 --check-target x86_64-pc-windows-msvc
```

```bash
ci/bootstrap-reference-sdks.sh fetch --component vst3 \
  --accept-license MIT
export ARA_VST3_SDK_DIR="$PWD/.third-party/vst3sdk"
cargo xtask ara provenance --check --component vst3
cargo xtask ara companion-probe vst3 --check-all
cargo test -p ara2-bridge-testkit --features vst3 --test vst3_abi --test vst3_interop
```

## AudioUnitSDK 1.0.0

Audio Unit v2 is built only on Apple targets. The SDK commit is `53ea94e5efebf864b70afb673bdd60c977818ec7` under Apache-2.0; platform Core Audio headers come from Xcode.

```bash
ci/bootstrap-reference-sdks.sh fetch --component audio-unit --accept-license Apache-2.0
export ARA_AUDIO_UNIT_SDK_DIR="$PWD/.third-party/AudioUnitSDK"
cargo xtask ara provenance --check --component audio-unit
cargo xtask ara companion-probe audio-unit-v2 --check-all
cargo test -p ara2-bridge-testkit --features audio-unit-v2 --test audio_unit_interop
```

Canonical probe envelopes are produced on each named native runner and imported without renaming. Routine native validation runs `cargo xtask ara companion-probe <component> --check-target <runner-triple>` to re-execute and compare every runner-owned envelope field with the canonical record, then `--check-all` to validate the complete canonical set. JSON whitespace is not evidence; any commit, tree, transitive input hash, target, payload, or probe hash mismatch fails closed.

Linux AArch64 may use a system-emulated target runner. The validated setup used `cross` 0.2.5 and
the locked AArch64 target:

```bash
ARA_VST3_SDK_DIR="$PWD/.third-party/vst3sdk" CXX=aarch64-linux-gnu-g++ \
  cross run -p xtask --target aarch64-unknown-linux-gnu --locked -- \
  ara companion-probe vst3 \
  --emit ara2-bridge-companion/probes/vst3-linux-aarch64.json \
  --target aarch64-unknown-linux-gnu
```

Probe runner identity comes from the compiled target, not a host `rustc -vV` query.
