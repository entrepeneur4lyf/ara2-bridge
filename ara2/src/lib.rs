//! Safe Rust bindings for the Celemony ARA2 SDK.
//!
//! ARA2 uses C structs of function pointers (vtables). This crate wraps
//! the `DocumentController` interface as a Rust trait with a vtable
//! builder that converts trait objects into the C-compatible struct
//! the DAW calls through.
//!
//! Unimplemented vtable entries are null pointers. Per the ARA2 spec,
//! the host reads `structSize` to determine which functions are present
//! and treats null entries as "feature not supported."

use ara2_sys::*;

// ── Document Controller Trait ────────────────────────────────────────

pub trait DocumentController {
    fn destroy(&mut self);
    fn get_factory(&self) -> *const ARAFactory;
    fn begin_editing(&mut self);
    fn end_editing(&mut self);
    fn notify_model_updates(&mut self);
    fn update_document_properties(&mut self, properties: &ARADocumentProperties);
    fn create_audio_source(
        &mut self, host_ref: ARAAudioSourceHostRef, properties: &ARAAudioSourceProperties,
    ) -> ARAAudioSourceRef;
    fn update_audio_source_properties(
        &mut self, source: ARAAudioSourceRef, properties: &ARAAudioSourceProperties,
    );
    fn update_audio_source_content(
        &mut self, source: ARAAudioSourceRef, range: Option<&ARAContentTimeRange>,
        flags: ARAContentUpdateFlags,
    );
    fn enable_audio_source_samples_access(&mut self, source: ARAAudioSourceRef, enable: ARABool);
    fn deactivate_audio_source_for_undo_history(
        &mut self, source: ARAAudioSourceRef, deactivate: ARABool,
    );
    fn destroy_audio_source(&mut self, source: ARAAudioSourceRef);
    fn request_audio_source_content_analysis(
        &mut self, source: ARAAudioSourceRef, count: ARASize, content_types: *const ARAContentType,
    );
    fn is_audio_source_content_available(
        &self, source: ARAAudioSourceRef, content_type: ARAContentType,
    ) -> ARABool;
    fn create_musical_context(
        &mut self, host_ref: ARAMusicalContextHostRef, properties: &ARAMusicalContextProperties,
    ) -> ARAMusicalContextRef;
    fn update_musical_context_properties(
        &mut self, ctx: ARAMusicalContextRef, properties: &ARAMusicalContextProperties,
    );
    fn update_musical_context_content(
        &mut self, ctx: ARAMusicalContextRef, range: Option<&ARAContentTimeRange>,
        flags: ARAContentUpdateFlags,
    );
    fn destroy_musical_context(&mut self, ctx: ARAMusicalContextRef);
    fn create_region_sequence(
        &mut self, host_ref: ARARegionSequenceHostRef, properties: &ARARegionSequenceProperties,
    ) -> ARARegionSequenceRef;
    fn update_region_sequence_properties(
        &mut self, seq: ARARegionSequenceRef, properties: &ARARegionSequenceProperties,
    );
    fn destroy_region_sequence(&mut self, seq: ARARegionSequenceRef);
    fn create_playback_region(
        &mut self, host_ref: ARAPlaybackRegionHostRef,
        audio_modification_ref: ARAAudioModificationRef,
        properties: &ARAPlaybackRegionProperties,
    ) -> ARAPlaybackRegionRef;
    fn update_playback_region_properties(
        &mut self, region: ARAPlaybackRegionRef, properties: &ARAPlaybackRegionProperties,
    );
    fn destroy_playback_region(&mut self, region: ARAPlaybackRegionRef);
    fn store_objects_to_archive(
        &mut self, archive_writer_host_ref: ARAArchiveWriterHostRef,
        filter: *const ARAStoreObjectsFilter,
    ) -> ARABool;
    fn restore_objects_from_archive(
        &mut self, archive_reader_host_ref: ARAArchiveReaderHostRef,
        filter: *const ARARestoreObjectsFilter,
    ) -> ARABool;
}

// ── Vtable + Instance Builder ────────────────────────────────────────

struct ControllerState {
    controller: Box<dyn DocumentController>,
}

