use ara2_bridge_core::{
    ApiGeneration, AraBool, AraError, AudioModificationProperties, AudioSourceProperties,
    ContentGrade, ContentTimeRange, ContentUpdateScopes, DocumentProperties, FilterSelection,
    MusicalContextProperties, NoteEvent, Notes, PlaybackRegionProperties, RegionSequenceProperties,
    RestoreFilter, StoreFilter,
};
use ara2_bridge_plugin::{
    AudioModifications, AudioSources, ContentObject, ContentProvider, ContentReaderSnapshot,
    ContentSnapshot, CreateContext, DocumentLifecycle, FactoryBuilder, HostContentScope,
    MusicalContexts, PartialPersistence, Persistence, PlaybackRegions, PluginBuilder,
    RegionSequences, UpdateEmitter, UpdateOrigin,
};
use ara2_bridge_sys::*;
use std::ffi::c_void;
use std::mem::offset_of;
use std::ptr::null;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn factory_creates_and_drives_an_owned_document_controller() {
    SOURCE_NOTIFICATIONS.store(0, Ordering::SeqCst);
    HOST_AUDIO_READERS_CREATED.store(0, Ordering::SeqCst);
    HOST_AUDIO_READERS_DESTROYED.store(0, Ordering::SeqCst);
    HOST_CONTENT_READERS_CREATED.store(0, Ordering::SeqCst);
    HOST_CONTENT_READERS_DESTROYED.store(0, Ordering::SeqCst);
    WRITTEN_BYTES.store(0, Ordering::SeqCst);
    RESTORES.store(0, Ordering::SeqCst);
    let documents = Arc::new(AtomicUsize::new(0));
    let destroyed_documents = Arc::new(AtomicUsize::new(0));
    let factory = FactoryBuilder::new("controller.test", "archive.controller.test")
        .display("Controller Test", "Example", "https://example.test", "1.0")
        .document_controller({
            let documents = documents.clone();
            let destroyed_documents = destroyed_documents.clone();
            move || {
                let mut builder = PluginBuilder::new(Model {
                    documents: documents.clone(),
                    destroyed_documents: destroyed_documents.clone(),
                    updates: None,
                    exercise_host_access: true,
                });
                let updates = builder.update_emitter();
                builder.model_mut().updates = Some(updates);
                builder
                    .content(TestContent)
                    .partial_persistence(TestPersistence)
                    .build()
            }
        })
        .build()
        .unwrap();

    let mut assertion: ARAAssertFunction = None;
    factory
        .entry()
        .initialize(ApiGeneration::V23Final, &raw mut assertion)
        .unwrap();
    let host = HostFixture::new();
    let host_instance = host.instance();
    let document = DocumentProperties::new(Some("Document")).unwrap();
    let raw_document = document.as_ffi();
    let raw_factory = factory.raw_copy();
    // SAFETY: copied from the complete live factory record.
    let create = unsafe {
        std::ptr::addr_of!(raw_factory.createDocumentControllerWithDocument).read_unaligned()
    }
    .unwrap();
    // SAFETY: the fixture and properties outlive the returned controller and satisfy ARA input
    // contracts; destruction occurs before either is dropped.
    let instance = unsafe { create(&raw const host_instance, raw_document.as_ref().as_ptr()) };
    assert!(!instance.is_null());
    assert_eq!(documents.load(Ordering::SeqCst), 1);

    // SAFETY: the returned instance is a complete live controller allocation.
    let controller_ref = unsafe {
        ara2_bridge_sys::access::read_field::<ARADocumentControllerRef>(
            instance.cast(),
            offset_of!(ARADocumentControllerInstance, documentControllerRef),
        )
    };
    // SAFETY: same complete live instance contract.
    let interface = unsafe {
        ara2_bridge_sys::access::read_field::<*const ARADocumentControllerInterface>(
            instance.cast(),
            offset_of!(ARADocumentControllerInstance, documentControllerInterface),
        )
    };
    assert!(!controller_ref.is_null());
    assert!(!interface.is_null());

    let begin: unsafe extern "C" fn(ARADocumentControllerRef) = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, beginEditing),
    );
    let end: unsafe extern "C" fn(ARADocumentControllerRef) = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, endEditing),
    );
    let create_source: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAAudioSourceHostRef,
        *const ARAAudioSourceProperties,
    ) -> ARAAudioSourceRef = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, createAudioSource),
    );
    let destroy_source: unsafe extern "C" fn(ARADocumentControllerRef, ARAAudioSourceRef) =
        callback(
            interface,
            offset_of!(ARADocumentControllerInterface, destroyAudioSource),
        );
    let enable_source: unsafe extern "C" fn(ARADocumentControllerRef, ARAAudioSourceRef, ARABool) =
        callback(
            interface,
            offset_of!(
                ARADocumentControllerInterface,
                enableAudioSourceSamplesAccess
            ),
        );
    let get_factory: unsafe extern "C" fn(ARADocumentControllerRef) -> *const ARAFactory = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, getFactory),
    );
    let destroy: unsafe extern "C" fn(ARADocumentControllerRef) = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, destroyDocumentController),
    );
    let notify_updates: unsafe extern "C" fn(ARADocumentControllerRef) = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, notifyModelUpdates),
    );
    let content_available: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAAudioSourceRef,
        ARAContentType,
    ) -> ARABool = callback(
        interface,
        offset_of!(
            ARADocumentControllerInterface,
            isAudioSourceContentAvailable
        ),
    );
    let content_grade: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAAudioSourceRef,
        ARAContentType,
    ) -> ARAContentGrade = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, getAudioSourceContentGrade),
    );
    let create_reader: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAAudioSourceRef,
        ARAContentType,
        *const ARAContentTimeRange,
    ) -> ARAContentReaderRef = callback(
        interface,
        offset_of!(
            ARADocumentControllerInterface,
            createAudioSourceContentReader
        ),
    );
    let reader_count: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAContentReaderRef,
    ) -> ARAInt32 = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, getContentReaderEventCount),
    );
    let reader_data: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAContentReaderRef,
        ARAInt32,
    ) -> *const c_void = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, getContentReaderDataForEvent),
    );
    let destroy_reader: unsafe extern "C" fn(ARADocumentControllerRef, ARAContentReaderRef) =
        callback(
            interface,
            offset_of!(ARADocumentControllerInterface, destroyContentReader),
        );
    let restore_objects: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAArchiveReaderHostRef,
        *const ARARestoreObjectsFilter,
    ) -> ARABool = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, restoreObjectsFromArchive),
    );
    let store_objects: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAArchiveWriterHostRef,
        *const ARAStoreObjectsFilter,
    ) -> ARABool = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, storeObjectsToArchive),
    );

    let source = AudioSourceProperties::new(
        Some("Source"),
        "source-1",
        48_000,
        48_000.0,
        2,
        AraBool::new(false),
    )
    .unwrap();
    let raw_source = source.as_ffi(ApiGeneration::V23Final).unwrap();
    // SAFETY: all callback inputs are live and the ARA edit sequence is balanced.
    unsafe { begin(controller_ref) };
    // SAFETY: same callback contract; the host identity remains live through destruction.
    let source_ref = unsafe {
        create_source(
            controller_ref,
            host.source_ref(),
            raw_source.as_ref().as_ptr(),
        )
    };
    assert!(!source_ref.is_null());
    // SAFETY: null selects the complete archive while the edit/restoration session is active.
    assert_eq!(
        unsafe { restore_objects(controller_ref, stable_ref(), null()) },
        kARATrue
    );
    assert_eq!(RESTORES.load(Ordering::SeqCst), 1);
    // SAFETY: source identity and requested standard content type are valid.
    assert_eq!(
        unsafe {
            content_available(
                controller_ref,
                source_ref,
                kARAContentTypeNotes as ARAContentType,
            )
        },
        kARATrue
    );
    // SAFETY: same live source/content query contract.
    assert_eq!(
        unsafe {
            content_grade(
                controller_ref,
                source_ref,
                kARAContentTypeNotes as ARAContentType,
            )
        },
        kARAContentGradeApproved as i32
    );
    // SAFETY: null range requests the entire live source.
    let reader = unsafe {
        create_reader(
            controller_ref,
            source_ref,
            kARAContentTypeNotes as ARAContentType,
            null(),
        )
    };
    assert!(!reader.is_null());
    // SAFETY: the reader is live and owned by this controller.
    assert_eq!(unsafe { reader_count(controller_ref, reader) }, 1);
    // SAFETY: index zero lies in the one-event reader.
    let note = unsafe { reader_data(controller_ref, reader, 0) };
    assert!(!note.is_null());
    // SAFETY: the provider published a validated complete note event at this pointer.
    let note = unsafe { note.cast::<ARAContentNote>().read_unaligned() };
    // SAFETY: copy the packed scalar without creating a potentially unaligned reference.
    let pitch = unsafe { std::ptr::addr_of!(note.pitchNumber).read_unaligned() };
    assert_eq!(pitch, 69);
    // SAFETY: unique reader destruction before controller teardown.
    unsafe { destroy_reader(controller_ref, reader) };
    // SAFETY: balances the creation/restoration edit before host notification delivery.
    unsafe { end(controller_ref) };
    // SAFETY: sample access toggles are model-thread calls allowed outside an edit session.
    unsafe {
        enable_source(controller_ref, source_ref, kARATrue);
        enable_source(controller_ref, source_ref, kARAFalse);
    }
    assert_eq!(HOST_AUDIO_READERS_CREATED.load(Ordering::SeqCst), 1);
    assert_eq!(HOST_AUDIO_READERS_DESTROYED.load(Ordering::SeqCst), 1);
    assert_eq!(HOST_CONTENT_READERS_CREATED.load(Ordering::SeqCst), 1);
    assert_eq!(HOST_CONTENT_READERS_DESTROYED.load(Ordering::SeqCst), 1);
    // SAFETY: delivers the recovery-origin source update queued during model creation.
    unsafe { notify_updates(controller_ref) };
    assert_eq!(SOURCE_NOTIFICATIONS.load(Ordering::SeqCst), 1);
    // SAFETY: graph destruction requires a fresh balanced edit session.
    unsafe { begin(controller_ref) };
    // SAFETY: `source_ref` is the live leaf created above.
    unsafe { destroy_source(controller_ref, source_ref) };
    // SAFETY: balances `begin`.
    unsafe { end(controller_ref) };
    // SAFETY: null selects the complete graph and editing is inactive during storage.
    assert_eq!(
        unsafe { store_objects(controller_ref, stable_ref(), null()) },
        kARATrue
    );
    assert_eq!(WRITTEN_BYTES.load(Ordering::SeqCst), 3);
    // SAFETY: pure query on the live controller.
    assert_eq!(unsafe { get_factory(controller_ref) }, factory.as_raw());
    // SAFETY: unique terminal controller call after all model children were destroyed.
    unsafe { destroy(controller_ref) };
    assert_eq!(destroyed_documents.load(Ordering::SeqCst), 1);
    factory.entry().uninitialize().unwrap();
}

