# API Generation Compatibility

Status: Normative cross-cutting specification  
Depends on: [ABI and Binding Generation](01-abi-and-generation.md), [Core Safety and Dispatch](02-core-safety-and-dispatch.md)  
Last revised: 2026-07-14

## Scope

This spec defines target availability, wire-graph normalization, versioned-struct prefixes, and exact fallbacks across released ARA generations. Host and plug-in runtimes consume this matrix; they do not improvise compatibility independently.

## Generation selection

The host selects one `desiredApiGeneration` within the factory's advertised inclusive range and passes it in `ARAInterfaceConfiguration`. Initialization returns `void`; there is no negotiated result. An invalid selection is a programming error reported through ARA assertions. Each exported factory/`PluginEntry` stores its own selected generation for its balanced initialization interval; it may select a different generation after uninitialization. Only the stable assert-function-pointer cell is process-coordinated, keyed by generation as required by the SDK.

Availability by target:

| Generation | x86/x86_64 | AArch64 | Required behavior |
|---|---:|---:|---|
| 1.0 Draft | yes | no | private legacy compatibility |
| 1.0 Final | yes | no | legacy graph, persistence, extension |
| 2.0 Draft | yes | no | transitional role/graph behavior |
| 2.0 Final | yes | yes | final ARA2 graph and roles |
| 2.X Draft | yes | yes | released development compatibility |
| 2.3 Final | yes | yes | 2.0 Final plus reliable persistent-state notifications |

ARM64 factories must advertise `lowestSupportedApiGeneration >= 2_0_Final`. Generation constants compiled out by the header are not synthesized for that target.

## Normalized graph

The safe API exposes the ARA2 graph: every region sequence has a valid musical context and every playback region has a valid region sequence. ARA2 wire properties enforce those non-null edges.

For ARA1, the adapter suppresses region-sequence calls and maps the playback region's deprecated `musicalContextRef` into an internal synthetic sequence keyed by musical context. Synthetic sequences never cross the ARA1 wire. Destroy/update ordering remains valid on both normalized and wire graphs.

## Versioned interface prefixes

Providers advertise a consecutive represented prefix ending at the last field they expose using `offset + field size`. Every function pointer in that prefix is non-null. Optional function tails are omitted by shortening the prefix; if a later tail is exposed, every intervening function is installed with its specified semantic default. Consumers accept larger future structs, access only known complete fields, and reject sizes below the generation's required minimum or through a partial field. This rule does not change explicitly nullable data pointers or optional ref/interface pairs.

The checked-in [API compatibility manifest](api-compatibility.toml) is the exhaustive normative source generated from the annotated 2.3 header and reviewed with this summary table. It records generation, target family, struct/interface, required terminal field, non-null callbacks, explicitly nullable data pairs, dependency rule, and fallback. Both runtimes consume generated Rust from that one artifact; CI compares it to the header annotations and this table.

| Provider surface | Selected generation | Required terminal field / non-null slots | Truncation, nullable data, or conditional behavior |
|---|---|---|---|
| `ARAFactory` | all | `supportedPlaybackTransformationFlags`; all unmarked fields/callbacks | `supportsStoringAudioFileChunks` is an optional 2.0 Final tail; absence is false |
| `ARADocumentControllerHostInstance` | all | `playbackControllerInterface`; audio and archiving ref/interface pairs non-null | content, model-update, and playback pairs are nullable as whole pairs |
| `ARAAudioAccessControllerInterface` | all | `destroyAudioReader`; all three slots | none |
| `ARAArchivingControllerInterface` | 1.x / 2.0 Draft | `notifyDocumentUnarchivingProgress`; all base slots | no archive-ID query |
| `ARAArchivingControllerInterface` | 2.0 Final+ | `getDocumentArchiveID`; all slots | none; hosts selecting this generation must provide the tail slot |
| `ARAContentAccessControllerInterface` | all | when the optional interface exists: `destroyContentReader`; all slots | absent interface suppresses host-content access |
| `ARAModelUpdateControllerInterface` | 1.x | when present: `notifyAudioModificationContentChanged`; all base slots | entire interface may be absent |
| `ARAModelUpdateControllerInterface` | 2.0 Draft+ | base prefix as above; non-null `notifyPlaybackRegionContentChanged` when its tail is exposed | interface or addendum tail may be absent via null pair/shorter prefix; suppress the unavailable notification |
| `ARAModelUpdateControllerInterface` | 2.3 Final | same, plus non-null `notifyDocumentDataChanged` when that tail is exposed | shorter prefix disables dirty-only document-data optimization, not 2.3 factory conformance |
| `ARAPlaybackControllerInterface` | all | when the optional interface exists: `requestEnableCycle`; all slots | absent interface suppresses playback requests |
| `ARADocumentControllerInterface` | 1.x | `destroyContentReader`; every base slot including deprecated persistence | none |
| `ARADocumentControllerInterface` | 2.0 Draft | `getPlaybackRegionHeadAndTailTime`; base plus region-sequence and head/tail slots | none |
| `ARADocumentControllerInterface` | 2.0 Final / 2.X Draft / 2.3 Final | `storeObjectsToArchive`; all preceding generation-required slots | later algorithm, licensing, chunk, and signal-query slots follow dependencies below |
| `ARAPlugInExtensionInstance` | 1.x | `plugInExtensionInterface`; legacy ref/interface non-null | ARA2 role fields are outside the prefix |
| `ARAPlugInExtensionInstance` | 2.0 Draft+ | `editorViewInterface`; each enabled role has a non-null ref/interface pair | a disabled known role has a null pair; deprecated pair is not used |
| renderer/view interfaces | 2.0 Draft+ | their SDK `MinSize` terminal field; every slot | the whole role interface is absent when that role is disabled |

