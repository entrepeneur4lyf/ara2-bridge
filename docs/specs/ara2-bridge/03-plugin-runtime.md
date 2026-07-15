# Plug-in Runtime

Status: Normative component specification  
Depends on: [Core Safety and Dispatch](02-core-safety-and-dispatch.md)  
Last revised: 2026-07-14

## Scope

`ara2-bridge-plugin` supplies the complete plug-in side: global entry and factory, document-controller runtime, ARA model objects, content production, persistence hooks, playback/editor extension roles, and host-interface clients.

## Authoring model

The public API shall use focused capability traits assembled by `PluginBuilder`, not a single 54-method trait. The runtime owns C instances, vtables, object registries, host wrappers, and lifecycle state. Application code owns domain data and implements only selected semantic hooks.

Required model operations are grouped into `DocumentLifecycle`, `DocumentModel`, `MusicalContexts`, `RegionSequences`, `AudioSources`, `AudioModifications`, `PlaybackRegions`, and `ContentProvider`. Optional groups are `AnalysisProvider`, `PartialPersistence`, `ProcessingAlgorithms`, `Licensing`, `AudioFileChunkWriter`, and `SignalPreservationQuery`. The crate provides documented no-content/no-analysis defaults where ARA permits them; it never fabricates success or silently discards persistent data.

## Global entry and factory

A binary-level registry may export multiple factories, but each immutable `ARAFactory` has its own `PluginEntry`, initialization state, and selected-generation cell. Since ARA factory callbacks carry no factory reference, the bridge assigns each live factory a dedicated static callback trampoline; one loaded binary supports at most 64 simultaneously live factories and rejects the 65th during construction. An entry validates `desiredApiGeneration` against only its factory's declared range and target availability, installs diagnostics, and rejects duplicate or misordered initialize/uninitialize calls independently of sibling entries. Initialization returns no negotiation result; invalid selections are programming errors reported through the assert facility. The only shared process state is the generation-keyed assert-function-address coordinator.

`FactoryBuilder` owns all strings, archive IDs, compatible archive IDs, analyzable content types, transformation flags, and callbacks for their entire required lifetime. Factory identity changes are documented whenever capabilities or archive compatibility changes. Document creation validates the host instance before invoking application code and returns a stable `ARADocumentControllerInstance` until destruction.

## Document-controller coverage manifest

The runtime shall classify every slot in the generated ordered ARA 2.3 manifest, grouped as follows:

- Lifecycle/update: `destroyDocumentController`, `getFactory`, `beginEditing`, `endEditing`, `notifyModelUpdates`, `updateDocumentProperties`.
- Generation-1 persistence compatibility: `beginRestoringDocumentFromArchive`, `endRestoringDocumentFromArchive`, `storeDocumentToArchive`.
- Musical contexts: create, update properties/content, destroy.
- Region sequences: create, update properties, destroy.
- Audio sources: create, update properties/content, enable sample access, deactivate for undo, destroy.
- Audio modifications: create, clone, update properties, deactivate for undo, destroy.
- Playback regions: create, update properties, query head/tail time, destroy.
- Content readers for audio sources, audio modifications, and playback regions: availability, grade, and reader creation for each; event count/data access; reader destruction.
- Analysis: incomplete query and analysis request.
- ARA 2 partial persistence: restore and store filtered objects.
- Processing algorithms: count, properties, active algorithm query, and algorithm request.
- Licensing: capability license query and optional modal activation request.
- Audio-file chunks: store an audio source and return archive ID/open policy.
- Signal query: whether an audio modification preserves the source signal.

The generated coverage manifest shall name every actual C slot and its safe delegate. Each selected capability extends a consecutive represented prefix, and every callback inside the advertised `structSize` is non-null. A missing tail capability shortens the prefix unless a later capability is enabled, in which case intervening callbacks use the compatibility spec's non-null semantic defaults. Optional behavior and older peers use the exact prefix and fallback rules in [API Generation Compatibility](09-generation-compatibility.md). Deprecated calls preserve generation-1 call ordering rather than being treated as generic ARA2 edits.

## Model graph and lifecycle

The runtime maintains document ownership and edges:

```text
Document
├── MusicalContext
├── RegionSequence ── required MusicalContext
└── AudioSource
    └── AudioModification
        └── PlaybackRegion ── required RegionSequence
```

Creation allocates a stable typed reference before calling the user hook and rolls back on failure. Updates copy ephemeral properties before delegation. Destruction enforces leaf-to-root ordering and invalidates references. Deactivation is distinct from destruction and retains persistent identity. Cloning creates independent modification state associated with the same source.

