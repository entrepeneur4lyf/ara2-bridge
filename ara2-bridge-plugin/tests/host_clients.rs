use ara2_bridge_core::{ApiGeneration, Notes};
use ara2_bridge_plugin::{HostAudioSourceRef, HostClients};
use ara2_bridge_sys::*;
use std::ffi::c_void;
use std::ptr::null;
use std::sync::atomic::{AtomicUsize, Ordering};

static DESTROYED_CONTENT_READERS: AtomicUsize = AtomicUsize::new(0);
static DESTROYED_AUDIO_READERS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn required_host_services_and_ara2_archive_id_are_validated() {
    let fixture = Fixture::new();
    let mut host = fixture.host();
    host.audioAccessControllerInterface = null();
    // SAFETY: all advertised fixture records remain live for the attempted construction.
    assert!(unsafe { HostClients::from_raw(&raw const host, ApiGeneration::V23Final) }.is_err());

    let mut archive = fixture.archive;
    archive.structSize = kARAArchivingControllerInterfaceMinSize as usize;
    let host = host_instance(&fixture.audio, &archive, null(), null(), null());
    // SAFETY: all advertised fixture records remain live for the attempted construction.
    assert!(unsafe { HostClients::from_raw(&raw const host, ApiGeneration::V2Final) }.is_err());
    // SAFETY: ARA 1 does not require the 2.0 archive-ID tail callback.
    assert!(unsafe { HostClients::from_raw(&raw const host, ApiGeneration::V1Final) }.is_ok());
}

#[test]
fn content_readers_require_the_current_object_scope_and_are_destroyed() {
    DESTROYED_CONTENT_READERS.store(0, Ordering::SeqCst);
    let fixture = Fixture::new();
    let host = fixture.host();
    // SAFETY: the fixture and every interface it references outlive `clients`.
    let clients =
        unsafe { HostClients::from_raw(&raw const host, ApiGeneration::V23Final).unwrap() };
    assert!(clients.content().is_none());

    // SAFETY: these opaque identities are kept live by the fixture callbacks for this test.
    let current = unsafe { HostAudioSourceRef::from_raw(fixture.source_ref(0)).unwrap() };
    // SAFETY: same fixture-owned opaque identity contract.
    let other = unsafe { HostAudioSourceRef::from_raw(fixture.source_ref(1)).unwrap() };
    clients.with_audio_source_content(current, |scope| {
        assert!(scope.audio_source::<Notes>(other, None).is_err());
        let reader = scope.audio_source::<Notes>(current, None).unwrap();
        assert_eq!(reader.len(), 0);
    });
    assert_eq!(DESTROYED_CONTENT_READERS.load(Ordering::SeqCst), 1);
}

#[test]
fn truncated_optional_model_update_tail_is_accepted() {
    let fixture = Fixture::new();
    let mut model = fixture.model;
    model.structSize = kARAModelUpdateControllerInterfaceMinSize as usize;
    let host = host_instance(
        &fixture.audio,
        &fixture.archive,
        &*fixture.content,
        &*model,
        null(),
    );
    // SAFETY: all represented fields and callback records remain live for `clients`.
    let clients =
        unsafe { HostClients::from_raw(&raw const host, ApiGeneration::V23Final).unwrap() };
    assert!(clients.model_updates().is_some());
    assert!(!clients
        .model_updates()
        .unwrap()
        .supports_document_data_changed());
}

#[test]
fn audio_reader_escapes_the_creation_scope_but_stays_controller_bound() {
    DESTROYED_AUDIO_READERS.store(0, Ordering::SeqCst);
    let fixture = Fixture::new();
    let host = fixture.host();
    // SAFETY: the fixture and every interface it references outlive `clients` and the reader.
    let clients =
        unsafe { HostClients::from_raw(&raw const host, ApiGeneration::V23Final).unwrap() };
    // SAFETY: the fixture owns this opaque identity through the test.
    let current = unsafe { HostAudioSourceRef::from_raw(fixture.source_ref(0)).unwrap() };
    let mut reader = clients.with_audio_source_content(current, |scope| {
        scope.audio_reader::<f32>(current, 2).unwrap()
    });
    let mut left = [0.0_f32; 4];
    let mut right = [0.0_f32; 4];
    reader.read(0, &mut [&mut left, &mut right]).unwrap();
    assert!(reader.read(-1, &mut [&mut left, &mut right]).is_err());
    assert_eq!(DESTROYED_AUDIO_READERS.load(Ordering::SeqCst), 0);
    drop(clients);
    assert_eq!(DESTROYED_AUDIO_READERS.load(Ordering::SeqCst), 1);
    assert!(reader.read(0, &mut [&mut left, &mut right]).is_err());
    drop(reader);
    assert_eq!(DESTROYED_AUDIO_READERS.load(Ordering::SeqCst), 1);
}

