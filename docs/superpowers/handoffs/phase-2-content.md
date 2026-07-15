# Phase 2 Handoff: Content and Persistence

Status: complete locally; CI and target-native Windows jobs remain merge authority  
Baseline: Phase 1 core handoff and the pinned ARA API/SDK provenance manifest

## Public boundary

`ara2-bridge-core` now adds:

- sealed typed content kinds and owned tempo, bar, note, tuning, key, and chord events;
- RAII typed/dynamic content readers with exact destruction, owned iteration, and bounded lending access;
- position-based `ReadAt`/`WriteAt`, `MemoryArchive`, progress tracking, and document-session store/restore filters;
- bounded `AraChunkSet` XML parsing/emission with nested extension retention and ordered multi-source archives;
- streaming WAVE/RF64/BW64/AIFF/AIFC iXML inspection and rewrite, plus validated atomic path replacement;
- ARA sample/time, tempo, bar, pitch, chord, key, range, channel-layout, processing-algorithm, and licensing utilities.

Examples `content-reader`, `archive-roundtrip`, and `audio-file-chunk` compile without application-facing raw pointers.

## Generated and pinned inputs

Five XML and five binary audio fixtures are generated from reviewed recipes. Fixture checks reject missing, stale, empty, or extra outputs; all provenance entries are globally path-sorted so set check order is irrelevant. `sdk-provenance.toml` pins the ARA sample-position, timeline, pitch-interpretation, and channel-format sources used by the Rust ports.

## Gate evidence

The following passed on x86_64 Linux on 2026-07-15:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                                      # 93 passed; 44 suites
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo +1.82.0 check --workspace
cargo +nightly fuzz run audio_file_xml -- -runs=10000
cargo +nightly fuzz run audio_file_container -- -runs=10000
cargo xtask ara generate --check
cargo xtask ara fixtures --check --set chunk-xml
cargo xtask ara fixtures --check --set audio-containers
cargo xtask ara provenance --check
cargo xtask ara probe-core --check-all
git diff --check
```

Focused Miri runs passed 34 content-event, reader, archive, XML, in-memory container, and utility tests. The path test remains a stable native test because it exercises filesystem permissions, symlinks, fsync, and rename. The archive address-space refusal passed as an actual i686 Windows GNU binary under Wine using target-local MinGW tools. MSVC linking is unavailable on this Linux machine; target-native MSVC CI remains authoritative.

## Closed revisions and next-phase constraints

- Bindgen-omitted/widened pitch constants were normalized and all three ABI envelopes refreshed.
- XML rewrites preserve structural extension templates while canonicalizing known scalars in place.
- RF64/BW64 table-sized iXML rewrites update both `riffSize` and the matching `ds64` row; parser allocations are bounded first.
- Exact negative half-sample rounding follows upstream `floor(x + 0.5)` and maps `-0.5` to zero.
- Future channel layouts require an explicitly unsafe opaque constructor; unknown flag bits remain representable.

Focused specification and plan re-audits are `CLEAR`. No discovered Phase 2 normative revision is pending. Host and plug-in phases must reuse these types and must not duplicate XML/container parsing, retain transient event pointers, or bypass registry-session filter validation.