pub fn build_document_controller_instance(
    controller: Box<dyn DocumentController>,
) -> *const ARADocumentControllerInstance {
    let state = Box::into_raw(Box::new(ControllerState { controller }));
    let vtable = Box::into_raw(Box::new(build_vtable()));
    let instance = Box::new(ARADocumentControllerInstance {
        structSize: std::mem::size_of::<ARADocumentControllerInstance>() as ARASize,
        documentControllerRef: state as ARADocumentControllerRef,
        documentControllerInterface: vtable,
    });
    Box::into_raw(instance)
}

fn build_vtable() -> ARADocumentControllerInterface {
    let mut v: ARADocumentControllerInterface = unsafe { std::mem::zeroed() };
    v.structSize = std::mem::size_of::<ARADocumentControllerInterface>() as ARASize;
    v.destroyDocumentController = Some(destroy_dc);
    v.getFactory = Some(get_factory_cb);
    v.beginEditing = Some(begin_editing_cb);
    v.endEditing = Some(end_editing_cb);
    v.notifyModelUpdates = Some(notify_model_updates_cb);
    v.updateDocumentProperties = Some(update_doc_props_cb);
    v.createAudioSource = Some(create_audio_source_cb);
    v.updateAudioSourceProperties = Some(update_audio_source_props_cb);
    v.updateAudioSourceContent = Some(update_audio_source_content_cb);
    v.enableAudioSourceSamplesAccess = Some(enable_audio_access_cb);
    v.deactivateAudioSourceForUndoHistory = Some(deactivate_audio_undo_cb);
    v.destroyAudioSource = Some(destroy_audio_source_cb);
    v.requestAudioSourceContentAnalysis = Some(request_audio_analysis_cb);
    v.isAudioSourceContentAvailable = Some(is_audio_content_avail_cb);
    v.createMusicalContext = Some(create_musical_ctx_cb);
    v.updateMusicalContextProperties = Some(update_musical_ctx_props_cb);
    v.updateMusicalContextContent = Some(update_musical_ctx_content_cb);
    v.destroyMusicalContext = Some(destroy_musical_ctx_cb);
    v.createRegionSequence = Some(create_region_seq_cb);
    v.updateRegionSequenceProperties = Some(update_region_seq_props_cb);
    v.destroyRegionSequence = Some(destroy_region_seq_cb);
    v.createPlaybackRegion = Some(create_playback_region_cb);
    v.updatePlaybackRegionProperties = Some(update_playback_region_props_cb);
    v.destroyPlaybackRegion = Some(destroy_playback_region_cb);
    v.storeObjectsToArchive = Some(store_objects_cb);
    v.restoreObjectsFromArchive = Some(restore_objects_cb);
    v
}

unsafe fn state<'a>(r: ARADocumentControllerRef) -> &'a mut ControllerState {
    &mut *(r as *mut ControllerState)
}

unsafe extern "C" fn destroy_dc(r: ARADocumentControllerRef) {
    drop(Box::from_raw(r as *mut ControllerState));
}

unsafe extern "C" fn get_factory_cb(r: ARADocumentControllerRef) -> *const ARAFactory {
    state(r).controller.get_factory()
}

unsafe extern "C" fn begin_editing_cb(r: ARADocumentControllerRef) {
    state(r).controller.begin_editing();
}

unsafe extern "C" fn end_editing_cb(r: ARADocumentControllerRef) {
    state(r).controller.end_editing();
}

unsafe extern "C" fn notify_model_updates_cb(r: ARADocumentControllerRef) {
    state(r).controller.notify_model_updates();
}

unsafe extern "C" fn update_doc_props_cb(
    r: ARADocumentControllerRef, p: *const ARADocumentProperties,
) {
    state(r).controller.update_document_properties(&*p);
}

unsafe extern "C" fn create_audio_source_cb(
    r: ARADocumentControllerRef, h: ARAAudioSourceHostRef, p: *const ARAAudioSourceProperties,
) -> ARAAudioSourceRef {
    state(r).controller.create_audio_source(h, &*p)
}

unsafe extern "C" fn update_audio_source_props_cb(
    r: ARADocumentControllerRef, s: ARAAudioSourceRef, p: *const ARAAudioSourceProperties,
) {
    state(r).controller.update_audio_source_properties(s, &*p);
}

unsafe extern "C" fn update_audio_source_content_cb(
    r: ARADocumentControllerRef, s: ARAAudioSourceRef,
    range: *const ARAContentTimeRange, flags: ARAContentUpdateFlags,
) {
    let opt = if range.is_null() { None } else { Some(&*range) };
    state(r).controller.update_audio_source_content(s, opt, flags);
}