struct Fixture {
    audio: Box<ARAAudioAccessControllerInterface>,
    archive: Box<ARAArchivingControllerInterface>,
    content: Box<ARAContentAccessControllerInterface>,
    model: Box<ARAModelUpdateControllerInterface>,
    source_identities: [Box<u8>; 2],
}

impl Fixture {
    fn new() -> Self {
        let audio = Box::new(ARAAudioAccessControllerInterface {
            structSize: std::mem::size_of::<ARAAudioAccessControllerInterface>(),
            createAudioReaderForSource: Some(create_audio_reader),
            readAudioSamples: Some(read_audio_samples),
            destroyAudioReader: Some(destroy_audio_reader),
        });
        let archive = Box::new(ARAArchivingControllerInterface {
            structSize: std::mem::size_of::<ARAArchivingControllerInterface>(),
            getArchiveSize: Some(get_archive_size),
            readBytesFromArchive: Some(read_archive),
            writeBytesToArchive: Some(write_archive),
            notifyDocumentArchivingProgress: Some(progress),
            notifyDocumentUnarchivingProgress: Some(progress),
            getDocumentArchiveID: Some(archive_id),
        });
        let content = Box::new(ARAContentAccessControllerInterface {
            structSize: std::mem::size_of::<ARAContentAccessControllerInterface>(),
            isMusicalContextContentAvailable: Some(musical_available),
            getMusicalContextContentGrade: Some(musical_grade),
            createMusicalContextContentReader: Some(create_musical_reader),
            isAudioSourceContentAvailable: Some(source_available),
            getAudioSourceContentGrade: Some(source_grade),
            createAudioSourceContentReader: Some(create_source_reader),
            getContentReaderEventCount: Some(content_count),
            getContentReaderDataForEvent: Some(content_data),
            destroyContentReader: Some(destroy_content_reader),
        });
        let model = Box::new(ARAModelUpdateControllerInterface {
            structSize: std::mem::size_of::<ARAModelUpdateControllerInterface>(),
            notifyAudioSourceAnalysisProgress: Some(analysis_progress),
            notifyAudioSourceContentChanged: Some(source_changed),
            notifyAudioModificationContentChanged: Some(modification_changed),
            notifyPlaybackRegionContentChanged: Some(region_changed),
            notifyDocumentDataChanged: Some(document_changed),
        });
        Self {
            audio,
            archive,
            content,
            model,
            source_identities: [Box::new(0), Box::new(0)],
        }
    }

    fn host(&self) -> ARADocumentControllerHostInstance {
        host_instance(
            &self.audio,
            &self.archive,
            &*self.content,
            &*self.model,
            null(),
        )
    }

    fn source_ref(&self, index: usize) -> ARAAudioSourceHostRef {
        std::ptr::from_ref(self.source_identities[index].as_ref())
            .cast_mut()
            .cast()
    }
}

fn host_instance(
    audio: &ARAAudioAccessControllerInterface,
    archive: &ARAArchivingControllerInterface,
    content: *const ARAContentAccessControllerInterface,
    model: *const ARAModelUpdateControllerInterface,
    playback: *const ARAPlaybackControllerInterface,
) -> ARADocumentControllerHostInstance {
    ARADocumentControllerHostInstance {
        structSize: std::mem::size_of::<ARADocumentControllerHostInstance>(),
        audioAccessControllerHostRef: stable_ref(),
        audioAccessControllerInterface: audio,
        archivingControllerHostRef: stable_ref(),
        archivingControllerInterface: archive,
        contentAccessControllerHostRef: stable_ref(),
        contentAccessControllerInterface: content,
        modelUpdateControllerHostRef: stable_ref(),
        modelUpdateControllerInterface: model,
        playbackControllerHostRef: stable_ref(),
        playbackControllerInterface: playback,
    }
}

fn stable_ref<T>() -> *mut T {
    std::ptr::from_ref(&DESTROYED_CONTENT_READERS)
        .cast_mut()
        .cast()
}

