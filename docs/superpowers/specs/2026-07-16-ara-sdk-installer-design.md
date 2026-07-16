# ARA SDK Installer Design

Status: Approved design  
Date: 2026-07-16

## Purpose

Provide one repeatable command that installs the complete Celemony ARA SDK development tree into a project that consumes `ara2-bridge` and proves that its libraries and examples build. The installer must use the official GitHub repository, never `reference/`, and must not require elevated privileges or manual environment setup.

## Command and Locations

`scripts/install-ara-sdk.sh` is run from the consuming project. It resolves that project's Git root, falling back to the current directory when it is not a Git checkout. `--project <path>` overrides discovery. By default it uses:

- SDK checkout: `<project>/.third-party/ARA_SDK`
- companion SDKs: `<project>/.third-party/clap`, `<project>/.third-party/vst3sdk`, and `<project>/.third-party/AudioUnitSDK`
- CMake build tree: `<project>/target/ara-sdk-build`

Options select the project root, build directory, configuration, and parallel job count. Repeated execution verifies and reuses correct clean checkouts and build trees. The script is self-contained so consumers can invoke it from a source checkout or download it directly from the `ara2-bridge` GitHub repository.

## Acquisition and Identity

The script clones `https://github.com/Celemony/ARA_SDK.git` at commit `a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c`, then explicitly runs `git submodule update --init --recursive`. The resulting checkout must pass commit, origin, cleanliness, and recursive-submodule checks. Installer pins are tested against `ci/reference-sdks.lock.toml` to prevent drift.

CLAP 1.1.9, VST3 `v3.8.0_build_66`, and AudioUnitSDK 1.0.0 are provisioned through the same lock. The script does not call Celemony's bundled VST3 installer because that release fetches VST3 `v3.7.11_build_10`, which conflicts with this repository's approved pin. AudioUnitSDK is installed only on macOS.

## Build

The script configures `ARA_Examples` with CMake and passes the installed companion paths explicitly. `ARA_SETUP_DEBUGGING=OFF` prevents the upstream project from copying plug-ins into user or system locations. It then builds the default upstream target set, which includes the ARA host and plug-in libraries, TestHost, TestPlugIn variants available on the current platform, MiniHost, and chunk writer.

The completed checkout plus companion SDKs constitute the SDK installation; no unsupported `cmake --install` or `/usr/local` copy is invented.

## Consumer Cargo Integration

The script records project-relative paths for the components installed on the current platform in the consuming project's `.cargo/config.toml` so subsequent `cargo build` commands can discover ARA, CLAP, VST3, and, on macOS, AudioUnit SDKs without shell exports. It preserves unrelated Cargo configuration, reuses matching entries, and fails rather than overwriting conflicting SDK entries. The generated paths use Cargo's relative environment-value form, keeping the project relocatable.

## Failure Handling and Verification

The script fails before building when Git, CMake, a C/C++ compiler, or required checkout identities are unavailable. It preserves failed build output for diagnosis, never repairs dirty SDK trees, and prints the resolved checkout, build directory, commit, configuration, and successful build command.

Tests cover project-root discovery, argument parsing, missing-tool diagnostics, reuse of an existing checkout, the recursive-submodule command, locked component selection, Cargo-config merging, Linux/macOS branching, and CMake configuration arguments. A live smoke run must complete on Linux, followed by the existing provenance and native conformance checks. A fixture consumer outside the bridge repository must then build using only its generated project-local configuration.