#[test]
#[cfg(not(target_arch = "aarch64"))]
fn ara1_playback_regions_use_internal_synthetic_sequences() {
    let factory = FactoryBuilder::new("controller.ara1", "archive.controller.ara1")
        .display("ARA1 Test", "Example", "https://example.test", "1.0")
        .generations(ApiGeneration::V1Final, ApiGeneration::V1Final)
        .document_controller(|| {
            let mut builder = PluginBuilder::new(Model {
                documents: Arc::new(AtomicUsize::new(0)),
                destroyed_documents: Arc::new(AtomicUsize::new(0)),
                updates: None,
                exercise_host_access: false,
            });
            let updates = builder.update_emitter();
            builder.model_mut().updates = Some(updates);
            builder.build()
        })
        .build()
        .unwrap();
    let mut assertion: ARAAssertFunction = None;
    factory
        .entry()
        .initialize(ApiGeneration::V1Final, &raw mut assertion)
        .unwrap();
    let host = HostFixture::new();
    let host_instance = host.instance();
    let document = DocumentProperties::new(Some("Legacy")).unwrap();
    let raw_document = document.as_ffi();
    let raw_factory = factory.raw_copy();
    // SAFETY: copied from the complete live factory.
    let create = unsafe {
        std::ptr::addr_of!(raw_factory.createDocumentControllerWithDocument).read_unaligned()
    }
    .unwrap();
    // SAFETY: host and document inputs outlive the controller.
    let instance = unsafe { create(&raw const host_instance, raw_document.as_ref().as_ptr()) };
    assert!(!instance.is_null());
    // SAFETY: complete live instance fields.
    let controller_ref = unsafe {
        ara2_bridge_sys::access::read_field::<ARADocumentControllerRef>(
            instance.cast(),
            offset_of!(ARADocumentControllerInstance, documentControllerRef),
        )
    };
    // SAFETY: same instance contract.
    let interface = unsafe {
        ara2_bridge_sys::access::read_field::<*const ARADocumentControllerInterface>(
            instance.cast(),
            offset_of!(ARADocumentControllerInstance, documentControllerInterface),
        )
    };
    let begin: unsafe extern "C" fn(ARADocumentControllerRef) = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, beginEditing),
    );
    let end: unsafe extern "C" fn(ARADocumentControllerRef) = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, endEditing),
    );
    let create_context: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAMusicalContextHostRef,
        *const ARAMusicalContextProperties,
    ) -> ARAMusicalContextRef = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, createMusicalContext),
    );
    let create_source: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAAudioSourceHostRef,
        *const ARAAudioSourceProperties,
    ) -> ARAAudioSourceRef = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, createAudioSource),
    );
    let create_modification: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAAudioSourceRef,
        ARAAudioModificationHostRef,
        *const ARAAudioModificationProperties,
    ) -> ARAAudioModificationRef = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, createAudioModification),
    );
    let create_region: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAAudioModificationRef,
        ARAPlaybackRegionHostRef,
        *const ARAPlaybackRegionProperties,
    ) -> ARAPlaybackRegionRef = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, createPlaybackRegion),
    );
    let destroy_region: unsafe extern "C" fn(ARADocumentControllerRef, ARAPlaybackRegionRef) =
        callback(
            interface,
            offset_of!(ARADocumentControllerInterface, destroyPlaybackRegion),
        );
    let destroy_modification: unsafe extern "C" fn(
        ARADocumentControllerRef,
        ARAAudioModificationRef,
    ) = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, destroyAudioModification),
    );
    let destroy_source: unsafe extern "C" fn(ARADocumentControllerRef, ARAAudioSourceRef) =
        callback(
            interface,
            offset_of!(ARADocumentControllerInterface, destroyAudioSource),
        );
    let destroy_context: unsafe extern "C" fn(ARADocumentControllerRef, ARAMusicalContextRef) =
        callback(
            interface,
            offset_of!(ARADocumentControllerInterface, destroyMusicalContext),
        );
    let destroy: unsafe extern "C" fn(ARADocumentControllerRef) = callback(
        interface,
        offset_of!(ARADocumentControllerInterface, destroyDocumentController),
    );

    let context_properties = MusicalContextProperties::new(Some("Context"), 0, None).unwrap();
    let raw_context = context_properties.as_ffi(ApiGeneration::V1Final).unwrap();
    let source_properties = AudioSourceProperties::new(
        Some("Source"),
        "legacy-source",
        48_000,
        48_000.0,
        2,
        AraBool::new(false),
    )
    .unwrap();
    let raw_source = source_properties.as_ffi(ApiGeneration::V1Final).unwrap();
    let modification_properties =
        AudioModificationProperties::new(Some("Modification"), "legacy-modification").unwrap();
    let raw_modification = modification_properties.as_ffi();
    // SAFETY: balanced edit and every host/property input remains live through the callbacks.
    unsafe { begin(controller_ref) };
    let context =
        unsafe { create_context(controller_ref, stable_ref(), raw_context.as_ref().as_ptr()) };
    let source = unsafe {
        create_source(
            controller_ref,
            host.source_ref(),
            raw_source.as_ref().as_ptr(),
        )
    };
    let modification = unsafe {
        create_modification(
            controller_ref,
            source,
            stable_ref(),
            raw_modification.as_ref().as_ptr(),
        )
    };
    assert!(!context.is_null() && !source.is_null() && !modification.is_null());
    let playback = ARAPlaybackRegionProperties {
        structSize: ara2_bridge_sys::layout::ARAPLAYBACK_REGION_PROPERTIES_MUSICAL_CONTEXT_REF,
        transformationFlags: 0,
        startInModificationTime: 0.0,
        durationInModificationTime: 1.0,
        startInPlaybackTime: 0.0,
        durationInPlaybackTime: 1.0,
        musicalContextRef: context,
        regionSequenceRef: std::ptr::null_mut(),
        name: null(),
        color: null(),
    };
    let region = unsafe {
        create_region(
            controller_ref,
            modification,
            stable_ref(),
            &raw const playback,
        )
    };
    assert!(!region.is_null());
    // SAFETY: destroy the normalized graph leaf-first, then balance and terminate the controller.
    unsafe {
        destroy_region(controller_ref, region);
        destroy_modification(controller_ref, modification);
        destroy_source(controller_ref, source);
        destroy_context(controller_ref, context);
        end(controller_ref);
        destroy(controller_ref);
    }
    factory.entry().uninitialize().unwrap();
}

