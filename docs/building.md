# Building ara2-bridge

This guide separates ordinary Rust builds from SDK-backed companion builds and maintainer-only
generation. Choose the smallest tier that matches the features you use.

## Build Tiers

| Tier | Features | Required tools and inputs |
| --- | --- | --- |
| Rust-only | default `plugin`, `host`, `testkit`, `clap` | Rust 1.82 or newer |
| Native companion | `vst3` | Rust, Git, CMake, a C++17 compiler, locked ARA and VST3 checkouts |
| Apple companion | `audio-unit-v2` | macOS, Xcode, CMake, locked ARA and AudioUnitSDK checkouts |
| Maintainer generation | bindings, provenance, native probes | Complete locked SDK set, Clang and libclang, platform-native runners |

`ara2-bridge-sys` selects checked-in bindings for x86_64, AArch64, or i686. It does not run
bindgen or require SDK headers during an ordinary Cargo build. VST3 and Audio Unit features compile
small native shims and therefore validate their SDK inputs before compilation.

## Platform Prerequisites

Install Rust with rustup and keep the repository's minimum supported version available:

```bash
rustup toolchain install 1.82.0
rustup component add rustfmt clippy
```

For the project-local SDK installer, also provide Bash 4+, Git, CMake 3.19 or newer, and a native
C/C++ toolchain.

- Linux: GCC or Clang works for the SDK build. On Debian or Ubuntu, install `build-essential
  cmake git curl`. Maintainer binding generation also needs `clang libclang-dev`.
- macOS: install the Xcode command-line tools with `xcode-select --install`, then make `cmake`
  available, for example with Homebrew. The installer requires `xcodebuild`, `clang`, and
  `clang++`.
- Windows: use the Rust MSVC toolchain, Visual Studio 2022 Build Tools with “Desktop development
  with C++”, CMake, and Git for Windows. Run the installer from Git Bash so `uname`, Bash, Git,
  CMake, and the Visual Studio generator share the Windows environment. LLVM's `clang-cl` is
  required for maintainer provenance and native-probe commands, but MSVC is supported for normal
  VST3 compilation.

## Build the Repository Without SDKs

A clean checkout builds and tests all default workspace members without downloading native SDKs:

```bash
git clone https://github.com/entrepeneur4lyf/ara2-bridge.git
cd ara2-bridge
cargo check --workspace --all-targets
cargo build --workspace
cargo test --workspace
```

Useful facade configurations are:

```bash
cargo build -p ara2-bridge
cargo build -p ara2-bridge --no-default-features --features host
cargo build -p ara2-bridge --no-default-features --features clap
```

The default feature is `plugin`. `full-portable` includes `plugin`, `host`, `clap`, and
`vst3`, so it is not SDK-free. `full-apple` adds `audio-unit-v2` and only builds for Apple
targets.

## Install the Locked SDKs

Cargo never downloads or accepts SDK licenses. Run the installer once from the root of the project
that consumes `ara2-bridge`:

```bash
curl -fsSLO https://raw.githubusercontent.com/entrepeneur4lyf/ara2-bridge/v0.3.0/scripts/install-ara-sdk.sh
bash install-ara-sdk.sh
```

From this repository, use `bash scripts/install-ara-sdk.sh`. The script:

1. Resolves the consuming Git root, or uses the current directory outside Git.
2. Clones the locked ARA SDK recursively plus CLAP and VST3. On macOS it also clones
   AudioUnitSDK.
3. Verifies every origin, commit, submodule, and clean worktree before reuse.
4. Writes relocatable SDK variables under the consuming project's `.cargo/config.toml`.
5. Configures `ARA_Examples` with `ARA_SETUP_DEBUGGING=OFF` and builds the upstream examples
   under `target/ara-sdk-build`.

The installation is project-local. It never reads `reference/`, invokes `sudo`, copies SDKs to
`/usr/local`, or installs plug-ins into user directories. The upstream CMake build proves the
pinned SDK and examples compile; Cargo consumes the source checkouts, not libraries from that build
tree.

