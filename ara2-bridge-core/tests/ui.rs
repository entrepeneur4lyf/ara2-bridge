use ara2_bridge_core::{AudioSourceKind, Handle};
use static_assertions::assert_not_impl_any;

#[test]
fn core_type_system_contracts() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/content_event_escape.rs");
    assert_not_impl_any!(Handle<AudioSourceKind>: Send, Sync);
}