The 2.0 Final document-controller tail consists of ordered atomic capability cut points. Ending before the four processing-algorithm slots means zero algorithms; exposing any later field requires all four non-null, with the count callback returning zero when algorithms are unsupported. Ending before `isLicensedForCapabilities` means licensed; exposing chunk or signal fields requires a non-null licensing callback that returns true when no licensing policy exists. `storeAudioSourceToAudioFileChunk` must be present and non-null when the factory advertises chunk storing; if a later signal field extends the prefix while the factory flag is false, a non-null chunk callback remains installed but is never called and returns false defensively. Ending before `isAudioModificationPreservingAudioSourceSignal` means false; when exposed it is non-null. Thus no advertised interface prefix contains a null function pointer.

Key versioned data rows are equally normative: ARA1 playback-region properties end at `musicalContextRef`; 2.0 Draft+ properties must include a non-null `regionSequenceRef`, while later `name`/`color` are optional. Region-sequence properties include through non-null `musicalContextRef`; `color` is optional. Audio-source properties require through `merits64BitSamples`; the 2.0 Final channel-arrangement type/pointer tail is optional as a validated pair and defaults to undefined/null. Restore/store filters and processing-algorithm properties use their declared 2.0 Final `MinSize`. Unknown larger tails are ignored.

Document-controller fallbacks are method-specific:

- absent partial-persistence calls use generation-1 begin/end restore and whole-document store only when the selected generation permits;
- absent processing-algorithm count means no host-selectable algorithms; zero has the same effect;
- absent licensing query means assume licensed;
- absent signal-preservation query means false;
- absent playback-region head/tail query means zero head and tail;
- audio-file chunk storing requires both field presence and `ARAFactory::supportsStoringAudioFileChunks`;
- absent 2.3 `notifyDocumentDataChanged` means the host cannot use dirty-only private-document archive optimization;
- absent optional host services suppress their corresponding plug-in behavior as specified, rather than producing a universal unsupported error.

Each fallback is encoded once in generated metadata plus handwritten policy tests.

## Generation-1 sessions and extension roles

Generation-1 restore uses dedicated begin/end restoring calls with the same archive reader reference and is not modeled as an ARA2 edit-plus-restore session. Whole-document store uses the deprecated call. The adapter exposes an equivalent safe persistence operation while preserving the wire call sequence.

The deprecated `ARAPlugInExtensionInterface` set/remove operation maps to both playback-renderer and editor-renderer assignment in the normalized runtime. Opening an ARA1 companion UI is treated as selection according to the SDK. ARA2 requires `assignedRoles` to be a subset of `knownRoles`; violation asserts and binding returns null. For each supported role, the interface is enabled exactly when `!known(role) || assigned(role)`: hosts that do not know a role receive backward-compatible behavior, known-but-unassigned roles are suppressed, and known-and-assigned roles are exposed.

## ARA 2.3 document dirty semantics

A plug-in may advertise `kARAAPIGeneration_2_3_Final` only if every persistent-state change is reliably classified and queued: audio-source changes use `notifyAudioSourceContentChanged`, audio-modification changes use `notifyAudioModificationContentChanged`, playback-region changes use `notifyPlaybackRegionContentChanged`, and private opaque document changes use `notifyDocumentDataChanged`. Host-originated property/content updates and successful restoration suppress echo notifications unless conversion, recovery, or plug-in-derived state creates a real additional change.

Pending changes coalesce per object/category and are delivered only while servicing `notifyModelUpdates`, then clear after successful delivery. A category changed again during delivery remains pending for the next flush. Hosts receiving a callback mark the corresponding partial archive dirty and may skip unchanged saves. If the optional model-update interface or a compatible tail callback is absent, the plug-in still tracks changes but suppresses that unavailable call; the host treats the affected archive category as potentially dirty and loses only the optimization. This does not prevent the factory from advertising 2.3.

## 32-bit archive behavior

On 32-bit targets, archive sizes and positions exceeding `usize::MAX` are rejected before allocation or pointer arithmetic with an explicit `ArchiveTooLargeForTarget` error. No truncation is permitted. Cross-architecture tests create a 64-bit-declared sparse archive and verify deterministic refusal on the tier-2 32-bit job.

## Acceptance criteria

The conformance kit runs generation-specific host/plug-in pairings for every row available on the target. Wire traces prove the correct graph, persistence calls, role mapping, prefix size, fallback result, and dirty-notification behavior. Role tests cover unknown, known-unassigned, known-assigned, and invalid assigned-not-known inputs. Dirty tests mutate every persistent category with full, truncated, and absent model-update interfaces. ARM64 builds contain no legacy constants or paths that the header excludes.

## Decisions and revisions

- 2026-07-14: Audit introduced a single compatibility matrix and an ARA1 synthetic-sequence normalization layer.
- 2026-07-14: Audit made generation state factory-local, corrected role enablement, and expanded 2.3 reliability to every persistent-state category.