fn callback<T: Copy>(interface: *const ARADocumentControllerInterface, offset: usize) -> T {
    // SAFETY: every requested field lies in the generation-2.3 interface prefix and is non-null.
    unsafe {
        ara2_bridge_sys::access::read_field::<Option<T>>(interface.cast(), offset)
            .expect("represented callback")
    }
}

struct HostFixture {
    _audio: Box<ARAAudioAccessControllerInterface>,
    _archive: Box<ARAArchivingControllerInterface>,
    _model: Box<ARAModelUpdateControllerInterface>,
    _content: Box<ARAContentAccessControllerInterface>,
    source_identity: Box<u8>,
}

impl HostFixture {
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
        let model = Box::new(ARAModelUpdateControllerInterface {
            structSize: std::mem::size_of::<ARAModelUpdateControllerInterface>(),
            notifyAudioSourceAnalysisProgress: None,
            notifyAudioSourceContentChanged: Some(source_changed),
            notifyAudioModificationContentChanged: None,
            notifyPlaybackRegionContentChanged: None,
            notifyDocumentDataChanged: None,
        });
        let content = Box::new(ARAContentAccessControllerInterface {
            structSize: std::mem::size_of::<ARAContentAccessControllerInterface>(),
            isMusicalContextContentAvailable: Some(host_musical_content_available),
            getMusicalContextContentGrade: Some(host_musical_content_grade),
            createMusicalContextContentReader: Some(host_create_musical_reader),
            isAudioSourceContentAvailable: Some(host_source_content_available),
            getAudioSourceContentGrade: Some(host_source_content_grade),
            createAudioSourceContentReader: Some(host_create_source_reader),
            getContentReaderEventCount: Some(host_content_count),
            getContentReaderDataForEvent: Some(host_content_data),
            destroyContentReader: Some(host_destroy_content_reader),
        });
        Self {
            _audio: audio,
            _archive: archive,
            _model: model,
            _content: content,
            source_identity: Box::new(0),
        }
    }

    fn instance(&self) -> ARADocumentControllerHostInstance {
        ARADocumentControllerHostInstance {
            structSize: std::mem::size_of::<ARADocumentControllerHostInstance>(),
            audioAccessControllerHostRef: stable_ref(),
            audioAccessControllerInterface: self._audio.as_ref(),
            archivingControllerHostRef: stable_ref(),
            archivingControllerInterface: self._archive.as_ref(),
            contentAccessControllerHostRef: stable_ref(),
            contentAccessControllerInterface: self._content.as_ref(),
            modelUpdateControllerHostRef: stable_ref(),
            modelUpdateControllerInterface: self._model.as_ref(),
            playbackControllerHostRef: null_mut_ref(),
            playbackControllerInterface: null(),
        }
    }

    fn source_ref(&self) -> ARAAudioSourceHostRef {
        std::ptr::from_ref(self.source_identity.as_ref())
            .cast_mut()
            .cast()
    }
}