Graph mutations require an editing guard. ARA2 restoration uses an editing guard plus restoration state; generation-1 restoration uses its dedicated begin/end session. Multiple partial archives may be restored in one ARA2 cycle. `endEditing` finalizes deferred work before queued model notifications are exposed. `notifyModelUpdates` is the only normal flush point outside editing/restoration. The compatibility layer supplies synthetic internal region sequences for the ARA1 wire graph.

For 2.3 Final, every persistent-state mutation is tracked in its owning category: audio source, audio modification, playback region, or private document data. Host-originated/restoration changes suppress echo notifications unless recovery, conversion, or derived state creates a real additional change. Notifications are coalesced and sent only from `notifyModelUpdates` as specified in the compatibility matrix.

## Content and analysis

Content readers are immutable snapshots whose event type matches the requested `ARAContentType`. They remain valid until explicit destruction and prevent conflicting controller operations as required by the selected generation. A raw event pointer is valid only until the next event-data call or reader destruction. Safe providers therefore copy into bridge-owned scratch storage per call; safe consumers receive owned event values or use a lending closure that cannot retain a borrow across another call.

Analysis jobs may run asynchronously, but start/cancel state, sample-access revocation, progress ordering, and final content notifications must follow ARA rules. Disabling source sample access synchronously prevents later reads through existing readers and cancels affected analysis. No worker callback may access a destroyed or deactivated object.

## Plug-in extension roles

Companion-specific CLAP, VST3, Audio Unit, or future AAX entry points bind an instance to the runtime. The binding produces `ARAPlugInExtensionInstance` with the deprecated ARA1 extension and/or any registered ARA2 role combination:

- playback renderer: add/remove playback region;
- editor renderer: add/remove playback region and region sequence;
- editor view: selection and hidden-region-sequence notifications.

Role flags are validated against known and assigned companion-instance roles. There is no generic unbind callback. Controller and extension handles share a reference-counted inner allocation; destroying either side tombstones its view while retaining storage until both companion and controller owners are gone. Graph-referencing role calls are invalid after controller destruction, but interface memory remains valid until companion destruction. Renderer code declares realtime and model-thread portions separately.

## Host clients

Validated wrappers provide safe access to audio samples, archives, model-update notifications, and playback requests. They preserve peer lifetimes and thread restrictions and never cache host pointers past the document-controller lifetime. Audio-reader creation uses the same current-audio-source or `endEditing` call-scope gate as the SDK, but a successfully created audio reader is long-lived and may be used later under its per-reader threading rules.

Host content access is intentionally not a controller-lifetime client. The dispatcher creates a non-storable `HostContentScope<'call>` token only while servicing a musical-context or audio-source create/update callback, restricted to that callback's current host object identity, or while servicing `endEditing`, where any live object is allowed. Availability, grade, and reader creation require this token; readers and event borrows cannot escape the callback scope. Calling the raw host content interface from any other stack frame is rejected before crossing FFI.

## Acceptance criteria

The Rust TestPlugIn can express every behavior exercised by upstream TestPlugIn without unsafe application code. All 54 callbacks have dispatch tests, malformed inputs are rejected, graph ordering tests pass, optional capability slots match registration, and full factory/controller/extension teardown is leak-free under Miri and sanitizers.

## Decisions and revisions

- 2026-07-14: Capability composition replaces expansion of the original monolithic `DocumentController` trait.
- 2026-07-14: The runtime owns ARA identity and graph integrity; application hooks own domain behavior.
- 2026-07-14: Audit corrected required ARA2 graph edges, content-event invalidation, companion binding, and 2.3 dirty-state delivery.
- 2026-07-15: Implementation audit made dispatcher-scoped host content/audio access explicit in model hooks. Long-lived host audio readers are owned handles registered with the controller and synchronously revoked on access disable, source destruction, or controller teardown; retained handles become inert instead of retaining callable foreign pointers past their lifetime.
- 2026-07-15: The plug-in crate directly depends on both the safe core and raw sys layer because it owns ABI vtables and callback entry points. This remains an acyclic downward dependency; the workspace graph test now records the implemented edge instead of assuming all raw ABI use could be hidden behind core.
- 2026-07-15: Native TestHost interoperability moved the analysis-start callback into the `notifyModelUpdates` flush, ahead of queued update/completion events. Starting analysis must never call the host's model-update interface directly from the request callback.
- 2026-07-15: Audio-file chunk storage validates the application-selected archive ID by value, then returns the exact matching pointer published by the factory. A temporary equal `CString` does not satisfy the ARA persistent-ID identity contract.
