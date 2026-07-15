use ara2_bridge_core::{
    AraError, ArchiveError, ArchiveProgress, AudioModificationKind, AudioSourceKind,
    FilterSelection, MemoryArchive, ReadAt, Registry, RegistrySession, RestoreFilter, RestorePhase,
    StoreFilter, WriteAt,
};
use proptest::prelude::*;

#[test]
fn archive_io_is_position_based_and_checked() {
    let archive = MemoryArchive::from(vec![1, 2, 3, 4]);
    let mut out = [0_u8; 2];
    archive.read_at(1, &mut out).unwrap();
    assert_eq!(out, [2, 3]);
    assert!(matches!(
        archive.read_at(u64::MAX, &mut out),
        Err(AraError::Archive(ArchiveError::RangeOverflow))
    ));

    let mut sparse = MemoryArchive::default();
    sparse.write_at(2, &[7, 8]).unwrap();
    assert_eq!(sparse.as_slice(), &[0, 0, 7, 8]);
}

#[test]
fn restore_filter_rejects_duplicate_archive_ids() {
    assert!(RestoreFilter::builder()
        .audio_source("archive-a", "current-x")
        .audio_source("archive-a", "current-y")
        .build()
        .is_err());
}

#[test]
fn archive_ids_bounds_and_progress_are_explicit() {
    let archive = MemoryArchive::with_id(vec![1, 2, 3], "document-1").unwrap();
    assert_eq!(archive.archive_id(), Some("document-1"));
    assert!(matches!(
        archive.read_at(2, &mut [0_u8; 2]),
        Err(AraError::Archive(ArchiveError::OutOfBounds))
    ));
    assert!(MemoryArchive::with_id(Vec::new(), "é").is_err());

    let mut progress = ArchiveProgress::default();
    progress.update(0.0).unwrap();
    progress.update(0.5).unwrap();
    progress.update(1.0).unwrap();
    assert_eq!(progress.current(), Some(1.0));
    assert!(progress.update(0.9).is_err());
    progress.reset();
    assert!(progress.update(f32::NAN).is_err());
}

#[test]
fn restore_filter_validates_ids_and_orders_document_data_last() {
    let filter = RestoreFilter::builder()
        .document_data(true)
        .audio_source("old-source", "new-source")
        .audio_modification("old-mod", "new-mod")
        .build()
        .unwrap();
    assert_eq!(filter.audio_sources()[0].archive_id(), "old-source");
    assert_eq!(filter.audio_sources()[0].current_id(), "new-source");
    assert_eq!(
        filter.phases(),
        vec![
            RestorePhase::AudioSources,
            RestorePhase::AudioModifications,
            RestorePhase::DocumentData,
        ]
    );
    assert!(RestoreFilter::builder()
        .audio_source("one", "same")
        .audio_source("two", "same")
        .build()
        .is_err());
    assert!(RestoreFilter::builder()
        .audio_modification("", "current")
        .build()
        .is_err());
}

#[test]
fn store_filter_rejects_foreign_and_duplicate_handles() {
    let session = RegistrySession::new();
    let mut sources = Registry::<AudioSourceKind, _>::in_session(session, 4);
    let mut modifications = Registry::<AudioModificationKind, _>::in_session(session, 4);
    let source = sources.insert("source").unwrap();
    let modification = modifications.insert("modification").unwrap();
    let filter = StoreFilter::builder(session)
        .document_data(true)
        .audio_source(source)
        .audio_modification(modification)
        .build()
        .unwrap();
    assert_eq!(filter.session(), session);
    assert!(filter.includes_document_data());

    assert!(StoreFilter::builder(session)
        .audio_source(source)
        .audio_source(source)
        .build()
        .is_err());
    let mut foreign = Registry::<AudioSourceKind, _>::new(1);
    let foreign = foreign.insert("foreign").unwrap();
    assert!(StoreFilter::builder(session)
        .audio_source(foreign)
        .build()
        .is_err());

    let all: FilterSelection<StoreFilter> = None.into();
    assert!(all.as_selected().is_none());
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn sparse_writes_match_a_checked_vec_model(
        position in 0usize..128,
        data in proptest::collection::vec(any::<u8>(), 0..32),
    ) {
        let mut archive = MemoryArchive::default();
        archive.write_at(position as u64, &data).unwrap();
        let mut expected = vec![0_u8; position + data.len()];
        expected[position..].copy_from_slice(&data);
        prop_assert_eq!(archive.as_slice(), expected);
    }
}

#[cfg(target_pointer_width = "32")]
#[test]
fn archive_larger_than_address_space_is_rejected() {
    let archive = MemoryArchive::default();
    let position = u64::from(u32::MAX) + 1;
    assert_eq!(
        archive.read_at(position, &mut []),
        Err(AraError::ArchiveTooLargeForTarget)
    );
}