fn null_mut_ref<T>() -> *mut T {
    std::ptr::null_mut()
}

fn stable_ref<T>() -> *mut T {
    std::ptr::from_ref(&DOCUMENT_SENTINEL).cast_mut().cast()
}

static DOCUMENT_SENTINEL: AtomicUsize = AtomicUsize::new(0);
static SOURCE_NOTIFICATIONS: AtomicUsize = AtomicUsize::new(0);
static WRITTEN_BYTES: AtomicUsize = AtomicUsize::new(0);
static RESTORES: AtomicUsize = AtomicUsize::new(0);
static HOST_AUDIO_READERS_CREATED: AtomicUsize = AtomicUsize::new(0);
static HOST_AUDIO_READERS_DESTROYED: AtomicUsize = AtomicUsize::new(0);
static HOST_CONTENT_READERS_CREATED: AtomicUsize = AtomicUsize::new(0);
static HOST_CONTENT_READERS_DESTROYED: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn create_audio_reader(
    _: ARAAudioAccessControllerHostRef,
    _: ARAAudioSourceHostRef,
    _: ARABool,
) -> ARAAudioReaderHostRef {
    HOST_AUDIO_READERS_CREATED.fetch_add(1, Ordering::SeqCst);
    stable_ref()
}
unsafe extern "C" fn read_audio_samples(
    _: ARAAudioAccessControllerHostRef,
    _: ARAAudioReaderHostRef,
    _: ARASamplePosition,
    _: ARASampleCount,
    _: *const *mut c_void,
) -> ARABool {
    kARATrue
}
unsafe extern "C" fn destroy_audio_reader(
    _: ARAAudioAccessControllerHostRef,
    _: ARAAudioReaderHostRef,
) {
    HOST_AUDIO_READERS_DESTROYED.fetch_add(1, Ordering::SeqCst);
}

