# Project-Local ARA SDK Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship and verify a self-contained script that installs and builds the locked ARA SDK inside any project consuming `ara2-bridge`.

**Architecture:** A portable Bash entrypoint owns project-root discovery, immutable Git checkout management, project-local Cargo configuration, and platform-aware CMake execution. Existing Rust build scripts consume the generated SDK environment paths instead of assuming the bridge repository layout. Shell contract tests exercise deterministic functions without network access; a live Linux smoke test proves the real upstream build.

**Tech Stack:** Bash 4+, Git, CMake 3.19+, C/C++, Cargo configuration TOML, Rust `cc` build scripts

---

## File Structure

- Create `scripts/install-ara-sdk.sh`: self-contained downstream installer and build orchestrator.
- Create `ci/tests/install-ara-sdk.sh`: offline shell contract tests for discovery, pins, Cargo config, and command generation.
- Modify `ara2-bridge-companion/build.rs`: resolve ARA headers through `ARA_SDK_DIR` supplied by the consuming project.
- Modify `ara2-bridge-testkit/build.rs`: use the same external ARA path contract for optional native tests.
- Modify `ci/reference-sdks.lock.toml`: remain the repository-side source checked against installer constants.
- Modify `README.md`, `docs/companion-sdk-setup.md`, `docs/troubleshooting.md`, and specifications: document consumer invocation and generated paths.

### Task 1: Add Failing Installer Contract Tests

**Files:**
- Create: `ci/tests/install-ara-sdk.sh`

- [ ] **Step 1: Write the shell test harness**

Source the installer with `ARA2_INSTALLER_TESTING=1`, create a temporary Git consumer project, and assert:

```bash
resolved="$(discover_project_root "$consumer/subdir")"
[[ "$resolved" == "$consumer" ]]
[[ "$ARA_REPOSITORY" == "https://github.com/Celemony/ARA_SDK.git" ]]
[[ "$ARA_COMMIT" == "a2b1aac1d1d5c4eed387db85a9c0cdb7d460254c" ]]
[[ "$VST3_COMMIT" == "9fad9770f2ae8542ab1a548a68c1ad1ac690abe0" ]]
```

Generate Cargo configuration and assert it preserves an existing `[build]` table while adding relative `[env]` entries. Call the command renderer and require `git submodule update --init --recursive`, `ARA_SETUP_DEBUGGING=OFF`, and the project-local SDK paths.

- [ ] **Step 2: Run the test and verify failure**

Run: `bash ci/tests/install-ara-sdk.sh`  
Expected: FAIL because `scripts/install-ara-sdk.sh` does not exist.

- [ ] **Step 3: Commit the failing contract**

```bash
git add ci/tests/install-ara-sdk.sh
git commit -m "test: define ARA SDK installer contract"
```

### Task 2: Implement the Self-Contained Installer

**Files:**
- Create: `scripts/install-ara-sdk.sh`

- [ ] **Step 1: Add constants, argument parsing, and project discovery**

Implement `--project`, `--build-dir`, `--config`, `--jobs`, and `--help`. Defaults are the invoking Git root, `target/ara-sdk-build`, `Release`, and detected CPU count. Define exact repository and commit constants for ARA, CLAP, VST3, and AudioUnitSDK.

```bash
discover_project_root() {
    local start="$1" root
    root="$(git -C "$start" rev-parse --show-toplevel 2>/dev/null || true)"
    [[ -n "$root" ]] || root="$(cd "$start" && pwd)"
    printf '%s\n' "$root"
}
```

- [ ] **Step 2: Add immutable checkout installation**

Implement `install_checkout <name> <repository> <commit> <path> <recursive>`. New paths clone with `core.autocrlf=false`, checkout the detached commit, and, when recursive, run exactly:

```bash
git -c core.autocrlf=false -c core.filemode=false \
    -C "$path" submodule update --init --recursive
```

Existing paths must have the expected origin and `HEAD`, an empty porcelain status, and no uninitialized or mismatched recursive submodule status.

- [ ] **Step 3: Add safe Cargo configuration merging**

Create or update `<project>/.cargo/config.toml` with relative entries:

```toml
[env]
ARA_SDK_DIR = { value = "../.third-party/ARA_SDK", relative = true }
ARA_CLAP_DIR = { value = "../.third-party/clap", relative = true }
ARA_VST3_SDK_DIR = { value = "../.third-party/vst3sdk", relative = true }
```

Add `ARA_AUDIO_UNIT_SDK_DIR` only on macOS. Preserve unrelated tables and fail when an existing SDK key has a different value.

- [ ] **Step 4: Add platform-aware CMake build execution**

Configure from `<project>/.third-party/ARA_SDK/ARA_Examples`, pass every installed companion path, and force `ARA_SETUP_DEBUGGING=OFF`:

```bash
cmake -S "$ara_sdk/ARA_Examples" -B "$build_dir" \
    -DARA_SETUP_DEBUGGING=OFF \
    -DARA_VST3_SDK_DIR="$vst3_sdk" \
    -DARA_CLAP_SDK_DIR="$clap_sdk" \
    -DCMAKE_BUILD_TYPE="$configuration"
cmake --build "$build_dir" --config "$configuration" --parallel "$jobs"
```

