# Companion SDK Setup

Cargo builds never download SDKs implicitly. Run the installer once from the consuming project; it clones the official repositories, initializes the ARA SDK recursively, builds Celemony's libraries and examples, and writes relocatable SDK paths to the project's `.cargo/config.toml`:

```bash
curl -fsSLO https://raw.githubusercontent.com/entrepeneur4lyf/ara2-bridge/main/scripts/install-ara-sdk.sh
bash install-ara-sdk.sh
```

The default installation is `.third-party/` in the consuming project, with build output in `target/ara-sdk-build`. Existing SDK checkouts must be clean and match the locked origin and commit. The installer preserves unrelated Cargo configuration and rejects conflicting SDK entries.

## CLAP 1.1.9

The `clap` feature uses commit `094bb76c85366a13cc6c49292226d8608d6ae50c` under MIT. Maintainer verification:

```bash
ci/bootstrap-reference-sdks.sh fetch --component clap --accept-license MIT
cargo xtask ara provenance --check --component clap
cargo xtask ara companion-probe clap --check-all
```

## VST3 v3.8.0_build_66

The exact commit is `9fad9770f2ae8542ab1a548a68c1ad1ac690abe0`. This VST3 release is pinned under MIT. Celemony's bundled ARA 2.3 installer script fetches an older VST3 release, so `install-ara-sdk.sh` installs the approved 3.8.0 checkout directly and passes its path to the ARA example build.

```bash
cargo xtask ara provenance --check --component vst3
cargo xtask ara companion-probe vst3 --check-all
```

## AudioUnitSDK 1.0.0

Audio Unit v2 is built only on Apple targets. The SDK commit is `53ea94e5efebf864b70afb673bdd60c977818ec7` under Apache-2.0; platform Core Audio headers come from Xcode.

```bash
cargo xtask ara provenance --check --component audio-unit
```

Canonical probe envelopes are produced on each named native runner, imported without renaming, then checked with `cargo xtask ara companion-probe <component> --check-all`. Any commit, tree, transitive input hash, target, payload, or probe hash mismatch fails closed.