unsafe extern "C" fn host_musical_content_available(
    _: ARAContentAccessControllerHostRef,
    _: ARAMusicalContextHostRef,
    _: ARAContentType,
) -> ARABool {
    kARAFalse
}

unsafe extern "C" fn host_musical_content_grade(
    _: ARAContentAccessControllerHostRef,
    _: ARAMusicalContextHostRef,
    _: ARAContentType,
) -> ARAContentGrade {
    kARAContentGradeInitial as ARAContentGrade
}

unsafe extern "C" fn host_create_musical_reader(
    _: ARAContentAccessControllerHostRef,
    _: ARAMusicalContextHostRef,
    _: ARAContentType,
    _: *const ARAContentTimeRange,
) -> ARAContentReaderHostRef {
    null_mut_ref()
}

unsafe extern "C" fn host_source_content_available(
    _: ARAContentAccessControllerHostRef,
    _: ARAAudioSourceHostRef,
    content_type: ARAContentType,
) -> ARABool {
    if content_type == kARAContentTypeNotes as ARAContentType {
        7
    } else {
        kARAFalse
    }
}

unsafe extern "C" fn host_source_content_grade(
    _: ARAContentAccessControllerHostRef,
    _: ARAAudioSourceHostRef,
    _: ARAContentType,
) -> ARAContentGrade {
    kARAContentGradeApproved as ARAContentGrade
}