unsafe extern "C" fn create_audio_reader(
    _: ARAAudioAccessControllerHostRef,
    _: ARAAudioSourceHostRef,
    _: ARABool,
) -> ARAAudioReaderHostRef {
    stable_ref()
}
unsafe extern "C" fn read_audio_samples(
    _: ARAAudioAccessControllerHostRef,
    _: ARAAudioReaderHostRef,
    sample_position: ARASamplePosition,
    _: ARASampleCount,
    _: *const *mut c_void,
) -> ARABool {
    if sample_position < 0 {
        kARAFalse
    } else {
        kARATrue
    }
}
unsafe extern "C" fn destroy_audio_reader(
    _: ARAAudioAccessControllerHostRef,
    _: ARAAudioReaderHostRef,
) {
    DESTROYED_AUDIO_READERS.fetch_add(1, Ordering::SeqCst);
}
unsafe extern "C" fn get_archive_size(
    _: ARAArchivingControllerHostRef,
    _: ARAArchiveReaderHostRef,
) -> ARASize {
    0
}
unsafe extern "C" fn read_archive(
    _: ARAArchivingControllerHostRef,
    _: ARAArchiveReaderHostRef,
    _: ARASize,
    _: ARASize,
    _: *mut ARAByte,
) -> ARABool {
    kARATrue
}
unsafe extern "C" fn write_archive(
    _: ARAArchivingControllerHostRef,
    _: ARAArchiveWriterHostRef,
    _: ARASize,
    _: ARASize,
    _: *const ARAByte,
) -> ARABool {
    kARATrue
}
unsafe extern "C" fn progress(_: ARAArchivingControllerHostRef, _: f32) {}
unsafe extern "C" fn archive_id(
    _: ARAArchivingControllerHostRef,
    _: ARAArchiveReaderHostRef,
) -> ARAPersistentID {
    c"archive.test".as_ptr()
}
unsafe extern "C" fn musical_available(
    _: ARAContentAccessControllerHostRef,
    _: ARAMusicalContextHostRef,
    _: ARAContentType,
) -> ARABool {
    kARATrue
}
unsafe extern "C" fn musical_grade(
    _: ARAContentAccessControllerHostRef,
    _: ARAMusicalContextHostRef,
    _: ARAContentType,
) -> ARAContentGrade {
    kARAContentGradeDetected as ARAContentGrade
}
unsafe extern "C" fn create_musical_reader(
    _: ARAContentAccessControllerHostRef,
    _: ARAMusicalContextHostRef,
    _: ARAContentType,
    _: *const ARAContentTimeRange,
) -> ARAContentReaderHostRef {
    stable_ref()
}
unsafe extern "C" fn source_available(
    _: ARAContentAccessControllerHostRef,
    _: ARAAudioSourceHostRef,
    _: ARAContentType,
) -> ARABool {
    kARATrue
}
unsafe extern "C" fn source_grade(
    _: ARAContentAccessControllerHostRef,
    _: ARAAudioSourceHostRef,
    _: ARAContentType,
) -> ARAContentGrade {
    kARAContentGradeDetected as ARAContentGrade
}
unsafe extern "C" fn create_source_reader(
    _: ARAContentAccessControllerHostRef,
    _: ARAAudioSourceHostRef,
    _: ARAContentType,
    _: *const ARAContentTimeRange,
) -> ARAContentReaderHostRef {
    stable_ref()
}
unsafe extern "C" fn content_count(
    _: ARAContentAccessControllerHostRef,
    _: ARAContentReaderHostRef,
) -> ARAInt32 {
    0
}
unsafe extern "C" fn content_data(
    _: ARAContentAccessControllerHostRef,
    _: ARAContentReaderHostRef,
    _: ARAInt32,
) -> *const c_void {
    null()
}
unsafe extern "C" fn destroy_content_reader(
    _: ARAContentAccessControllerHostRef,
    _: ARAContentReaderHostRef,
) {
    DESTROYED_CONTENT_READERS.fetch_add(1, Ordering::SeqCst);
}
unsafe extern "C" fn analysis_progress(
    _: ARAModelUpdateControllerHostRef,
    _: ARAAudioSourceHostRef,
    _: ARAAnalysisProgressState,
    _: f32,
) {
}
unsafe extern "C" fn source_changed(
    _: ARAModelUpdateControllerHostRef,
    _: ARAAudioSourceHostRef,
    _: *const ARAContentTimeRange,
    _: ARAContentUpdateFlags,
) {
}
unsafe extern "C" fn modification_changed(
    _: ARAModelUpdateControllerHostRef,
    _: ARAAudioModificationHostRef,
    _: *const ARAContentTimeRange,
    _: ARAContentUpdateFlags,
) {
}
unsafe extern "C" fn region_changed(
    _: ARAModelUpdateControllerHostRef,
    _: ARAPlaybackRegionHostRef,
    _: *const ARAContentTimeRange,
    _: ARAContentUpdateFlags,
) {
}
unsafe extern "C" fn document_changed(_: ARAModelUpdateControllerHostRef) {}
