# Migrating from 0.1 to 0.2

Version 0.2 replaces the monolithic, partial callback wrapper with focused plug-in, host, companion, core, and raw-ABI crates. This is an intentional API break: the 0.1 `DocumentController` exposed raw pointers, represented only part of the released interface, and could not encode ARA object or edit lifetimes soundly.

## Dependency and Feature Changes

The facade now defaults to plug-in authoring:

```toml
[dependencies]
ara2-bridge = "0.2"
```

Select only the roles used by the consumer:

```toml
ara2-bridge = { version = "0.2", default-features = false, features = ["host"] }
```

| Feature | Facade module | Notes |
| --- | --- | --- |
| `plugin` (default) | `ara2_bridge::plugin` | Plug-in factory, model, analysis, persistence, and callbacks |
| `host` | `ara2_bridge::host` | Host services, factory loading, documents, edits, and extensions |
| `clap` | `ara2_bridge::companion::clap` | Portable CLAP host and plug-in adapter |
| `vst3` | `ara2_bridge::companion::vst3` | Requires the pinned SDK through `ARA_VST3_SDK_DIR` |
| `audio-unit-v2` | `ara2_bridge::companion::audio_unit` | Apple-only; requires `ARA_AUDIO_UNIT_SDK_DIR` |
| `testkit` | `ara2_bridge::testkit` | Mock peers and executable conformance scenarios |

`full-portable` enables plug-in, host, CLAP, and VST3. `full-apple` additionally enables Audio Unit v2. Core validation and raw bindings remain available as `core` and `sys` with no role feature.

## API Mapping

| 0.1 API | 0.2 replacement |
| --- | --- |
| `DocumentController` | Implement the focused `plugin` traits: `DocumentLifecycle`, model-object traits, `ContentProvider`, and `Persistence`/`PartialPersistence` |
| `build_document_controller_instance` | Construct a `Plugin` with `PluginBuilder`, publish it through `FactoryBuilder`, and register it with `PluginRegistryBuilder` |
| `PlaybackRegionHost` / `ModelUpdateController` | Implement `host::PlaybackProvider` and `host::ModelUpdateProvider`, then install them with `HostServicesBuilder` |
| `ArchiveReaderHost` / `ArchiveWriterHost` | Implement `host::ArchivingProvider`; archive sessions are scoped and validated by the host runtime |
| generated `build_ara*_vtable` exports | Use `HostServicesBuilder`; raw layouts remain in `sys` for audited FFI integration only |
| build-time bindgen | Pregenerated, provenance-checked `sys` bindings; maintainers refresh them with `cargo xtask ara generate` |

The old trait and instance-builder names have no compatibility aliases. Preserving them would also preserve unsound raw-pointer ownership and incomplete callback coverage.

## Plug-in Construction

The 0.1 pattern boxed one raw callback trait:

```rust,ignore
let instance = build_document_controller_instance(Box::new(MyController));
```

In 0.2, construction is explicit and fallible. A minimal factory can be configured before a document-controller constructor is added:

```rust
use ara2_bridge::plugin::FactoryBuilder;

let factory = FactoryBuilder::new("org.example.plugin", "org.example.archive")
    .display("Example", "Example Audio", "https://example.invalid", "2.0")
    .build()?;
assert_eq!(factory.id(), "org.example.plugin");
# Ok::<(), ara2_bridge::core::AraError>(())
```

Production plug-ins attach a `PluginBuilder` constructor with `FactoryBuilder::document_controller`. The builder owns callback backing, negotiates ARA generations, contains panics at the ABI, and keeps model identities session-scoped.

## Host Construction and Lifetimes

Hosts build `HostServices`, load a validated `LoadedFactory`, and create a `DocumentSession`. Model mutation occurs through an `EditSession`; object handles cannot be mixed between sessions. Reader leases, restore scopes, render assignments, and controller teardown are explicit guards rather than conventions around opaque C pointers.

When migrating, replace stored raw ARA references with the matching host handle. Do not cache pointers obtained from `sys`, move handles between documents, or retain readers after sample-access revocation. Close documents explicitly and report any `CloseFailure`; `Drop` remains a cleanup backstop, not the primary error channel.

## Companion SDK Configuration

User builds never download SDKs. CLAP declarations are checked in from the pinned source. VST3 and Audio Unit builds require verified local SDK checkouts at the environment variables above. Maintainer ARA conformance uses the official Celemony GitHub repository cached at `.third-party/ARA_SDK`, with no `ARA_SDK_DIR`. Repository CI provisions exact commits through `ci/bootstrap-reference-sdks.sh` only after an explicit license-policy identifier is supplied.