unsafe extern "C" fn host_create_source_reader(
    _: ARAContentAccessControllerHostRef,
    _: ARAAudioSourceHostRef,
    _: ARAContentType,
    _: *const ARAContentTimeRange,
) -> ARAContentReaderHostRef {
    HOST_CONTENT_READERS_CREATED.fetch_add(1, Ordering::SeqCst);
    stable_ref()
}

unsafe extern "C" fn host_content_count(
    _: ARAContentAccessControllerHostRef,
    _: ARAContentReaderHostRef,
) -> ARAInt32 {
    1
}

static HOST_NOTE: ARAContentNote = ARAContentNote {
    frequency: 440.0,
    pitchNumber: 69,
    volume: 1.0,
    startPosition: 0.0,
    attackDuration: 0.0,
    noteDuration: 1.0,
    signalDuration: 1.0,
};

unsafe extern "C" fn host_content_data(
    _: ARAContentAccessControllerHostRef,
    _: ARAContentReaderHostRef,
    index: ARAInt32,
) -> *const c_void {
    if index == 0 {
        std::ptr::from_ref(&HOST_NOTE).cast()
    } else {
        null()
    }
}

unsafe extern "C" fn host_destroy_content_reader(
    _: ARAContentAccessControllerHostRef,
    _: ARAContentReaderHostRef,
) {
    HOST_CONTENT_READERS_DESTROYED.fetch_add(1, Ordering::SeqCst);
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
    length: ARASize,
    _: *const ARAByte,
) -> ARABool {
    WRITTEN_BYTES.fetch_add(length, Ordering::SeqCst);
    kARATrue
}
unsafe extern "C" fn progress(_: ARAArchivingControllerHostRef, _: f32) {}
unsafe extern "C" fn archive_id(
    _: ARAArchivingControllerHostRef,
    _: ARAArchiveReaderHostRef,
) -> ARAPersistentID {
    c"archive.controller.test".as_ptr()
}
unsafe extern "C" fn source_changed(
    _: ARAModelUpdateControllerHostRef,
    _: ARAAudioSourceHostRef,
    _: *const ARAContentTimeRange,
    _: ARAContentUpdateFlags,
) {
    SOURCE_NOTIFICATIONS.fetch_add(1, Ordering::SeqCst);
}

