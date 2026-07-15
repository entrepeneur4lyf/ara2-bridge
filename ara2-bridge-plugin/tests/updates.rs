use ara2_bridge_core::{ContentTimeRange, ContentUpdateScopes, RawHandle, Registry};
use ara2_bridge_plugin::{UpdateNotification, UpdateOrigin, UpdateTracker};

#[test]
fn every_persistent_category_flushes_only_during_notify_model_updates() {
    let [source, modification, region] = ids();
    let mut tracker = UpdateTracker::new();
    tracker
        .mark_source(
            source,
            Some(ContentTimeRange::new(1.0, 2.0).unwrap()),
            ContentUpdateScopes::SIGNAL_REMAINS_UNCHANGED,
            UpdateOrigin::Application,
        )
        .unwrap();
    tracker
        .mark_modification(
            modification,
            None,
            ContentUpdateScopes::empty(),
            UpdateOrigin::Recovery,
        )
        .unwrap();
    tracker
        .mark_region(
            region,
            None,
            ContentUpdateScopes::empty(),
            UpdateOrigin::Application,
        )
        .unwrap();
    tracker.mark_document(UpdateOrigin::Application);
    assert_eq!(tracker.pending_count(), 4);

    let mut delivered = Vec::new();
    tracker.flush_with(|notification, _| delivered.push(notification));
    assert!(matches!(
        delivered[0],
        UpdateNotification::AudioSource { .. }
    ));
    assert!(matches!(
        delivered[1],
        UpdateNotification::AudioModification { .. }
    ));
    assert!(matches!(
        delivered[2],
        UpdateNotification::PlaybackRegion { .. }
    ));
    assert!(matches!(delivered[3], UpdateNotification::Document));
    assert_eq!(tracker.pending_count(), 0);
}

#[test]
fn ranges_and_flags_coalesce_and_reentrant_changes_wait_for_next_flush() {
    let [source, modification, _] = ids();
    let mut tracker = UpdateTracker::new();
    tracker
        .mark_source(
            source,
            Some(ContentTimeRange::new(4.0, 2.0).unwrap()),
            ContentUpdateScopes::SIGNAL_REMAINS_UNCHANGED
                | ContentUpdateScopes::TIMING_REMAINS_UNCHANGED,
            UpdateOrigin::Application,
        )
        .unwrap();
    tracker
        .mark_source(
            source,
            Some(ContentTimeRange::new(2.0, 3.0).unwrap()),
            ContentUpdateScopes::SIGNAL_REMAINS_UNCHANGED,
            UpdateOrigin::Application,
        )
        .unwrap();
    let mut first = Vec::new();
    tracker.flush_with(|notification, pending| {
        first.push(notification);
        pending
            .mark_modification(
                modification,
                None,
                ContentUpdateScopes::empty(),
                UpdateOrigin::Application,
            )
            .unwrap();
    });
    let UpdateNotification::AudioSource { range, flags, .. } = &first[0] else {
        panic!("expected source notification")
    };
    let range = range.as_ref().unwrap();
    assert_eq!((range.start(), range.duration()), (2.0, 4.0));
    assert_eq!(*flags, ContentUpdateScopes::SIGNAL_REMAINS_UNCHANGED);
    assert_eq!(tracker.pending_count(), 1);

    let mut second = Vec::new();
    tracker.flush_with(|notification, _| second.push(notification));
    assert!(matches!(
        second.as_slice(),
        [UpdateNotification::AudioModification { .. }]
    ));
}

#[test]
fn host_and_restore_changes_do_not_echo_but_recovery_changes_do() {
    let [source, _, _] = ids();
    let mut tracker = UpdateTracker::new();
    for origin in [UpdateOrigin::Host, UpdateOrigin::Restore] {
        tracker
            .mark_source(source, None, ContentUpdateScopes::empty(), origin)
            .unwrap();
    }
    assert_eq!(tracker.pending_count(), 0);
    tracker
        .mark_source(
            source,
            None,
            ContentUpdateScopes::empty(),
            UpdateOrigin::Recovery,
        )
        .unwrap();
    assert_eq!(tracker.pending_count(), 1);
}

fn ids() -> [RawHandle; 3] {
    enum Kind {}
    let mut registry = Registry::<Kind, ()>::new(3);
    [
        registry.insert(()).unwrap().into_raw(),
        registry.insert(()).unwrap().into_raw(),
        registry.insert(()).unwrap().into_raw(),
    ]
}
