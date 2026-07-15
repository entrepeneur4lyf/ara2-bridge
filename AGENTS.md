# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust 2021 workspace with two library crates:

- `ara2-bridge-sys/` contains the ARA2 C headers, the `bindgen` build script, and raw FFI exports. Generated bindings are written to Cargo's `OUT_DIR`; do not hand-edit generated code.
- `ara2-bridge/` provides the safe-facing traits, callback shims, host vtable constructors, and controller state management.
- `ara2-bridge/src/lib.rs` also contains the current unit tests under `#[cfg(test)]`.
- `.github/workflows/ci.yml` is the authoritative CI checklist. `reference/ARA_API/` is an ignored local copy of the upstream SDK, not workspace source.

## Build, Test, and Development Commands

Install `clang`/`libclang` before building because `ara2-bridge-sys` runs bindgen.

- `cargo check --workspace` — type-check both crates quickly.
- `cargo build --workspace` — compile the complete workspace.
- `cargo test --workspace` — run all unit and documentation tests.
- `cargo fmt --all --check` — verify formatting; use `cargo fmt --all` to fix it.
- `cargo clippy --workspace -- -D warnings` — run the CI lint policy.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — validate public documentation.

There is no local application command; these crates are consumed by an ARA-capable audio plugin.

## Coding Style & Naming Conventions

Use standard `rustfmt` output (four-space indentation). Follow Rust naming conventions: `snake_case` functions and modules, `CamelCase` types and traits, and `SCREAMING_SNAKE_CASE` constants. Preserve upstream C names only in raw FFI bindings. Keep safe abstractions in `ara2-bridge` and raw ABI details in `ara2-bridge-sys`. Document every `unsafe` block or callback with the pointer, lifetime, thread, and ownership invariant it relies on.

## Testing Guidelines

Add focused `#[test]` functions beside the code they exercise; use descriptive `snake_case` names such as `test_plugin_lifecycle`. Cover callback-to-trait dispatch, vtable construction, null handling, and ownership cleanup when changing ABI-facing code. No numeric coverage threshold is configured; CI requires the full workspace test suite to pass.

## Commit & Pull Request Guidelines

Recent history uses Conventional Commit prefixes such as `feat:`, `fix:`, `docs:`, and `chore:`. Keep commits scoped and imperative. Pull requests should explain the behavior and ABI impact, identify the ARA SDK/API generation involved, link relevant issues, and include the commands run. Screenshots are only useful for downstream plugin UI changes; for this library, prefer test output or integration notes.