unsafe extern "C" fn enable_audio_access_cb(
    r: ARADocumentControllerRef, s: ARAAudioSourceRef, enable: ARABool,
) {
    state(r).controller.enable_audio_source_samples_access(s, enable);
}

unsafe extern "C" fn deactivate_audio_undo_cb(
    r: ARADocumentControllerRef, s: ARAAudioSourceRef, deactivate: ARABool,
) {
    state(r).controller.deactivate_audio_source_for_undo_history(s, deactivate);
}

unsafe extern "C" fn destroy_audio_source_cb(
    r: ARADocumentControllerRef, s: ARAAudioSourceRef,
) {
    state(r).controller.destroy_audio_source(s);
}

unsafe extern "C" fn request_audio_analysis_cb(
    r: ARADocumentControllerRef, s: ARAAudioSourceRef,
    count: ARASize, types: *const ARAContentType,
) {
    state(r).controller.request_audio_source_content_analysis(s, count, types);
}

unsafe extern "C" fn is_audio_content_avail_cb(
    r: ARADocumentControllerRef, s: ARAAudioSourceRef, ct: ARAContentType,
) -> ARABool {
    state(r).controller.is_audio_source_content_available(s, ct)
}

unsafe extern "C" fn create_musical_ctx_cb(
    r: ARADocumentControllerRef, h: ARAMusicalContextHostRef, p: *const ARAMusicalContextProperties,
) -> ARAMusicalContextRef {
    state(r).controller.create_musical_context(h, &*p)
}

unsafe extern "C" fn update_musical_ctx_props_cb(
    r: ARADocumentControllerRef, c: ARAMusicalContextRef, p: *const ARAMusicalContextProperties,
) {
    state(r).controller.update_musical_context_properties(c, &*p);
}

unsafe extern "C" fn update_musical_ctx_content_cb(
    r: ARADocumentControllerRef, c: ARAMusicalContextRef,
    range: *const ARAContentTimeRange, flags: ARAContentUpdateFlags,
) {
    let opt = if range.is_null() { None } else { Some(&*range) };
    state(r).controller.update_musical_context_content(c, opt, flags);
}

unsafe extern "C" fn destroy_musical_ctx_cb(
    r: ARADocumentControllerRef, c: ARAMusicalContextRef,
) {
    state(r).controller.destroy_musical_context(c);
}

unsafe extern "C" fn create_region_seq_cb(
    r: ARADocumentControllerRef, h: ARARegionSequenceHostRef, p: *const ARARegionSequenceProperties,
) -> ARARegionSequenceRef {
    state(r).controller.create_region_sequence(h, &*p)
}

unsafe extern "C" fn update_region_seq_props_cb(
    r: ARADocumentControllerRef, s: ARARegionSequenceRef, p: *const ARARegionSequenceProperties,
) {
    state(r).controller.update_region_sequence_properties(s, &*p);
}

unsafe extern "C" fn destroy_region_seq_cb(
    r: ARADocumentControllerRef, s: ARARegionSequenceRef,
) {
    state(r).controller.destroy_region_sequence(s);
}

unsafe extern "C" fn create_playback_region_cb(
    r: ARADocumentControllerRef, m: ARAAudioModificationRef,
    h: ARAPlaybackRegionHostRef, p: *const ARAPlaybackRegionProperties,
) -> ARAPlaybackRegionRef {
    state(r).controller.create_playback_region(h, m, &*p)
}

unsafe extern "C" fn update_playback_region_props_cb(
    r: ARADocumentControllerRef, reg: ARAPlaybackRegionRef, p: *const ARAPlaybackRegionProperties,
) {
    state(r).controller.update_playback_region_properties(reg, &*p);
}

unsafe extern "C" fn destroy_playback_region_cb(
    r: ARADocumentControllerRef, reg: ARAPlaybackRegionRef,
) {
    state(r).controller.destroy_playback_region(reg);
}

unsafe extern "C" fn store_objects_cb(
    r: ARADocumentControllerRef, wr: ARAArchiveWriterHostRef, f: *const ARAStoreObjectsFilter,
) -> ARABool {
    state(r).controller.store_objects_to_archive(wr, f)
}

unsafe extern "C" fn restore_objects_cb(
    r: ARADocumentControllerRef, rr: ARAArchiveReaderHostRef, f: *const ARARestoreObjectsFilter,
) -> ARABool {
    state(r).controller.restore_objects_from_archive(rr, f)
}
