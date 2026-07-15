#!/usr/bin/env bash
set -euo pipefail

mode=${1:-}
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
target=${TARGET:-x86_64-unknown-linux-gnu}

build_invalid_pointer_case() {
    local sanitizer=$1
    local output_dir="$root/target/sanitizers/$sanitizer"
    CARGO_TARGET_DIR=$output_dir \
        RUSTFLAGS="-Zsanitizer=$sanitizer" \
        cargo +nightly build \
        --target "$target" \
        -p ara2-bridge-testkit \
        --bin invalid_pointer_case
    printf '%s/%s/debug/invalid_pointer_case' "$output_dir" "$target"
}

expect_invalid_pointer_failure() {
    local binary=$1
    local case_name=$2
    local sanitizer=$3
    local log="$root/target/sanitizers/$sanitizer/$case_name.log"
    set +e
    ARA2_BRIDGE_ALLOW_INVALID_POINTER_CASE=1 \
        ASAN_OPTIONS=detect_leaks=0:halt_on_error=1 \
        UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1 \
        "$binary" "$case_name" >"$log" 2>&1
    local status=$?
    set -e
    if [[ $status -eq 0 ]]; then
        echo "$sanitizer $case_name unexpectedly succeeded" >&2
        return 1
    fi
    if ! grep -Eiq \
        'AddressSanitizer|UndefinedBehaviorSanitizer|runtime error|SEGV|segmentation fault|signal' \
        "$log"; then
        echo "$sanitizer $case_name lacked a sanitizer/signal classification" >&2
        cat "$log" >&2
        return 1
    fi
}

case "$mode" in
    asan-invalid-pointer)
        binary=$(build_invalid_pointer_case address)
        expect_invalid_pointer_failure "$binary" null-adjacent address
        expect_invalid_pointer_failure "$binary" unreadable address
        expect_invalid_pointer_failure "$binary" guard-page address
        ;;
    ubsan-invalid-pointer)
        cargo +nightly run -p ara2-bridge-testkit --bin invalid_pointer_case -- malformed
        mkdir -p "$root/target/sanitizers/undefined"
        "${CC:-clang}" \
            -std=c11 \
            -D_GNU_SOURCE \
            -fsanitize=undefined \
            -fno-sanitize-recover=all \
            "$root/ci/invalid-pointer-ubsan.c" \
            -o "$root/target/sanitizers/undefined/invalid-pointer-ubsan"
        binary="$root/target/sanitizers/undefined/invalid-pointer-ubsan"
        expect_invalid_pointer_failure "$binary" null-adjacent undefined
        expect_invalid_pointer_failure "$binary" unreadable undefined
        expect_invalid_pointer_failure "$binary" guard-page undefined
        ;;
    tsan-state-models)
        CARGO_TARGET_DIR="$root/target/sanitizers/thread-state-models" \
            RUSTFLAGS='-Zsanitizer=thread' \
            cargo +nightly -Zbuild-std test \
            --target "$target" \
            -p ara2-bridge-core \
            --test state_models
        ;;
    tsan-production)
        CARGO_TARGET_DIR="$root/target/sanitizers/thread-production" \
            RUSTFLAGS='-Zsanitizer=thread' \
            cargo +nightly -Zbuild-std test \
            --target "$target" \
            -p ara2-bridge-testkit \
            --test analysis_concurrency \
            --test sample_access_concurrency \
            --test editor_renderer_concurrency
        ;;
    *)
        echo "usage: $0 {asan-invalid-pointer|ubsan-invalid-pointer|tsan-state-models|tsan-production}" >&2
        exit 64
        ;;
esac
