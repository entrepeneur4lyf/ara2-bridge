use ara2_bridge_core::{AraError, Handle, Registry};

enum AudioSourceKind {}
enum PlaybackRegionKind {}

#[test]
fn registry_rejects_stale_and_wrong_kind_handles() {
    let mut registry = Registry::<AudioSourceKind, String>::new(2);
    let handle = registry.insert(String::from("source")).unwrap();
    let raw = handle.into_raw();

    assert_eq!(registry.remove(handle).unwrap(), "source");
    assert!(matches!(
        registry.get(handle),
        Err(AraError::InvalidArgument("stale handle"))
    ));
    assert!(matches!(
        registry.remove(handle),
        Err(AraError::InvalidArgument("stale handle"))
    ));
    assert!(matches!(
        Handle::<PlaybackRegionKind>::try_from_raw(raw),
        Err(AraError::InvalidArgument("wrong handle kind"))
    ));
}

#[test]
fn cells_are_stable_never_reused_and_capacity_is_bounded() {
    let mut registry = Registry::<AudioSourceKind, u32>::new(2);
    let first = registry.insert(10).unwrap();
    let first_pointer = registry.opaque_pointer(first).unwrap();
    let second = registry.insert(20).unwrap();

    assert_eq!(registry.opaque_pointer(first).unwrap(), first_pointer);
    assert!(matches!(
        registry.insert(30),
        Err(AraError::InvalidState("registry capacity exhausted"))
    ));

    assert_eq!(registry.remove(first).unwrap(), 10);
    assert!(matches!(
        registry.insert(30),
        Err(AraError::InvalidState("registry capacity exhausted"))
    ));
    assert_eq!(registry.get(second).unwrap(), &20);
}

#[test]
fn registries_reject_foreign_sessions_and_pointers() {
    let mut first = Registry::<AudioSourceKind, u32>::new(2);
    let second = Registry::<AudioSourceKind, u32>::new(2);
    let handle = first.insert(10).unwrap();
    let pointer = first.opaque_pointer(handle).unwrap();

    assert!(matches!(
        second.get(handle),
        Err(AraError::InvalidArgument("foreign handle"))
    ));
    assert!(matches!(
        second.handle_from_opaque(pointer.as_ptr()),
        Err(AraError::InvalidArgument("foreign handle pointer"))
    ));
    assert_eq!(first.handle_from_opaque(pointer.as_ptr()).unwrap(), handle);
}
