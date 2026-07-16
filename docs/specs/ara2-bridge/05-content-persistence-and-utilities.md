# Content, Persistence, and Utilities

Status: Normative component specification  
Depends on: [Core Safety and Dispatch](02-core-safety-and-dispatch.md), [Plug-in Runtime](03-plugin-runtime.md), [Host Runtime](04-host-runtime.md)  
Last revised: 2026-07-15

## Scope

This spec defines shared semantic data and algorithms that cross host/plug-in boundaries: content events and readers, analysis state, archives and filters, ARA audio-file chunks, processing algorithms, licensing, and the generally useful conversions supplied by `ARA_Library`.

## Typed content

The safe API represents the six released content kinds with sealed marker types and their exact events:

- tempo entries → `ARAContentTempoEntry`;
- bar signatures → `ARAContentBarSignature`;
- notes → `ARAContentNote`;
- static tuning → `ARAContentTuning`;
- key signatures → `ARAContentKeySignature`;
- sheet chords → `ARAContentChord`.

`ContentKind` binds the ARA content-type constant, aligned owned event type, validation rules, and documentation. `ContentReader<K>` is RAII, immutable, bounds-checked, and holds exclusive reader access to its controller/session. Untyped dynamic readers are available for host discovery and downcast only after validating the content type.

Event validators port the relevant upstream content-validator rules: finite and ordered time values, valid duration/pitch/range fields, legal enum/flag values, monotonic sequences where required, and valid UTF-8/C strings. Content grade and update-scope flags use non-exhaustive Rust representations that preserve unknown future bits.

## Content readers and updates

Reader creation accepts an optional time range and returns absence distinctly from failure. Count conversion is checked. Because an event pointer expires on the next data call, the primary iterator yields aligned owned events. A zero-copy alternative is a lending `with_event(index, |event| ...)` closure that cannot retain the borrow or make another controller call. Reader destruction occurs exactly once, including early-return and panic paths.

Update ranges use `Option<ContentTimeRange>`. Scope flags preserve signal, note, timing, and other SDK semantics without collapsing them into a boolean. Host-triggered updates and successful restoration suppress echo notifications unless recovery/conversion creates a real additional plug-in change. Model notifications are queued during editing/analysis and flushed only at valid points. The testkit supplies content loggers and validators equivalent to the SDK debug helpers.

## Analysis and processing algorithms

Analysis state is tracked per audio source and content type. When progress notifications are emitted, they follow the ordered states and value ranges defined by ARA; a plug-in that completes before polling may legally omit the sequence. Cancellation on sample-access disable or destruction is deterministic. Hosts can await completion outside edit/restoration cycles without blocking a model or realtime thread.

Processing-algorithm properties own persistent IDs and display names for the controller lifetime. Indices are checked against a stable per-controller list. Requests may be declined by the plug-in as ARA permits, but the resulting actual algorithm remains queryable. License requests validate that content types and transformation flags are subsets of factory capabilities; modal activation is never initiated unless explicitly requested by the host.

Host processing catalogs copy both foreign strings before returning. Algorithm selection is edit-scoped and validates the index before dispatch. Licensing requests retain owned content arrays and are constructed only against the loaded factory's advertised content and transformation capabilities. Signal-preservation and playback head/tail results are checked before exposure; head and tail must be finite and nonnegative. Successful source-to-audio-file chunk storage must return a non-empty factory-compatible archive ID, while the returned automatic-open policy is copied immediately.

## Archive I/O and partial persistence

Host archive services adapt random-access `ReadAt`/`WriteAt` traits, not cursor-dependent `Read`/`Write`. All position-plus-length arithmetic is checked. Readers expose archive size and ID; writers report failures without partial success claims. Progress is finite and monotonic within an emitted operation. Oversized 32-bit archives follow the explicit refusal rule in [API Generation Compatibility](09-generation-compatibility.md).

Owned `StoreFilter` and `RestoreFilter` builders validate count/pointer pairs, document-data flags, unique object references, persistent-ID mappings, and graph ownership. Their FFI forms are pinned after allocating owned string/reference arrays and before publishing any interior pointer. A null filter means all matching state. ARA2 readers must report a non-empty archive ID accepted by the loaded factory before dispatch. Partial restore supports multiple archives in one edit cycle and preserves the SDK dependency rule that document data is restored after dependent graph state when archives are split; ARA1 uses the balanced full-document callbacks.

Archive encoding remains application-defined. The bridge owns transport, filtering, lifetime, and error semantics, not a universal plug-in state format.

## ARA audio-file chunks

The library shall parse, edit, and emit the ARA subtree inside AIFF/WAVE iXML. It supports multiple `<audioSource>` entries keyed by `documentArchiveID` and all 2.3 fields: `openAutomatically`, `createDistinctAudioModification`, suggested plug-in metadata, `persistentID`, and MIME-compatible Base64 `archiveData` with or without line feeds. The audited Rust constant for `createDistinctAudioModification` is tested against the C++ header branch because the released C macro branch omits it.

The parser and canonical emitter use this schema (`?` means optional, `*` means repeated):