Use `-G Xcode` on macOS as required by upstream and add AudioUnitSDK there. Preserve the build directory on failure and print all resolved paths and identities on success.

- [ ] **Step 5: Run shell tests**

Run: `bash ci/tests/install-ara-sdk.sh`  
Expected: PASS with `ARA SDK installer contract: PASS`.

- [ ] **Step 6: Commit installer implementation**

```bash
git add scripts/install-ara-sdk.sh ci/tests/install-ara-sdk.sh
git commit -m "feat: add project-local ARA SDK installer"
```

### Task 3: Connect Dependency Build Scripts to the Consumer Installation

**Files:**
- Modify: `ara2-bridge-companion/build.rs`
- Modify: `ara2-bridge-testkit/build.rs`
- Test: `ara2-bridge/tests/features.rs`

- [ ] **Step 1: Add a failing consumer-boundary assertion**

Extend the feature test to inspect both build scripts and reject `reference/ARA_SDK`, `.third-party/ARA_SDK` derived from `CARGO_MANIFEST_DIR`, or parent-workspace assumptions. Require `cargo:rerun-if-env-changed=ARA_SDK_DIR`.

- [ ] **Step 2: Run the focused test and verify failure**

Run: `cargo test -p ara2-bridge --test features`  
Expected: FAIL because both build scripts still derive ARA headers from their workspace parent.

- [ ] **Step 3: Implement common environment resolution**

In each build script, resolve and validate:

```rust
fn ara_sdk() -> PathBuf {
    println!("cargo:rerun-if-env-changed=ARA_SDK_DIR");
    let sdk = env::var_os("ARA_SDK_DIR")
        .map(PathBuf::from)
        .expect("ARA_SDK_DIR must point at the project-local SDK installed by scripts/install-ara-sdk.sh");
    assert!(sdk.join("ARA_API/ARAInterface.h").is_file(), "invalid ARA_SDK_DIR: {}", sdk.display());
    sdk
}
```

Use `ara_sdk().join("ARA_API")` for VST3, AudioUnit, CLAP probes, and C++ interop. Keep default feature-free builds independent of all SDKs.

- [ ] **Step 4: Run focused Rust and companion tests**

Run:

```bash
cargo test -p ara2-bridge --test features
ARA_SDK_DIR="$PWD/.third-party/ARA_SDK" cargo test -p ara2-bridge-testkit --features cpp-interop --test cpp_interop
```

Expected: both PASS.

- [ ] **Step 5: Commit build-boundary changes**

```bash
git add ara2-bridge-companion/build.rs ara2-bridge-testkit/build.rs ara2-bridge/tests/features.rs
git commit -m "fix: resolve ARA SDK from consuming project"
```

### Task 4: Document and Validate the Consumer Workflow

**Files:**
- Modify: `README.md`
- Modify: `docs/companion-sdk-setup.md`
- Modify: `docs/troubleshooting.md`
- Modify: `docs/specs/ara2-bridge/01-abi-and-generation.md`
- Modify: `docs/specs/ara2-bridge/06-companion-integrations.md`

- [ ] **Step 1: Document the one-command install**

Add the downstream command:

```bash
curl -fsSLO https://raw.githubusercontent.com/entrepeneur4lyf/ara2-bridge/main/scripts/install-ara-sdk.sh
bash install-ara-sdk.sh
cargo build
```

Explain project-local paths, Cargo configuration preservation, exact pins, recursive submodules, platform behavior, and `ARA_SETUP_DEBUGGING=OFF`.

- [ ] **Step 2: Remove obsolete reference-folder guidance**

Run: `rg -n 'reference/ARA_SDK' --glob '!reference/**' .`  
Expected: no matches.

- [ ] **Step 3: Run documentation and CI validators**

Run:

```bash
cargo xtask ara docs --check
cargo xtask ci validate
bash ci/tests/install-ara-sdk.sh
```

Expected: all PASS.

- [ ] **Step 4: Commit documentation**

```bash
git add README.md docs/companion-sdk-setup.md docs/troubleshooting.md docs/specs/ara2-bridge
git commit -m "docs: add downstream ARA SDK installation workflow"
```

### Task 5: Live Installation and Release-Grade Verification

**Files:**
- Modify only generated ignored paths under `.third-party/` and `target/`

- [ ] **Step 1: Run the real installer in the bridge repository**

Run: `bash scripts/install-ara-sdk.sh --project "$PWD" --jobs "$(nproc)"`  
Expected: locked repositories verify, CMake configures, and the complete available upstream target set builds.

- [ ] **Step 2: Verify provenance and workspace quality**

Run:

```bash
cargo xtask ara provenance --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Expected: all PASS.

- [ ] **Step 3: Prove an external consumer path**

Create a temporary Cargo project outside the repository with a path dependency on `ara2-bridge`, run the installer with `--project` set to that fixture, and run `cargo check`. Expected: PASS without shell exports and with all SDK paths coming from the fixture's `.cargo/config.toml`.

- [ ] **Step 4: Record completion**

Update the plan checkboxes and installer documentation with any platform-specific revisions discovered during the live build, then commit only tracked source and documentation changes.
