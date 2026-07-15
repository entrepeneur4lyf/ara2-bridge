# Host Runtime

Status: Normative component specification  
Depends on: [Core Safety and Dispatch](02-core-safety-and-dispatch.md)  
Last revised: 2026-07-15

## Scope

`ara2-bridge-host` supplies complete DAW-host authoring support: host callback interfaces, plug-in factory and controller dispatch, host model identities, edit/restoration orchestration, plug-in extension control, and checked lifecycle management. Companion discovery/loading is specified separately.

## Host service interfaces

`HostServicesBuilder` constructs a stable `ARADocumentControllerHostInstance` and owns each registered interface/reference pair. All ARA 2.3 host methods are covered:

- Audio access: create reader, read planar/non-interleaved per-channel sample buffers, destroy reader.
- Archiving: get size, read bytes, write bytes, archiving progress, unarchiving progress, document archive ID.
- Content access: availability/grade/reader creation for musical contexts and audio sources; reader event count/data; reader destruction.
- Model updates: source analysis progress, source/modification/playback-region content changes, and document-data change.
- Playback control: start, stop, set position, set cycle range, and enable cycle.

Required services are enforced by the selected API generation and plug-in capabilities. Optional services are represented by absent interface pointers, never a non-null zeroed vtable. Callback dispatch applies the common validation, panic, threading, and poisoning policy.

## Plug-in dispatch

Version-aware wrappers expose every document-controller operation listed in the generated plug-in coverage manifest. Before each call they validate controller lifetime, complete-field presence, argument ownership, normalized/wire graph state, and thread. Missing tail fields use the method-specific compatibility result; a null callback inside an advertised prefix or a missing required prefix rejects the plug-in instance.

Factory loading validates generation range, stable metadata pointers, archive IDs, count/pointer pairs, and factory/controller identity. Initialization is balanced across every success and failure path. Host code may inspect capabilities before creating a document.

Factory metadata is copied into host ownership before initialization and includes all identity/display strings, archive compatibility IDs, analyzable content types, transformation flags, and chunk-storage support. Empty required IDs, duplicate array entries, unknown content types or flag bits, malformed count/pointer pairs, and non-null callbacks missing from represented prefixes are rejected. Processing-algorithm callbacks are accepted only as one complete prefix group; chunk storage must be callable whenever the factory advertises it.

The 54 raw host-to-plug-in call shells are generated from the canonical x86_64 binding and joined to generated per-target field extents. `host-dispatch --check` is non-mutating and rejects absent or stale derivatives. The generated layer performs only represented-field and callback-presence validation; handwritten controller/document wrappers remain responsible for thread, graph, ownership, argument-backing, and semantic fallback policy.

## Host document model

`DocumentSession` owns host-side `Document`, `MusicalContext`, `RegionSequence`, `AudioSource`, `AudioModification`, and `PlaybackRegion` records plus the plug-in references returned for them. It maintains the required ARA2 edges and uses the compatibility spec's normalized graph for ARA1 peers.

Mutations occur through scoped `EditSession<'_>`. The guard begins editing once, exposes ordered create/update/deactivate/destroy operations, and ends editing exactly once. Since most plug-in calls return `void`, `finish()` reports only locally observable validation, poison, assert, and transport failures; it does not claim foreign atomicity. `Drop` performs best-effort balancing and records diagnostics. ARA2 object restoration is an operation on the edit guard, so one edit can span multiple archives; ARA1 uses a dedicated restoration guard backed by the legacy begin/end callbacks. APIs prevent storing archives while editing.

Destruction is explicit and leaf-first. `DocumentSession::close` revokes content/audio readers, removes extension role assignments, then destroys playback regions, modifications, sources, sequences, contexts, and finally the controller. Checked teardown continues after individual failures and returns an ordered `CloseError`; Drop is a guarded fallback, not the primary error-reporting path.

## Host object data

Host references point to stable runtime-owned records. Create operations register a provisional record before crossing FFI so synchronous plug-in callbacks can legally resolve the host reference. Host services track whether audio, content, or model-update callbacks observe that provisional address. A non-null returned plug-in reference commits it; an error/null rolls it back when unobserved, but an observed failed create poisons the session before guarded teardown continues. Void update/destroy calls update local state according to the issued command and record assertions/panics, but cannot promise rollback of foreign state.

Host and plug-in model references occupy separate identity domains. Stored properties retain host references for local graph validation; each outbound region-sequence or playback-region call builds temporary peer properties containing the corresponding plug-in references. A typed foreign `ModelRef` may only be admitted from a non-null ABI pointer under an explicit unsafe lifetime, kind, thread, and stability contract. Sending a host pointer where the plug-in expects its own reference is always invalid.

Audio-source and audio-modification persistent IDs share the document conflict set, including clones and imported updates. Updates validate a replacement ID against that set before FFI and commit the replacement only after locally successful dispatch. Deactivated modifications must have no playback regions; a source may be deactivated only after all its modifications, and redo uses the reverse order. Sample access is disabled initially and may be changed inside or outside an edit scope.

For ARA 1, playback properties retain a host musical-context edge while the plug-in adapter owns one synthetic region sequence per context. The ABI property is normalized to the plug-in musical-context reference. ARA 2 draft/final controllers use explicit region-sequence records and peer references.

