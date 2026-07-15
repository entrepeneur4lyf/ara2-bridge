use ara2_bridge_core::{ApiGeneration, PlaybackTransformationFlags};
use ara2_bridge_plugin::{FactoryBuilder, FactoryCapabilities, PluginRegistry};
use ara2_bridge_sys::{ARAAssertFunction, ARAInterfaceConfiguration};
use std::sync::{Mutex, MutexGuard};

static FACTORY_PROCESS_STATE: Mutex<()> = Mutex::new(());

fn process_state() -> MutexGuard<'static, ()> {
    FACTORY_PROCESS_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn each_factory_entry_has_independent_generation_state() {
    let _process_state = process_state();
    let registry = PluginRegistry::builder()
        .factory(factory(
            "one",
            ApiGeneration::V2Final,
            ApiGeneration::V23Final,
        ))
        .factory(factory(
            "two",
            ApiGeneration::V2Final,
            ApiGeneration::V23Final,
        ))
        .build()
        .unwrap();
    let mut first_assert: ARAAssertFunction = None;
    let mut second_assert: ARAAssertFunction = None;
    registry
        .entry("one")
        .unwrap()
        .initialize(ApiGeneration::V2Final, &raw mut first_assert)
        .unwrap();
    registry
        .entry("two")
        .unwrap()
        .initialize(ApiGeneration::V23Final, &raw mut second_assert)
        .unwrap();
    assert_eq!(
        registry.entry("one").unwrap().generation(),
        Some(ApiGeneration::V2Final)
    );
    assert_eq!(
        registry.entry("two").unwrap().generation(),
        Some(ApiGeneration::V23Final)
    );
}

#[test]
fn raw_factory_pointer_and_callbacks_are_stable_across_balanced_reinitialization() {
    let _process_state = process_state();
    let registry = PluginRegistry::builder()
        .factory(factory(
            "stable",
            ApiGeneration::V2Final,
            ApiGeneration::V23Final,
        ))
        .build()
        .unwrap();
    let factory = registry.factory("stable").unwrap();
    assert_eq!(factory.as_raw(), factory.as_raw());
    let raw = factory.raw_copy();
    // SAFETY: each field is copied from a live packed record with an unaligned read.
    let initialize =
        unsafe { std::ptr::addr_of!(raw.initializeARAWithConfiguration).read_unaligned() };
    // SAFETY: same packed-record copy invariant as above.
    let uninitialize = unsafe { std::ptr::addr_of!(raw.uninitializeARA).read_unaligned() };
    // SAFETY: same packed-record copy invariant as above.
    let create =
        unsafe { std::ptr::addr_of!(raw.createDocumentControllerWithDocument).read_unaligned() };
    assert!(initialize.is_some());
    assert!(uninitialize.is_some());
    assert!(create.is_some());

    let mut assertion: ARAAssertFunction = None;
    let configuration = ARAInterfaceConfiguration {
        structSize: std::mem::size_of::<ARAInterfaceConfiguration>(),
        desiredApiGeneration: ApiGeneration::V23Final.as_raw(),
        assertFunctionAddress: &raw mut assertion,
    };
    // SAFETY: the factory and complete configuration remain live for the callback.
    unsafe { initialize.unwrap()(&raw const configuration) };
    assert_eq!(
        registry.entry("stable").unwrap().generation(),
        Some(ApiGeneration::V23Final)
    );
    // SAFETY: this balances the successful initialization above.
    unsafe { uninitialize.unwrap()() };
    assert_eq!(registry.entry("stable").unwrap().generation(), None);

    registry
        .entry("stable")
        .unwrap()
        .initialize(ApiGeneration::V2Final, &raw mut assertion)
        .unwrap();
    registry.entry("stable").unwrap().uninitialize().unwrap();
}

#[test]
fn invalid_ranges_capabilities_duplicates_and_assert_addresses_are_rejected() {
    let _process_state = process_state();
    assert!(FactoryBuilder::new("bad", "archive.bad")
        .generations(ApiGeneration::V23Final, ApiGeneration::V2Final)
        .build()
        .is_err());
    assert!(FactoryBuilder::new("bad", "é")
        .generations(ApiGeneration::V2Final, ApiGeneration::V23Final)
        .build()
        .is_err());
    assert!(FactoryBuilder::new("bad", "archive.bad")
        .capabilities(
            FactoryCapabilities::default()
                .with_playback_transformations(PlaybackTransformationFlags::REFLECT_TEMPO,)
        )
        .build()
        .is_err());

    let registry = PluginRegistry::builder()
        .factory(factory(
            "duplicate",
            ApiGeneration::V2Final,
            ApiGeneration::V23Final,
        ))
        .factory(factory(
            "duplicate",
            ApiGeneration::V2Final,
            ApiGeneration::V23Final,
        ))
        .build();
    assert!(registry.is_err());

    let registry = PluginRegistry::builder()
        .factory(factory(
            "first",
            ApiGeneration::V2Final,
            ApiGeneration::V23Final,
        ))
        .factory(factory(
            "second",
            ApiGeneration::V2Final,
            ApiGeneration::V23Final,
        ))
        .build()
        .unwrap();
    let mut first: ARAAssertFunction = None;
    let mut dirty: ARAAssertFunction = None;
    registry
        .entry("first")
        .unwrap()
        .initialize(ApiGeneration::V2Final, &raw mut first)
        .unwrap();
    assert!(registry
        .entry("second")
        .unwrap()
        .initialize(ApiGeneration::V2Final, &raw mut dirty)
        .is_err());
    assert!(registry
        .entry("second")
        .unwrap()
        .initialize(ApiGeneration::V2Final, std::ptr::null_mut())
        .is_err());
}

fn factory(id: &str, low: ApiGeneration, high: ApiGeneration) -> ara2_bridge_plugin::Factory {
    FactoryBuilder::new(id, format!("archive.{id}"))
        .display(id, "Example", "https://example.test", "1.0")
        .generations(low, high)
        .build()
        .unwrap()
}