struct Model {
    documents: Arc<AtomicUsize>,
    destroyed_documents: Arc<AtomicUsize>,
    updates: Option<UpdateEmitter>,
    exercise_host_access: bool,
}

impl DocumentLifecycle for Model {
    type Document = ();

    fn create_document(
        &mut self,
        _: &CreateContext,
        _: DocumentProperties,
    ) -> Result<Self::Document, AraError> {
        self.documents.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn destroy_document(&mut self, _: Self::Document) {
        self.destroyed_documents.fetch_add(1, Ordering::SeqCst);
    }
}

impl MusicalContexts for Model {
    type MusicalContext = ();
    fn create_musical_context(
        &mut self,
        _: &CreateContext,
        _: MusicalContextProperties,
        _: &HostContentScope<'_, '_>,
    ) -> Result<Self::MusicalContext, AraError> {
        Ok(())
    }
}

impl RegionSequences for Model {
    type RegionSequence = ();
    fn create_region_sequence(
        &mut self,
        _: &CreateContext,
        _: RegionSequenceProperties,
    ) -> Result<Self::RegionSequence, AraError> {
        Ok(())
    }
}

impl AudioSources for Model {
    type AudioSource = Option<ara2_bridge_plugin::HostAudioReader<f32>>;
    fn create_audio_source(
        &mut self,
        context: &CreateContext,
        _: AudioSourceProperties,
        host: &HostContentScope<'_, '_>,
    ) -> Result<Self::AudioSource, AraError> {
        self.updates
            .as_ref()
            .expect("fixture emitter installed")
            .mark_source(
                context
                    .object_handle()
                    .expect("source identity provisioned"),
                None,
                ContentUpdateScopes::empty(),
                UpdateOrigin::Recovery,
            )?;
        if !self.exercise_host_access {
            return Ok(None);
        }
        let source = host
            .current_audio_source()
            .ok_or(AraError::InvalidState("source callback scope is missing"))?;
        let mut content = host.audio_source::<Notes>(source, None)?;
        if content.event(0)?.pitch_number() != 69 {
            return Err(AraError::Peer("unexpected host note content"));
        }
        Ok(None)
    }