Property builders own names, persistent IDs, channel arrangements, colors, and referenced arrays across each call. Persistent IDs are unique within a document and conflict-aware during import. Channel arrangements use owned variants for undefined, VST3 speaker arrangement, Core Audio layout, AAX stem format, CLAP channel map, and CLAP ambisonic data; unsupported/unknown data types are preserved as explicitly unsafe opaque bytes or rejected before a safe call.

Audio readers expose checked planar/non-interleaved channel buffers for 32- or 64-bit samples. They validate source access state, channel counts, and buffer lengths; out-of-range portions before or after the source are filled with silence and are not errors. A failed read fills every requested sample with silence before returning false. `readAudioSamples` is potentially blocking, is allowed only on non-realtime threads (including offline render threads), and permits at most one concurrent call per reader while distinct readers may run concurrently. Disabling access synchronously waits for in-flight reads and forces reader teardown as required.

## Extension control

Host wrappers bind companion instances to a document controller with declared known/assigned role flags and validate coherent, size-complete returned reference/interface pairs. Assigned roles must be known and represented; known but unassigned roles must be absent, while unknown supported roles follow the SDK enablement formula. RAII assignments manage plug-in peer references for playback regions and region sequences, including the ARA1 set/remove mapping. Editor-view notifications resolve checked graph handles into temporary peer arrays whose backing remains valid through the call.

Renderer assignments are confined to the document's model thread and rejected while the companion instance is in render state; editor-view notifications remain model-thread checked and may run inside editing. Document close shuts down every registered extension before referenced graph objects, and later RAII-guard drops become inert.

## Content and processing facade

Document sessions expose model-update flushing plus typed content availability, grade, analysis-request, and reader operations for audio sources, audio modifications, and playback regions. Readers own a mutable controller borrow for their complete lifetime, validate every returned event extent before exposing typed data, and are revoked before explicit document teardown. Processing-algorithm catalogs are copied into Rust ownership and indices are range-checked. Algorithm changes require an edit guard; licensing, signal-preservation, chunk-storage, and playback head/tail queries require a live session outside editing. Chunk storage accepts only archive IDs advertised by the loaded factory.

## Failure and recovery

Peer false/null returns become typed errors with the provisional/commanded state rules above. Invalid plug-ins are quarantined at their document or extension instance; one plug-in cannot poison global host services. The ABI cannot prove a foreign plug-in rolled back partial mutation, so poisoning prevents further normal work after an assert, panic, or impossible return. Archive restore may report partial recovery exactly as ARA permits, with diagnostics retaining both transport and plug-in decode failures.

## Acceptance criteria

The Rust TestHost can run every upstream TestHost scenario against both the Rust TestPlugIn and Celemony's C++ TestPlugIn where buildable. Misordered graph operations fail before crossing FFI. Every host callback and every plug-in dispatch method has positive, absent-slot, malformed-input, panic, and teardown coverage appropriate to its signature.

## Decisions and revisions

- 2026-07-14: Host creation uses provisional records; void foreign calls do not claim transactional rollback.
- 2026-07-14: Explicit close/finish methods carry observable errors; Drop only guarantees best-effort balance and cleanup.
- 2026-07-15: Host service aggregates own published state and vtables through fixed-address raw ownership; moving a Rust `Box` owner after publishing an interior pointer is forbidden by the strict-provenance model.
- 2026-07-15: Host content providers publish validated immutable typed snapshots. Named-event pointers target self-owned string backing, so snapshots may be read concurrently until synchronous reader destruction.
- 2026-07-15: Audio-reader revocation first removes and marks all source readers inactive, then waits on each per-reader lock. This prevents a queued read from starting after access disable while preserving concurrency between distinct readers.
- 2026-07-15: Factory and controller records are identity-checked by exact raw factory pointer. A controller guard borrows its initialized factory and host services, destroys the controller exactly once, and therefore cannot outlive either dependency.
- 2026-07-15: ABI producers pin factory string/array backing before publishing interior pointers. Moving a `CString`, `Vec`, or owning `Box` after taking an interior pointer is treated as invalid even when its heap address appears unchanged; strict Miri enforces this rule.
- 2026-07-15: Host dispatch code generation owns repetitive signatures, offsets, extents, and raw calls for all 54 slots. Safe semantic wrappers remain handwritten and consume the generated layer rather than duplicating ABI signatures.
- 2026-07-15: Host-owned and plug-in-owned graph pointers are distinct reference domains. Outbound dependent properties are rebuilt with checked peer references; local records continue to retain host references.
- 2026-07-15: A factory may implement ARA 2 Final optional controller tails while also supporting legacy generations. Legacy controller instances expose only their generation prefix, and host validation requires advertised audio-file chunk storage only where that callback exists.
- 2026-07-15: Undo-history activation is a tracked graph state with modification-before-source deactivation and source-before-modification reactivation. Sample-access enablement is tracked separately and remains legal outside editing.
- 2026-07-15: Typed content readers and processing queries retain an exclusive document-controller borrow, preventing overlapping mutation or teardown while plug-in reader state is live.
