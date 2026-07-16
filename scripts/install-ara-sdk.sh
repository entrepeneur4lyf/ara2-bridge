#!/usr/bin/env bash

set -euo pipefail

readonly ARA_REPOSITORY="https://github.com/Celemony/ARA_SDK.git"
readonly ARA_COMMIT="a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c"
readonly CLAP_REPOSITORY="https://github.com/free-audio/clap.git"
readonly CLAP_COMMIT="094bb76c85366a13cc6c49292226d8608d6ae50c"
readonly VST3_REPOSITORY="https://github.com/steinbergmedia/vst3sdk.git"
readonly VST3_COMMIT="9fad9770f2ae8542ab1a548a68c1ad1ac690abe0"
readonly AUDIO_UNIT_REPOSITORY="https://github.com/apple/AudioUnitSDK.git"
readonly AUDIO_UNIT_COMMIT="53ea94e5efebf864b70afb673bdd60c977818ec7"

fail() {
    echo "install-ara-sdk: $*" >&2
    return 1
}

usage() {
    cat <<'EOF'
usage: install-ara-sdk.sh [options]

Install and build the locked Celemony ARA SDK inside the consuming project.

Options:
  --project <path>    Consuming project root (default: current Git root or PWD)
  --build-dir <path> CMake build directory (default: <project>/target/ara-sdk-build)
  --config <name>    CMake configuration (default: Release)
  --jobs <count>     Parallel build jobs (default: detected CPU count)
  -h, --help         Show this help
EOF
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "required command is not installed: $1"
}

normalize_url() {
    local value="${1%/}"
    printf '%s\n' "${value%.git}"
}

discover_project_root() {
    local start="$1" root
    root="$(git -C "$start" rev-parse --show-toplevel 2>/dev/null || true)"
    [[ -n "$root" ]] || root="$(cd "$start" && pwd)"
    printf '%s\n' "$root"
}

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

submodule_update_arguments() {
    printf '%s\n' "submodule update --init --recursive"
}

verify_checkout() {
    local name="$1" repository="$2" commit="$3" path="$4" recursive="$5"
    local actual origin dirty status

    [[ -d "$path" ]] || fail "$name checkout does not exist: $path"
    git -C "$path" rev-parse --is-inside-work-tree >/dev/null 2>&1 ||
        fail "$name path is not a Git checkout: $path"

    actual="$(git -C "$path" rev-parse HEAD)"
    [[ "$actual" == "$commit" ]] ||
        fail "$name HEAD is $actual; expected $commit"
    origin="$(git -C "$path" remote get-url origin)"
    [[ "$(normalize_url "$origin")" == "$(normalize_url "$repository")" ]] ||
        fail "$name origin is $origin; expected $repository"
    dirty="$(git -C "$path" status --porcelain --ignore-submodules=none)"
    [[ -z "$dirty" ]] || fail "$name checkout is dirty: $path"

    if [[ "$recursive" == "true" ]]; then
        status="$(git -C "$path" submodule status --recursive)"
        if grep -Eq '^[+-]|^U' <<<"$status"; then
            fail "$name has missing or mismatched recursive submodules"
        fi
    fi
}

install_checkout() {
    local name="$1" repository="$2" commit="$3" path="$4" recursive="$5"
    local temporary

    if [[ -e "$path" ]]; then
        verify_checkout "$name" "$repository" "$commit" "$path" "$recursive"
        echo "$name verified: $path"
        return
    fi

    mkdir -p "$(dirname "$path")"
    temporary="${path}.install-$$"
    [[ ! -e "$temporary" ]] || fail "temporary checkout already exists: $temporary"

    cleanup_checkout() {
        rm -rf -- "$temporary"
    }
    trap cleanup_checkout RETURN

    echo "Installing $name from $repository"
    git -c core.autocrlf=false -c core.filemode=false \
        clone --no-checkout "$repository" "$temporary"
    git -C "$temporary" config core.autocrlf false
    git -C "$temporary" config core.filemode false
    git -C "$temporary" checkout --detach "$commit"
    if [[ "$recursive" == "true" ]]; then
        git -c core.autocrlf=false -c core.filemode=false \
            -C "$temporary" submodule update --init --recursive
        git -C "$temporary" submodule foreach --recursive \
            'git config core.autocrlf false && git config core.filemode false'
    fi
    mv "$temporary" "$path"
    trap - RETURN
    verify_checkout "$name" "$repository" "$commit" "$path" "$recursive"
}

cargo_entry() {
    local key="$1" value="$2"
    printf '%s = { value = "%s", relative = true }' "$key" "$value"
}

write_cargo_config() {
    local project="$1" platform="$2"
    local cargo_dir="$project/.cargo" config="$project/.cargo/config.toml"
    local key value expected missing="" temporary
    local -a entries=(
        "ARA_SDK_DIR|../.third-party/ARA_SDK"
        "ARA_CLAP_DIR|../.third-party/clap"
        "ARA_VST3_SDK_DIR|../.third-party/vst3sdk"
    )
    if [[ "$platform" == "Darwin" ]]; then
        entries+=("ARA_AUDIO_UNIT_SDK_DIR|../.third-party/AudioUnitSDK")
    fi

    mkdir -p "$cargo_dir"
    [[ -f "$config" ]] || : > "$config"

    for entry in "${entries[@]}"; do
        key="${entry%%|*}"
        value="${entry#*|}"
        expected="$(cargo_entry "$key" "$value")"
        if grep -Eq "^[[:space:]]*${key}[[:space:]]*=" "$config"; then
            grep -Fqx "$expected" "$config" ||
                fail "$config already defines a conflicting $key"
        else
            missing+="${missing:+$'\n'}$expected"
        fi
    done

    if [[ -z "$missing" ]]; then
        return 0
    fi
    temporary="$(mktemp "$cargo_dir/config.toml.install.XXXXXX")"
    awk -v entries="$missing" '
        BEGIN { inserted = 0 }
        /^[[:space:]]*\[env\][[:space:]]*$/ && !inserted {
            print
            print entries
            inserted = 1
            next
        }
        { print }
        END {
            if (!inserted) {
                print ""
                print "[env]"
                print entries
            }
        }
    ' "$config" > "$temporary"
    mv "$temporary" "$config"
}

