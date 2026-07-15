#!/usr/bin/env bash

set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repository_root="$(cd "$script_dir/.." && pwd)"
readonly lock_file="$script_dir/reference-sdks.lock.toml"
python_command="python3"
if ! command -v "$python_command" >/dev/null 2>&1; then
    python_command="python"
fi

usage() {
    cat >&2 <<'EOF'
usage: ci/bootstrap-reference-sdks.sh <fetch|check> --component <name> [options]

Options:
  --root <path>             Repository root (defaults to the current repository)
  --accept-license <id>     Explicit SPDX or commercial policy identifier
EOF
    exit 2
}

fail() {
    echo "bootstrap-reference-sdks: $*" >&2
    exit 1
}

normalize_url() {
    local value="${1%/}"
    printf '%s\n' "${value%.git}"
}

command="${1:-}"
[[ "$command" == "fetch" || "$command" == "check" ]] || usage
shift

component=""
root="$repository_root"
accepted_license=""
while (($#)); do
    case "$1" in
        --component)
            (($# >= 2)) || usage
            component="$2"
            shift 2
            ;;
        --root)
            (($# >= 2)) || usage
            root="$2"
            shift 2
            ;;
        --accept-license)
            (($# >= 2)) || usage
            accepted_license="$2"
            shift 2
            ;;
        *) usage ;;
    esac
done

[[ -n "$component" ]] || usage
[[ -d "$root" ]] || fail "repository root does not exist: $root"
root="$(cd "$root" && pwd)"

mapfile -t fields < <("$python_command" - "$lock_file" "$component" <<'PY'
import sys
import tomllib

with open(sys.argv[1], "rb") as stream:
    data = tomllib.load(stream)
for item in data.get("component", []):
    if item["name"] == sys.argv[2]:
        print(item["path"])
        print(item["repository"])
        print(item.get("tag", ""))
        print(item["commit"])
        print(item["tree"])
        print(item["license_url"])
        print("|".join(item["accepted_licenses"]))
        break
else:
    raise SystemExit(f"unknown component: {sys.argv[2]}")
PY
) || fail "could not read $lock_file"

((${#fields[@]} == 7)) || fail "unknown component '$component' in $lock_file"
relative_path="${fields[0]}"
repository="${fields[1]}"
tag="${fields[2]}"
commit="${fields[3]}"
tree="${fields[4]}"
license_url="${fields[5]}"
license_choices="${fields[6]}"
checkout="$root/$relative_path"

license_diagnostic() {
    local rendered="${license_choices//|/, }"
    echo "locked license: $license_url" >&2
    echo "accepted policy identifiers: $rendered" >&2
}

verify_checkout() {
    [[ -d "$checkout" ]] || {
        license_diagnostic
        fail "missing $relative_path; run fetch --component $component --accept-license ${license_choices%%|*} (repository $repository, commit $commit)"
    }
    git -C "$checkout" rev-parse --is-inside-work-tree >/dev/null 2>&1 ||
        fail "$relative_path is not a Git checkout"

    local dirty actual
    dirty="$(git -C "$checkout" status --porcelain --ignore-submodules=none)"
    [[ -z "$dirty" ]] || fail "$relative_path is dirty; refusing to update or accept it"

    actual="$(git -C "$checkout" rev-parse HEAD)"
    [[ "$actual" == "$commit" ]] || fail "$relative_path HEAD is $actual; expected $commit"
    actual="$(git -C "$checkout" rev-parse 'HEAD^{tree}')"
    [[ "$actual" == "$tree" ]] || fail "$relative_path tree is $actual; expected $tree"
    actual="$(git -C "$checkout" remote get-url origin)"
    [[ "$(normalize_url "$actual")" == "$(normalize_url "$repository")" ]] ||
        fail "$relative_path origin is $actual; expected $repository"

    while IFS=$'\t' read -r path sub_repository sub_commit sub_tree; do
        [[ -n "$path" ]] || continue
        local sub_checkout="$checkout/$path"
        [[ -e "$sub_checkout/.git" ]] || fail "$relative_path submodule is not initialized: $path"
        actual="$(git -C "$sub_checkout" rev-parse HEAD)"
        [[ "$actual" == "$sub_commit" ]] ||
            fail "$relative_path/$path HEAD is $actual; expected $sub_commit"
        actual="$(git -C "$sub_checkout" rev-parse 'HEAD^{tree}')"
        [[ "$actual" == "$sub_tree" ]] ||
            fail "$relative_path/$path tree is $actual; expected $sub_tree"
        actual="$(git -C "$sub_checkout" remote get-url origin)"
        [[ "$(normalize_url "$actual")" == "$(normalize_url "$sub_repository")" ]] ||
            fail "$relative_path/$path origin is $actual; expected $sub_repository"
    done < <("$python_command" - "$lock_file" "$component" <<'PY'
import sys
import tomllib

with open(sys.argv[1], "rb") as stream:
    data = tomllib.load(stream)
item = next(entry for entry in data["component"] if entry["name"] == sys.argv[2])
for submodule in item.get("submodule", []):
    print("\t".join((submodule["path"], submodule["repository"], submodule["commit"], submodule["tree"])))
PY
)

    printf '%s\n' "$component SDK verified at $relative_path ($commit)"
}

if [[ "$command" == "check" ]]; then
    verify_checkout
    exit 0
fi

[[ -n "$accepted_license" ]] || {
    license_diagnostic
    fail "fetch requires --accept-license <policy-id>"
}
case "|$license_choices|" in
    *"|$accepted_license|"*) ;;
    *)
        license_diagnostic
        fail "license policy '$accepted_license' does not match the lock"
        ;;
esac
echo "accepted $accepted_license for $component; terms: $license_url" >&2

if [[ -e "$checkout" ]]; then
    verify_checkout
    exit 0
fi

mkdir -p "$(dirname "$checkout")"
temporary="$checkout.bootstrap-$$"
trap 'rm -rf "$temporary"' EXIT
[[ ! -e "$temporary" ]] || fail "temporary checkout already exists: $temporary"

git clone --no-checkout "$repository" "$temporary"
git -C "$temporary" checkout --detach "$commit"
git -C "$temporary" submodule update --init --recursive
mv "$temporary" "$checkout"
trap - EXIT
verify_checkout