    fn enable_audio_source_samples_access(
        &mut self,
        state: &mut Self::AudioSource,
        enable: bool,
        host: &HostContentScope<'_, '_>,
    ) -> Result<(), AraError> {
        if enable {
            let source = host
                .current_audio_source()
                .ok_or(AraError::InvalidState("source callback scope is missing"))?;
            *state = Some(host.audio_reader::<f32>(source, 2)?);
        } else {
            *state = None;
        }
        Ok(())
    }
}

impl AudioModifications for Model {
    type AudioModification = ();
    fn create_audio_modification(
        &mut self,
        _: &CreateContext,
        _: AudioModificationProperties,
    ) -> Result<Self::AudioModification, AraError> {
        Ok(())
    }
    fn clone_audio_modification(
        &mut self,
        _: &CreateContext,
        _: &Self::AudioModification,
        _: AudioModificationProperties,
    ) -> Result<Self::AudioModification, AraError> {
        Ok(())
    }
}

impl PlaybackRegions for Model {
    type PlaybackRegion = ();
    fn create_playback_region(
        &mut self,
        _: &CreateContext,
        _: PlaybackRegionProperties,
    ) -> Result<Self::PlaybackRegion, AraError> {
        Ok(())
    }
}

struct TestContent;

impl ContentProvider for TestContent {
    fn is_content_available(&self, _: ContentObject, content_type: i32) -> bool {
        content_type == kARAContentTypeNotes as i32
    }

    fn content_grade(&self, _: ContentObject, _: i32) -> ContentGrade {
        ContentGrade::APPROVED
    }

    fn create_content_reader(
        &mut self,
        _: ContentObject,
        content_type: i32,
        _: Option<ContentTimeRange>,
    ) -> Result<Option<ContentReaderSnapshot>, AraError> {
        if content_type != kARAContentTypeNotes as i32 {
            return Ok(None);
        }
        let note = NoteEvent::new(Some(440.0), Some(69), 1.0, 0.0, 0.0, 1.0, 1.0)?;
        Ok(Some(
            ContentSnapshot::<Notes>::new([note])?.into_reader(ContentGrade::APPROVED),
        ))
    }
}

struct TestPersistence;

impl Persistence for TestPersistence {
    fn restore_document(&mut self, _: &[u8]) -> Result<(), AraError> {
        RESTORES.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn store_document(&mut self) -> Result<Vec<u8>, AraError> {
        Ok(vec![1, 2, 3])
    }
}

impl PartialPersistence for TestPersistence {
    fn restore_objects(
        &mut self,
        _: &FilterSelection<RestoreFilter>,
        _: &[u8],
    ) -> Result<(), AraError> {
        RESTORES.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn store_objects(&mut self, _: &FilterSelection<StoreFilter>) -> Result<Vec<u8>, AraError> {
        Ok(vec![1, 2, 3])
    }
}