cmake_configure_arguments() {
    local project="$1" build_dir="$2" configuration="$3" platform="$4"
    local ara_sdk="$project/.third-party/ARA_SDK"
    local -a arguments=(
        cmake
        -S "$ara_sdk/ARA_Examples"
        -B "$build_dir"
        -DARA_SETUP_DEBUGGING=OFF
        -DARA_VST3_SDK_DIR="$project/.third-party/vst3sdk"
        -DARA_CLAP_SDK_DIR="$project/.third-party/clap"
        -DCMAKE_BUILD_TYPE="$configuration"
    )
    if [[ "$platform" == "Darwin" ]]; then
        arguments+=(
            -G Xcode
            -DARA_AUDIO_UNIT_SDK_DIR="$project/.third-party/AudioUnitSDK"
        )
    fi
    printf '%s\0' "${arguments[@]}"
}

render_cmake_command() {
    local project="$1" build_dir="$2" configuration="$3" jobs="$4" platform="$5"
    local -a arguments=()
    while IFS= read -r -d '' argument; do
        arguments+=("$argument")
    done < <(cmake_configure_arguments "$project" "$build_dir" "$configuration" "$platform")
    printf '%q ' "${arguments[@]}"
    printf '&& '
    printf '%q ' cmake --build "$build_dir" --config "$configuration" --parallel "$jobs"
    printf '\n'
}

configure_and_build() {
    local project="$1" build_dir="$2" configuration="$3" jobs="$4" platform="$5"
    local -a arguments=()
    while IFS= read -r -d '' argument; do
        arguments+=("$argument")
    done < <(cmake_configure_arguments "$project" "$build_dir" "$configuration" "$platform")
    "${arguments[@]}"
    cmake --build "$build_dir" --config "$configuration" --parallel "$jobs"
}

default_jobs() {
    local jobs
    jobs="$(getconf _NPROCESSORS_ONLN 2>/dev/null || true)"
    if [[ -z "$jobs" ]]; then
        jobs="$(sysctl -n hw.ncpu 2>/dev/null || true)"
    fi
    printf '%s\n' "${jobs:-1}"
}

main() {
    local project="" build_dir="" configuration="Release" jobs="$(default_jobs)"
    local platform

    while (($#)); do
        case "$1" in
            --project)
                (($# >= 2)) || { usage >&2; return 2; }
                project="$2"
                shift 2
                ;;
            --build-dir)
                (($# >= 2)) || { usage >&2; return 2; }
                build_dir="$2"
                shift 2
                ;;
            --config)
                (($# >= 2)) || { usage >&2; return 2; }
                configuration="$2"
                shift 2
                ;;
            --jobs)
                (($# >= 2)) || { usage >&2; return 2; }
                jobs="$2"
                shift 2
                ;;
            -h|--help)
                usage
                return 0
                ;;
            *)
                usage >&2
                fail "unknown argument: $1"
                return 2
                ;;
        esac
    done

    require_command git
    require_command cmake
    platform="$(uname -s)"
    project="$(discover_project_root "${project:-$PWD}")"
    if [[ -z "$build_dir" ]]; then
        build_dir="$project/target/ara-sdk-build"
    elif [[ "$build_dir" != /* ]]; then
        build_dir="$project/$build_dir"
    fi
    [[ "$jobs" =~ ^[1-9][0-9]*$ ]] || fail "--jobs must be a positive integer"

    case "$platform" in
        Darwin)
            require_command xcodebuild
            require_command clang
            require_command clang++
            ;;
        Linux)
            require_command cc
            require_command c++
            ;;
        MINGW*|MSYS*|CYGWIN*) ;;
        *) fail "unsupported build platform: $platform" ;;
    esac

    install_checkout ARA "$ARA_REPOSITORY" "$ARA_COMMIT" \
        "$project/.third-party/ARA_SDK" true
    install_checkout CLAP "$CLAP_REPOSITORY" "$CLAP_COMMIT" \
        "$project/.third-party/clap" false
    install_checkout VST3 "$VST3_REPOSITORY" "$VST3_COMMIT" \
        "$project/.third-party/vst3sdk" true
    if [[ "$platform" == "Darwin" ]]; then
        install_checkout AudioUnitSDK "$AUDIO_UNIT_REPOSITORY" "$AUDIO_UNIT_COMMIT" \
            "$project/.third-party/AudioUnitSDK" false
    fi

    write_cargo_config "$project" "$platform"
    echo "CMake command: $(render_cmake_command \
        "$project" "$build_dir" "$configuration" "$jobs" "$platform")"
    configure_and_build "$project" "$build_dir" "$configuration" "$jobs" "$platform"

    echo "ARA SDK installation complete"
    echo "  project: $project"
    echo "  SDK: $project/.third-party/ARA_SDK ($ARA_COMMIT)"
    echo "  build: $build_dir ($configuration)"
}

if [[ "${ARA2_INSTALLER_TESTING:-0}" != "1" ]]; then
    main "$@"
fi
