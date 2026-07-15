use ara2_bridge_companion::{
    CompanionFactory, CompanionProcessorBinding, CompanionRoles, LifecycleEvent,
};
use ara2_bridge_sys::{ARADocumentControllerRef, ARAFactory};
use std::mem::size_of;

fn factory_fixture() -> Box<ARAFactory> {
    // The neutral boundary treats the factory as opaque; callback completeness is validated by the
    // host/plugin runtimes when the pointer is consumed.
    Box::new(ARAFactory {
        structSize: size_of::<ARAFactory>(),
        ..unsafe { std::mem::zeroed() }
    })
}

fn controller_fixture() -> (Box<u8>, ARADocumentControllerRef) {
    let mut storage = Box::new(0_u8);
    let reference = (&mut *storage as *mut u8).cast();
    (storage, reference)
}

fn fixture<'a>(factory: &'a ARAFactory) -> CompanionProcessorBinding<'a> {
    // SAFETY: the boxed factory remains at a stable address for the returned binding lifetime.
    let factory = unsafe { CompanionFactory::from_raw("test.factory", factory) }.unwrap();
    CompanionProcessorBinding::new([factory], CompanionRoles::all()).unwrap()
}

#[test]
fn binding_must_precede_processor_boundaries() {
    let factory = factory_fixture();
    let processor = fixture(&factory);
    assert!(processor.observe(LifecycleEvent::StateLoad).is_err());
    let (_controller_storage, controller) = controller_fixture();
    // SAFETY: controller storage remains live through the attempted binding.
    assert!(
        unsafe { processor.bind(controller, CompanionRoles::all(), CompanionRoles::all()) }
            .is_err()
    );
}

#[test]
fn binding_is_one_shot_and_validates_roles() {
    let factory = factory_fixture();
    let processor = fixture(&factory);
    let (_controller_storage, controller) = controller_fixture();
    // SAFETY: controller storage remains live until the controller binding is dropped.
    let bound = unsafe {
        processor
            .bind(controller, CompanionRoles::all(), CompanionRoles::all())
            .unwrap()
    };
    assert!(
        unsafe { processor.bind(controller, CompanionRoles::all(), CompanionRoles::all()) }
            .is_err()
    );
    assert_eq!(bound.enabled_roles(), CompanionRoles::all());

    let other_factory = factory_fixture();
    let other = fixture(&other_factory);
    assert!(unsafe {
        other.bind(
            controller,
            CompanionRoles::PLAYBACK_RENDERER,
            CompanionRoles::EDITOR_RENDERER,
        )
    }
    .is_err());
}

#[test]
fn factory_lookup_lifecycle_and_both_teardown_orders_are_deterministic() {
    let factory = factory_fixture();
    let processor = fixture(&factory);
    assert_eq!(processor.factory_count(), 1);
    assert_eq!(processor.factory(0).unwrap().id(), "test.factory");
    assert_eq!(
        processor.factory_for_id("test.factory").unwrap().as_raw(),
        &*factory as *const ARAFactory
    );
    let probe = processor.lifetime_probe();
    let (_controller_storage, controller) = controller_fixture();
    // SAFETY: controller storage remains live through this test.
    let bound = unsafe {
        processor
            .bind(controller, CompanionRoles::all(), CompanionRoles::all())
            .unwrap()
    };
    processor.observe(LifecycleEvent::StateLoad).unwrap();
    processor.observe(LifecycleEvent::Activate).unwrap();
    processor.observe(LifecycleEvent::Process).unwrap();
    processor.observe(LifecycleEvent::BeginRendering).unwrap();
    assert!(processor.observe(LifecycleEvent::ModelMutation).is_err());
    processor.observe(LifecycleEvent::EndRendering).unwrap();
    processor.observe(LifecycleEvent::ModelMutation).unwrap();
    drop(bound);
    assert!(probe.processor_alive());
    assert!(!probe.controller_alive());
    drop(processor);
    assert!(!probe.storage_is_alive());

    let processor = fixture(&factory);
    let probe = processor.lifetime_probe();
    let (_other_storage, controller) = controller_fixture();
    // SAFETY: controller storage outlives the returned binding.
    let bound = unsafe {
        processor
            .bind(controller, CompanionRoles::all(), CompanionRoles::all())
            .unwrap()
    };
    drop(processor);
    assert!(!probe.processor_alive());
    assert!(probe.controller_alive());
    assert!(probe.storage_is_alive());
    drop(bound);
    assert!(!probe.storage_is_alive());
}
