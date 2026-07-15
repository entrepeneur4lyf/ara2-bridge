use ara2_bridge_core::ApiGeneration;
use ara2_bridge_plugin::{
    document_controller_interface, ControllerCapabilities, PLUGIN_CONTRACT_TESTS, PLUGIN_DELEGATES,
};

#[test]
fn every_document_controller_slot_has_one_delegate_and_contract_class() {
    let expected = ara2_bridge_sys::compatibility::DOCUMENT_CONTROLLER_CALLBACKS;
    assert_eq!(PLUGIN_DELEGATES.len(), 54);
    assert_eq!(PLUGIN_CONTRACT_TESTS.len(), 54);
    for callback in expected {
        assert!(PLUGIN_DELEGATES
            .iter()
            .any(|delegate| delegate.c_name == *callback));
        assert!(PLUGIN_CONTRACT_TESTS
            .iter()
            .any(|contract| contract.c_name == *callback));
    }
}

#[test]
fn generation_prefixes_and_later_capabilities_have_non_null_intervening_slots() {
    for generation in ApiGeneration::ALL
        .into_iter()
        .filter(|generation| generation.supported_on_target())
    {
        let base =
            document_controller_interface(generation, ControllerCapabilities::default()).unwrap();
        assert!(base.represented_callbacks_are_non_null());
    }
    let later = document_controller_interface(
        ApiGeneration::V23Final,
        ControllerCapabilities::default().with_signal_preservation(true),
    )
    .unwrap();
    assert!(later.represented_callbacks_are_non_null());
    assert!(later.represented_callback_count() > 50);
}
