# ARA2 Bridge Full Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver complete public ARA SDK 2.3 host and plug-in support in Rust, including companion adapters, conformance tooling, and manual-ready documentation.

**Architecture:** Build upward from pregenerated ABI artifacts into a shared safety core, semantic content/persistence layer, independent plug-in and host runtimes, companion adapters, then cross-language conformance and packaging. Each phase has a separate plan and release-quality exit gate so implementers load only this overview, the active plan, and its linked normative specs.

**Tech Stack:** Rust 2021/MSRV 1.82, C11/C++17 ABI probes and shims, bindgen as a maintainer-only generator, cargo-nextest-compatible tests, trybuild, proptest, Miri, sanitizers, CLAP 1.1.9, VST3 v3.7.11_build_10, AudioUnitSDK 1.0.0.

**Audit status:** `CLEAR` — see [the final implementation-plan audit](../../reviews/2026-07-15-ara2-bridge-plan-audit.md).

---

## Execution order

| Phase | Plan | Produces | Gate before next phase |
|---:|---|---|---|
| 0 | [ABI and workspace](2026-07-14-ara2-bridge-abi-workspace.md) | reproducible SDK bootstrap, crate graph, pregenerated ABI, exhaustive symbol coverage/provenance | clean builds without libclang; all-symbol and C/C++ layout probes pass |
| 1 | [Core safety](2026-07-14-ara2-bridge-core-safety.md) | safe types, registries, sized-struct access, diagnostics, thread/lifecycle primitives | Miri/unit/compile-fail tests pass |
| 2 | [Content and persistence](2026-07-14-ara2-bridge-content-persistence.md) | typed content, archive/filter APIs, chunk/container support, utilities | property/golden/fuzz-smoke tests pass |
| 3 | [Plug-in runtime](2026-07-14-ara2-bridge-plugin-runtime.md) | factory, 54 callbacks, host clients, roles, dirty tracking | Rust mock host drives every callback/generation |
| 4 | [Host runtime](2026-07-14-ara2-bridge-host-runtime.md) | five host services, document graph, plug-in dispatch, role control | Rust host ↔ Rust plug-in scenarios pass |
| 5 | [Companion adapters](2026-07-14-ara2-bridge-companions.md) | CLAP, VST3, AUv2 host/plug-in binding | native discovery/binding/teardown tests pass |
| 6 | [Conformance and delivery](2026-07-14-ara2-bridge-conformance-delivery.md) | upstream parity, cross-language matrix, examples, migration/manual inputs, release gates | full CI/release checklist passes |

## Normative coverage

| Spec | Primary implementation plan | Final evidence |
|---|---|---|
| `00` system overview | all phases | phase gates and release report |
| `01` ABI/generation | ABI/workspace | native probes and binding freshness |
| `02` safety/dispatch | core, plug-in, host | Miri, malformed-peer, panic/exception tests |
| `03` plug-in runtime | plug-in | 54-slot TestPlugIn contract gate |
| `04` host runtime | host | TestHost ↔ TestPlugIn scenario gate |
| `05` content/persistence/utilities | content/persistence, both runtimes | golden archives/media and fuzz targets |
| `06` companion integrations | companions | CLAP/VST3/AUv2 native interoperability |
| `07` conformance/quality | every phase, finalized in delivery | manifest join and CI matrix |
| `08` packaging/manual | ABI/workspace, delivery | clean-room packages and manual source map |
| `09` generation compatibility | ABI/workspace, plug-in, host | generations 1–6 contract matrix |
| `api-compatibility.toml` | ABI generator, all dispatch phases | exhaustive delegate/test joins |

## Global execution rules

- [ ] Before each phase, read its plan, only the listed numbered specs, and compact prior-phase handoff manifests under `docs/superpowers/handoffs/`; do not carry prior task narratives.
- [ ] Use red-green-refactor for every behavioral task; never add a callback without its contract test in the same commit.
- [ ] Before each red command, declare and re-export every new module through the exact parent/root files listed by that task, using only the minimal compiling API needed to reach the intended failing assertion. Stage those wiring files in the same task commit; an unresolved import is not an acceptable red result unless the plan explicitly names it.
- [ ] Keep `docs/specs/ara2-bridge/api-compatibility.toml` authoritative. Generated Rust metadata and tests must derive from it.
- [ ] Route every generated Rust/C/C++/JSON/TOML/Markdown derivative through the shared provenance encoder. Each artifact carries source repository/tag/commit, generator crate/version, SPDX license, and `DO NOT EDIT` in a format-appropriate header; freshness and release tests reject every missing or mismatched field.
- [ ] Revise a normative spec in the same commit whenever implementation evidence changes behavior, safety, public API shape, or scope.
- [ ] At each phase gate run formatting, workspace tests, and the active plan's target-specific clippy/rustdoc matrix: portable features everywhere, `full-portable` only with `ARA_VST3_SDK_DIR`, `full-apple` only on macOS with both native SDK variables, plus a separate non-Apple AUv2 compile-fail check. Never use workspace `--all-features` as a portable command.
- [ ] Preserve the user's pre-existing working-tree changes; stage only paths named by the active task.

## Completion evidence

- [ ] Every manifest callback maps to a safe delegate, a compatibility policy, and at least one positive and negative contract test.
- [ ] Every upstream scenario runs without capability skips against the capability-rich Rust fixture.
- [ ] Rust/C++ interoperability succeeds in both directions on Linux, Windows, and macOS.
- [ ] Published tarballs build without `reference/`, clang, network downloads, or undeclared SDK inputs.
- [ ] Specs, rustdoc, examples, migration notes, and conformance commands provide a traceable source for every planned manual chapter.