Default locations are:

```text
.third-party/ARA_SDK
.third-party/clap
.third-party/vst3sdk
.third-party/AudioUnitSDK       # macOS only
target/ara-sdk-build
```

Inspect or override installer settings with:

```bash
bash scripts/install-ara-sdk.sh --help
bash scripts/install-ara-sdk.sh --project /path/to/consumer --build-dir target/ara-sdk-build \
  --config Release --jobs 8
```

Repeated runs reuse only clean checkouts at the exact locked identities. A dirty checkout, wrong
remote, wrong commit, or conflicting Cargo environment entry fails closed instead of being repaired
or overwritten.

## Enable Native Companion Features

The installer writes these relative Cargo environment values:

```toml
[env]
ARA_SDK_DIR = { value = ".third-party/ARA_SDK", relative = true }
ARA_CLAP_DIR = { value = ".third-party/clap", relative = true }
ARA_VST3_SDK_DIR = { value = ".third-party/vst3sdk", relative = true }
ARA_AUDIO_UNIT_SDK_DIR = { value = ".third-party/AudioUnitSDK", relative = true } # macOS
```

Select dependency features in the consuming project's manifest:

```toml
[dependencies]
ara2-bridge = { version = "0.3.0", default-features = false, features = ["host", "vst3"] }
```

Then build normally:

```bash
cargo build
```

For repository examples:

```bash
cargo run -p ara2-bridge --example clap-binding --features clap
cargo run -p ara2-bridge --example vst3-binding --features vst3
cargo run -p ara2-bridge --example audio-unit-v2-binding --features audio-unit-v2 # macOS
```

Build scripts verify the locked Git repository, commit, tree, required headers, and clean status.
This means pointing an SDK variable at a compatible-looking but different checkout is intentionally
rejected.

## Maintainer Generation and Quality Gate

Generation is separate from compilation. Never edit `ara2-bridge-sys/src/generated/`, native
probe JSON, or provenance manifests by hand. After installing the SDKs, verify the checked-in
derivatives:

```bash
cargo xtask ara provenance --check
cargo xtask ara generate --check
cargo xtask ara probe-core --check-all
cargo xtask ara provenance --check --component clap
cargo xtask ara companion-probe clap --check-all
```

VST3 and Audio Unit checks require their matching SDKs and native platforms; see
[`companion-sdk-setup.md`](companion-sdk-setup.md) for component-specific commands. Native probe
results must be produced on the named target runner. Cross-compilation checks compilation, but does
not replace runtime conformance.

Run the repository quality gate before submitting changes:

```bash
bash ci/tests/install-ara-sdk.sh
cargo fmt --all --check
cargo check --workspace --all-targets
cargo +1.82.0 check --workspace --all-targets --locked
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo xtask docs verify-manual-map
cargo xtask docs verify-public-docs
cargo xtask ci validate
```

CI repeats runtime checks on x86_64 and AArch64 Linux, x86_64 Windows MSVC, and Intel and Apple
Silicon macOS. Native companion conformance runs separately for every supported platform.

## Common Failures

- `ARA_SDK_DIR must point...`: install the locked SDKs in the consuming project; do not point at
  this repository's `reference/` directory.
- `SDK ... is dirty` or identity mismatch: restore the checkout to its locked, clean state or
  replace it by rerunning the installer after moving the invalid directory aside.
- Missing `libclang`: ordinary builds do not need it. Install it only for maintainer generation.
- `audio-unit-v2 is supported only on Apple targets`: remove that feature on Linux or Windows.
- Cargo configuration conflict: preserve the existing value intentionally or reconcile it with the
  exact relative entry above; the installer will not overwrite it.

See [`troubleshooting.md`](troubleshooting.md) for runtime, lifecycle, persistence, and native
conformance failures.