| Parent / element | Cardinality | Value and default |
|---|---:|---|
| iXML / `ARA` | `0..1` | absence means no ARA archives; duplicates are ambiguous errors |
| `ARA` / `audioSources` | `0..1` | absence is an empty dictionary; the emitter creates one when writing an entry |
| `audioSources` / `audioSource` | `0..*` | one archive record per child |
| `audioSource` / `documentArchiveID` | exactly 1 | non-empty ASCII persistent ID; unique across the dictionary |
| `audioSource` / `openAutomatically` | `0..1` | `true` or `false`; absence defaults to `false` |
| `audioSource` / `createDistinctAudioModification` | `0..1` | `true` or `false`; absence in older chunks defaults to `false` |
| `audioSource` / `suggestedPlugIn` | `0..1` | optional display-only metadata container |
| `suggestedPlugIn` / each of `plugInName`, `lowestSupportedVersion`, `manufacturerName`, `informationURL` | `0..1` each | optional string; empty text normalizes to absent and is omitted canonically |
| `audioSource` / `persistentID` | exactly 1 | non-empty ASCII ID used to match the restored audio source |
| `audioSource` / `archiveData` | exactly 1 | MIME Base64; zero decoded bytes are valid |

Duplicate singleton children, invalid text, or a missing required child are typed errors. Unknown elements and attributes are retained when rewriting but do not affect the typed record. The emitter writes both boolean fields explicitly, uses unwrapped Base64, and preserves dictionary order; parser defaults exist solely for compatibility with older chunks.

Rewriting retains structural templates at the `ARA`, `audioSources`, `audioSource`, and suggested-plug-in layers. Known scalar values are canonicalized in place so extension attributes and vendor elements remain at their original relative positions instead of being collected into a synthetic extension block.

XML parsing is namespace-tolerant where iXML permits, entity-safe, and size-limited. It preserves the semantic content and ordering of unrelated iXML nodes, but does not promise byte-identical whitespace or quoting inside the rewritten iXML chunk. Duplicate archive IDs, invalid booleans/Base64, missing required fields, and oversized decoded data are typed errors.

Container helpers support RIFF/WAVE, RF64/BW64, AIFF, and AIFC, including padding, endianness, large-size tables, unknown chunks, and one unambiguous iXML chunk. RF64/BW64 rewrites retain a table-sized iXML sentinel and update both `riffSize` and its matching `ds64` table entry. iXML allocation, container chunk count, and `ds64` table count are bounded before allocation. Wave64 and multiple conflicting iXML chunks return explicit unsupported/ambiguous errors. A streaming transform writes a caller-provided output and never mutates its input. The path helper writes a same-directory temporary file, preserves permissions, fsyncs file and containing directory where supported, refuses symlinks by default, and atomically replaces only after complete validation. On Windows the validated temporary receives the destination permissions before the destination's read-only attribute is cleared, so permission failure cannot be reported after replacement has already committed; sharing violations restore the original attributes and retain the temporary-path diagnostic.

## Utilities

Rust-native, tested ports are required for:

- sample-position/time conversion and ARA-defined rounding;
- tempo time↔quarter conversion;
- bar-signature quarter/beat/bar conversion;
- pitch, chord, and key-signature interpretation;
- channel-arrangement inspection needed by companion integrations;
- content-reader iterators and update-scope helpers.

Ports must match upstream edge behavior for negative time, boundaries, floating-point tolerance, empty content, enharmonic spelling, German/ASCII naming options, and unusual meter. Utilities unrelated to correct ARA operation are optional and do not block core conformance.

Sample rounding is exactly `floor(x + 0.5)`: an exact `-0.5` sample maps to `0`, while `-1.5` maps to `-1`. Channel layouts are retained as owned typed variants; only a caller that already knows a future companion representation may construct an opaque variant through an explicitly unsafe constructor. Processing-algorithm strings are catalog-owned for the document-controller lifetime, and license requests are validated as content-type and transformation-flag subsets before dispatch.

## Acceptance criteria

All six event types round-trip through host and plug-in readers. Upstream content-reading, content-update, processing-algorithm, archiving, split-archive, drag/drop, and chunk load/save scenarios pass. Parsers survive fuzzing without panic or unbounded allocation. Numeric utilities pass upstream-derived vectors and explicit boundary/property tests.

## Decisions and revisions

- 2026-07-14: Typed readers are primary; validated dynamic readers support runtime discovery.
- 2026-07-14: Archive transport is standardized while archive payload schemas remain plug-in-owned.
- 2026-07-14: Audio-file chunk support includes safe, explicitly bounded container mutation, not only XML constants.
- 2026-07-14: Audit corrected event-pointer invalidation and non-vacuous 2.3 chunk coverage.
- 2026-07-15: Host content and processing facades copy catalog/chunk metadata, scope readers to an exclusive controller borrow, and validate capability subsets, indices, event extents, and realtime-query results before exposing them.
- 2026-07-15: Celemony's content validator defines note volume as finite and nonnegative without an upper bound. Values above unity are retained; negative values remain invalid.
- 2026-07-15: Windows path replacement applies destination permissions to the validated temporary before mutating the destination, eliminating the post-commit permission-restoration failure boundary.
- 2026-07-16: A failed Windows rename must report any subsequent failure to restore the destination's read-only attributes while retaining the validated temporary path.
