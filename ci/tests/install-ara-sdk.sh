#!/usr/bin/env bash

set -euo pipefail

readonly repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly installer="$repo_root/scripts/install-ara-sdk.sh"

export ARA2_INSTALLER_TESTING=1
# shellcheck source=../../scripts/install-ara-sdk.sh
source "$installer"

fail() {
    echo "install-ara-sdk test: $*" >&2
    exit 1
}

temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT

consumer="$temporary/consumer"
mkdir -p "$consumer/subdir" "$consumer/.cargo"
git -C "$consumer" init --quiet

resolved="$(discover_project_root "$consumer/subdir")"
[[ "$resolved" == "$consumer" ]] || fail "project discovery returned $resolved"

[[ "$ARA_REPOSITORY" == "https://github.com/Celemony/ARA_SDK.git" ]] ||
    fail "unexpected ARA repository"
[[ "$ARA_COMMIT" == "a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c" ]] ||
    fail "unexpected ARA commit"
[[ "$VST3_COMMIT" == "9fad9770f2ae8542ab1a548a68c1ad1ac690abe0" ]] ||
    fail "unexpected VST3 commit"
grep -Fq "commit = \"$ARA_COMMIT\"" "$repo_root/ci/reference-sdks.lock.toml" ||
    fail "ARA installer commit drifted from the repository lock"
grep -Fq "commit = \"$VST3_COMMIT\"" "$repo_root/ci/reference-sdks.lock.toml" ||
    fail "VST3 installer commit drifted from the repository lock"
[[ "$(submodule_update_arguments)" == "submodule update --init --recursive" ]] ||
    fail "recursive submodule arguments changed"

cat > "$consumer/.cargo/config.toml" <<'EOF'
[build]
target-dir = "target-custom"
EOF

write_cargo_config "$consumer" Linux
config="$consumer/.cargo/config.toml"
grep -Fq 'target-dir = "target-custom"' "$config" || fail "existing Cargo config was lost"
grep -Fq 'ARA_SDK_DIR = { value = ".third-party/ARA_SDK", relative = true }' "$config" ||
    fail "ARA SDK entry is missing"
grep -Fq 'ARA_CLAP_DIR = { value = ".third-party/clap", relative = true }' "$config" ||
    fail "CLAP entry is missing"
grep -Fq 'ARA_VST3_SDK_DIR = { value = ".third-party/vst3sdk", relative = true }' "$config" ||
    fail "VST3 entry is missing"
if grep -Fq 'ARA_AUDIO_UNIT_SDK_DIR' "$config"; then
    fail "AudioUnit entry must not be written on Linux"
fi

before="$(sha256_file "$config")"
write_cargo_config "$consumer" Linux
after="$(sha256_file "$config")"
[[ "$before" == "$after" ]] || fail "Cargo config update is not idempotent"

mkdir -p "$consumer/src"
cat > "$consumer/Cargo.toml" <<'EOF'
[package]
name = "installer-path-fixture"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]
EOF
cat > "$consumer/build.rs" <<'EOF'
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    for (key, suffix) in [
        ("ARA_SDK_DIR", ".third-party/ARA_SDK"),
        ("ARA_CLAP_DIR", ".third-party/clap"),
        ("ARA_VST3_SDK_DIR", ".third-party/vst3sdk"),
    ] {
        assert_eq!(PathBuf::from(std::env::var(key).unwrap()), root.join(suffix));
    }
}
EOF
: > "$consumer/src/lib.rs"
(cd "$consumer" && cargo check --quiet) ||
    fail "Cargo did not resolve generated SDK paths from the consuming project"

mac_consumer="$temporary/mac-consumer"
mkdir -p "$mac_consumer"
write_cargo_config "$mac_consumer" Darwin
grep -Fq 'ARA_AUDIO_UNIT_SDK_DIR = { value = ".third-party/AudioUnitSDK", relative = true }' \
    "$mac_consumer/.cargo/config.toml" || fail "AudioUnit entry is missing on macOS"

conflict="$temporary/conflict"
mkdir -p "$conflict/.cargo"
cat > "$conflict/.cargo/config.toml" <<'EOF'
[env]
ARA_SDK_DIR = "/opt/other-sdk"
EOF
if (write_cargo_config "$conflict" Linux >/dev/null 2>&1); then
    fail "conflicting Cargo SDK entry was overwritten"
fi

rendered="$(render_cmake_command \
    "$consumer" "$consumer/target/ara-sdk-build" Release 8 Linux)"
[[ "$rendered" == *'-DARA_SETUP_DEBUGGING=OFF'* ]] || fail "debug installation was not disabled"
[[ "$rendered" == *'.third-party/ARA_SDK/ARA_Examples'* ]] || fail "ARA source path is missing"
[[ "$rendered" == *'.third-party/vst3sdk'* ]] || fail "VST3 path is missing"
[[ "$rendered" == *'.third-party/clap'* ]] || fail "CLAP path is missing"
[[ "$rendered" == *'CMAKE_CXX_FLAGS=-include'* ]] ||
    fail "Linux command is missing the GCC 15 ARA header compatibility include"
if [[ "$rendered" == *'AudioUnitSDK'* ]]; then
    fail "Linux CMake command contains AudioUnitSDK"
fi

mac_rendered="$(render_cmake_command \
    "$consumer" "$consumer/target/ara-sdk-build" Release 8 Darwin)"
[[ "$mac_rendered" == *'-G Xcode'* ]] || fail "macOS command does not select Xcode"
[[ "$mac_rendered" == *'AudioUnitSDK'* ]] || fail "macOS command is missing AudioUnitSDK"

echo "ARA SDK installer contract: PASS"
