#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bootstrap="$repo_root/ci/bootstrap-reference-sdks.sh"
empty_root="$(mktemp -d)"
trap 'rm -rf "$empty_root"' EXIT

if output="$($bootstrap check --root "$empty_root" --component ara 2>&1)"; then
    echo "expected the ARA preflight to reject an absent SDK checkout" >&2
    exit 1
fi

for expected in \
    ".third-party/ARA_SDK" \
    "https://github.com/Celemony/ARA_SDK.git" \
    "a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c" \
    "--accept-license Apache-2.0"
do
    if [[ "$output" != *"$expected"* ]]; then
        echo "absent-input diagnostic did not name: $expected" >&2
        echo "$output" >&2
        exit 1
    fi
done

if output="$($bootstrap fetch --root "$empty_root" --component ara --accept-license MIT 2>&1)"; then
    echo "expected a mismatched license policy to be rejected" >&2
    exit 1
fi
[[ "$output" == *"does not match the lock"* ]]
[[ "$output" == *"LICENSE.txt"* ]]

git -C "$repo_root" check-ignore -q .third-party/ARA_SDK
git -C "$repo_root" check-ignore -q .third-party/clap

$bootstrap check --component ara
$bootstrap fetch --component ara --accept-license Apache-2.0

mkdir -p "$empty_root/.third-party"
git clone --quiet --shared --no-checkout "$repo_root/.third-party/ARA_SDK" \
    "$empty_root/.third-party/ARA_SDK"
git -C "$empty_root/.third-party/ARA_SDK" remote set-url origin \
    https://github.com/Celemony/ARA_SDK.git
git -C "$empty_root/.third-party/ARA_SDK" checkout --quiet --detach \
    a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c
printf '\ndirty test\n' >> "$empty_root/.third-party/ARA_SDK/README.md"
if output="$($bootstrap fetch --root "$empty_root" --component ara --accept-license Apache-2.0 2>&1)"; then
    echo "expected an existing dirty checkout to be rejected" >&2
    exit 1
fi
[[ "$output" == *"is dirty"* ]]

echo "absent-input preflight diagnostic: PASS"
