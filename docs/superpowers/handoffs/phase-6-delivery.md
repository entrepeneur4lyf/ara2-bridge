# Phase 6 Delivery Handoff

Candidate `0.2.0-alpha.1` publishes seven crates with default plug-in authoring
and additive host, testkit, CLAP, VST3, and Apple-only Audio Unit v2 features.
Manual authors start from `docs/manual-source-map.md`, executable facade examples,
crate rustdoc, migration guidance, and `docs/troubleshooting.md`.

The local release surface is executable through `cargo xtask release`: API,
unsafe, and license audits; precommit source-input verification; deterministic
source-bundle creation; and isolated offline bundle verification. The source bundle
contains seven `.crate` archives, their unpacked clean-room workspace, the locked
directory source, complete licenses and provenance, and a sorted SHA-256 inventory.
Only the operator-controlled local procedure creates, signs, or publishes release
artifacts; CI is validation-only and has no release workflow.

No tracked document claims run-specific success. Exact package hashes, native
platform results, sanitizer/fuzz durations, workflow IDs, and conclusions remain
operator-reviewed candidate evidence. AAX and Audio Unit v3 remain explicit
boundaries. Any tracked change creates a new candidate and requires the complete
matrix plus local source-bundle verification again.
