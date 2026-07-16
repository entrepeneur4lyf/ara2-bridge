# Repository Guidelines

## Project Structure & Module Organization

This Rust 2021 workspace contains eight members. `ara2-bridge-sys/` holds checked-in,
target-selected raw bindings; ordinary builds do not run bindgen. `ara2-bridge-core/` owns shared
safe types and dispatch. `ara2-bridge-plugin/` and `ara2-bridge-host/` provide the two runtimes.
`ara2-bridge-companion/` contains CLAP, VST3, and Audio Unit v2 adapters.
`ara2-bridge-testkit/` supplies fixtures and conformance peers, `ara2-bridge/` is the facade, and
`xtask/` owns generation, provenance, documentation, CI, and release audits.

Tests live beside each crate under `tests/` or `#[cfg(test)]`. Generated ABI files are under
`ara2-bridge-sys/src/generated/`; do not edit them manually. `reference/` is ignored research
material and is never a build input. See `docs/building.md` for the complete build process.

## Build, Test, and Development Commands

Rust 1.82 is the minimum. Default workspace builds are SDK- and Clang-free:

- `cargo check --workspace --all-targets` type-checks every default target.
- `cargo build --workspace` builds all workspace members.
- `cargo test --workspace` runs unit, integration, and documentation tests.
- `cargo fmt --all --check` verifies formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` enforces the lint gate.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` validates public docs.

VST3, Audio Unit, generation, and native-probe work requires the project-local SDK installation:

```bash
bash scripts/install-ara-sdk.sh
cargo xtask ara provenance --check
cargo xtask ara generate --check
cargo xtask ara probe-core --check-all
```

## Coding Style & Naming Conventions

Use standard `rustfmt` output. Follow Rust naming conventions: `snake_case` functions and modules,
`CamelCase` types and traits, and `SCREAMING_SNAKE_CASE` constants. Preserve upstream names only
in raw ABI declarations. Every unsafe block or callback must document its pointer, lifetime,
thread, and ownership invariants.

## Testing Guidelines

Use descriptive `snake_case` test names. ABI changes must cover callback dispatch, layout,
nullability, ownership cleanup, and generated drift. Companion changes need the matching native
feature tests and probe checks. CI has no numeric coverage threshold; every configured gate must
pass without skipped required capabilities.

## Commit & Pull Request Guidelines

Use scoped Conventional Commit prefixes such as `feat:`, `fix:`, `docs:`, and `chore:`.
Pull requests must describe behavior and ABI impact, identify affected SDK generations, link issues,
and list commands run. Prefer test or conformance evidence over screenshots.
