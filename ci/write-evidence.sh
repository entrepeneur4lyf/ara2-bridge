#!/usr/bin/env bash

set -euo pipefail

output="${1:?usage: ci/write-evidence.sh <output.json>}"
: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"
: "${GITHUB_SHA:?GITHUB_SHA is required}"
: "${GITHUB_WORKFLOW:?GITHUB_WORKFLOW is required}"
: "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"
: "${GITHUB_JOB:?GITHUB_JOB is required}"
: "${EVIDENCE_TARGET:?EVIDENCE_TARGET is required}"
: "${EVIDENCE_TOOLCHAIN:?EVIDENCE_TOOLCHAIN is required}"
: "${EVIDENCE_COMMAND:?EVIDENCE_COMMAND is required}"
: "${EVIDENCE_CONCLUSION:?EVIDENCE_CONCLUSION is required}"

mkdir -p "$(dirname "$output")"

python_command="python3"
if ! command -v "$python_command" >/dev/null 2>&1; then
    python_command="python"
fi

"$python_command" - "$output" <<'PY'
import hashlib
import json
import os
import pathlib
import sys

def hashes(variable):
    result = {}
    for item in os.environ.get(variable, "").splitlines():
        if not item:
            continue
        path = pathlib.Path(item)
        if path.is_file():
            result[item] = hashlib.sha256(path.read_bytes()).hexdigest()
    return dict(sorted(result.items()))

fragment = {
    "schema": 1,
    "repository": os.environ["GITHUB_REPOSITORY"],
    "head_sha": os.environ["GITHUB_SHA"],
    "workflow": os.environ["GITHUB_WORKFLOW"],
    "workflow_run_id": os.environ["GITHUB_RUN_ID"],
    "job_id": os.environ["GITHUB_JOB"],
    "target": os.environ["EVIDENCE_TARGET"],
    "toolchain": os.environ["EVIDENCE_TOOLCHAIN"],
    "command": os.environ["EVIDENCE_COMMAND"],
    "conclusion": os.environ["EVIDENCE_CONCLUSION"],
    "input_hashes": hashes("EVIDENCE_INPUTS"),
    "output_hashes": hashes("EVIDENCE_OUTPUTS"),
}
pathlib.Path(sys.argv[1]).write_text(
    json.dumps(fragment, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
PY

cargo xtask ci validate-evidence "$output"
