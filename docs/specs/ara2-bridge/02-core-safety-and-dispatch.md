# Core Safety and Dispatch

Status: Normative component specification  
Depends on: [System Overview](00-overview.md), [ABI and Binding Generation](01-abi-and-generation.md)  
Last revised: 2026-07-15

## Scope

`ara2-bridge-core` owns shared safe types, validation, callback dispatch, diagnostics, thread/lifecycle primitives, and the narrow unsafe boundary used by host and plug-in runtimes.

## References and ownership

Each ARA opaque reference has a distinct Rust newtype. Cross-kind conversion is impossible without explicit unsafe code. Null is represented by `Option`, never a valid handle. Borrowed peer references carry the owning session lifetime where practical; stored or FFI-returned references use runtime-owned stable allocations and typed registries.

Registries shall detect stale, foreign, double-destroyed, and wrong-kind references in checked builds. One explicit `RegistrySession` identifies the document and is shared by every typed registry in that document; registry kind and cell index remain separate parts of handle identity. Removing an object invalidates its handle before user destruction hooks run. Handle cells are not reused within a document session; compact tombstones remain until close. A configurable hard cap, default 1,048,576 cells per typed registry, bounds memory and fails creation before crossing FFI when exhausted. Public model handles are neither `Send` nor `Sync` unless a method-specific contract proves otherwise.

## Sized structures and pointers

An arbitrary foreign address cannot be proven readable in portable Rust. Every exported callback therefore has an unsafe ABI precondition: each non-null inbound pointer is readable for its documented object or array extent, including the complete extent advertised by `structSize`, and writable where the API requires output. After that precondition is met, dispatch validates nullability, alignment where applicable, minimum/implemented size, required nested pointers, count/pointer agreement, enum semantics, and finite numeric values before reading dependent data. Packed fields are copied through generated unaligned accessors into aligned safe mirrors. Optional tail fields are read only when `structSize` covers the complete field. Borrowed names and IDs are copied when their documented lifetime does not outlive the call.

Outbound builders initialize every byte, set the correct `structSize`, retain backing strings/arrays for the required lifetime, and expose no pointer into movable storage. Variable arrays use checked length conversion and reject arithmetic overflow.

Opaque channel-arrangement storage is copied only after its complete extent has been resolved from the declared data type. Core handles fixed-size layouts and core-visible variable layouts whose size follows directly from the ARA header. CoreAudio layouts and CLAP ambisonic payloads require the corresponding companion adapter to validate and supply their extent; before that adapter is present, core returns `Unsupported` rather than guessing a size or retaining a borrowed pointer.

## Errors and diagnostics

Expected failures use a non-exhaustive `AraError` with ABI, invalid argument, invalid state, invalid thread, unsupported capability, peer failure, poisoned instance, archive, and companion-integration categories. Methods returning an ARA boolean or nullable reference map errors to the mandated sentinel while recording diagnostics. APIs capable of reporting failure return `Result` on the Rust side.

The global ARA assert facility is a stable pointer to a function-pointer cell, not merely a callback. A process-scoped initialization coordinator owns one cell per active generation until the last matching uninitialization and ensures all factories initialized for that generation observe the same address. Generation selection itself remains local to each initialized factory/`PluginEntry`. Diagnostics include category, interface, method, document/instance identity, and a static or owned message. Logging is pluggable and never required for correctness.

## Panic and foreign-exception containment

No Rust panic may unwind through C. Every exported callback is wrapped in `catch_unwind`; on panic it records a diagnostic, poisons the affected runtime, and returns the method's safe sentinel. Only destruction and diagnostic operations remain permitted on a poisoned instance. If a platform configuration cannot guarantee unwind containment, the callback aborts rather than crossing the ABI.

C++ shims catch all foreign exceptions and translate them to failure before returning to Rust. Destructors are idempotence-guarded at the dispatch layer even when duplicate destruction remains a caller contract violation.

## Threading and realtime behavior

The runtime records the ARA model thread at document creation and checks model-thread-only calls in debug and validation modes. Render-capable objects explicitly declare their concurrency contract. Realtime paths must not allocate, block, lock an unbounded mutex, perform file I/O, panic, or emit synchronous logs.

Thread markers and scoped guards represent model editing, restoration, sample access, and render activation. `getPlaybackRegionHeadAndTailTime` uses a dedicated shareable realtime query view because ARA permits it on realtime/offline render threads; its implementation is bounded, allocation-free, and cannot reach mutable model-thread state. An escape hatch may be unsafe, but its documentation must state the exact ARA rule the caller assumes.

## Dispatch generation

Mechanical callback shims and vtable assignment may be generated from the ABI coverage manifest. Generated shims perform only pointer recovery, common validation, panic containment, argument conversion, and delegation to a named safe trait method. Behavioral defaults remain handwritten.

Required functions are always installed through the generation's required prefix. Optional tail groups extend `structSize` only through the last consecutively represented field, and every function pointer inside that prefix is non-null. Optionality is expressed by ending the prefix before a tail group or, when a later group is exposed, by installing the specified non-null default callback for intervening methods. Host wrappers check complete-field presence and use the manifest's method-specific result: unsupported error, false, zero, assume-licensed, or factory-capability gate. Nullable data/ref/interface pairs remain nullable only where the API explicitly says so.

## Safety documentation and tests

Every unsafe function has a `# Safety` section. Every unsafe block has a local invariant comment. The core crate denies `unsafe_op_in_unsafe_fn`, missing safety docs, and undocumented public items. Compile-fail tests cover lifetime and thread misuse; Miri covers registries, destruction, and callback recovery; fuzzing covers valid allocated buffers containing malformed sized structs, counts, content events, and archive/chunk data. Tests for arbitrary invalid addresses run only in isolated subprocesses or sanitizer harnesses, because dereferencing such an address is outside the in-process Rust safety contract.

## Acceptance criteria

Normal host and plug-in examples contain no unsafe code. Panics cannot cross the ABI; malformed values contained in caller-valid foreign storage are rejected before dependent dereferences. Arbitrary dangling or unreadable foreign pointers remain a documented C-caller contract violation. Lifecycle misuse produces a deterministic error/assert. Realtime-designated calls satisfy the realtime checklist and have targeted tests.

## Decisions and revisions

- 2026-07-14: Runtime-owned typed registries selected over exposing application pointers as ARA references.
- 2026-07-14: Panic containment poisons an instance and permits cleanup, rather than silently continuing after inconsistent state.
- 2026-07-14: Audit replaced full-size nullable vtables with consecutive implemented prefixes and method-specific fallbacks.
- 2026-07-15: Channel-arrangement ownership requires a validated data-type-specific extent; companion-defined variable payloads remain unsupported until their adapter supplies it.
- 2026-07-15: A document-scoped `RegistrySession` is shared across its typed registries so cross-kind filters can prove common graph ownership.
