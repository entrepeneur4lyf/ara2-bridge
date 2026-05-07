//! Safe Rust bindings for the [Celemony ARA2 SDK](https://github.com/Celemony/ARA_API).
//!
//! ARA2 (Audio Random Access) lets audio plugins access DAW audio regions
//! directly — not just streaming audio at the insert point. This crate
//! wraps the C API as Rust traits with vtable builders.
//!
//! ## How ARA2 Works
//!
//! The DAW calls into your plugin through C structs of function pointers
//! (vtables). Each vtable has a `structSize` field that tells the host
//! which functions are present; null entries mean "not supported."
//!
//! This crate implements 25 of 55 vtable entries. The remaining 30 return
//! null pointers per the ARA2 spec.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use ara2_bridge::*;
//! use ara2_bridge_sys::*;
//!
//! struct MyPlugin;
//!
//! impl DocumentController for MyPlugin {
//!     // ... implement all 25 methods
//! }
//!
//! // Build the C-compatible instance for the DAW
//! let controller = Box::new(MyPlugin);
//! let instance = build_document_controller_instance(controller);
//! ```

use ara2_bridge_sys::*;

// ────────────────────────────────────────────────────────────────────
// DocumentController — the main plugin interface
// ────────────────────────────────────────────────────────────────────

/// Plugin-side document controller.
///
/// Created once per DAW document. Manages audio sources, musical contexts,
/// region sequences, playback regions, persistence, and undo history.
///
/// The DAW calls these methods through the function pointer vtable
/// built by [`build_document_controller_instance`].
///
/// ## Lifecycle
///
/// 1. DAW creates a document → calls your `ARAFactory` callback
/// 2. You return an `ARADocumentControllerInstance`
/// 3. DAW calls `begin_editing()` → you set up internal state
/// 4. DAW calls `create_audio_source()`, `create_musical_context()`, etc.
/// 5. DAW calls `request_audio_source_content_analysis()` → you analyze
/// 6. DAW calls `end_editing()` → you commit changes
/// 7. DAW calls `store_objects_to_archive()` → you serialize to project
/// 8. DAW calls `destroy()` → you free resources
pub trait DocumentController {
    /// Destroy the controller and free all resources.
    ///
    /// Called by the DAW when the document is closed. After this call,
    /// no further callbacks will be received.
    fn destroy(&mut self);

    /// Return a pointer to the static [`ARAFactory`] that created this controller.
    ///
    /// The returned pointer must remain valid for the lifetime of the controller.
    /// This is how the DAW discovers the plugin's capabilities (name, version,
    /// supported content types, archive IDs).
    fn get_factory(&self) -> *const ARAFactory;

    /// Begin an editing session.
    ///
    /// All model changes between `begin_editing` and [`end_editing`] are
    /// treated as a single undoable operation. The plugin may defer
    /// expensive updates until `end_editing` is called.
    ///
    /// [`end_editing`]: DocumentController::end_editing
    fn begin_editing(&mut self);

    /// End an editing session.
    ///
    /// The plugin should now perform any deferred updates and notify the
    /// host of changes via `ARAModelUpdateControllerInterface`.
    fn end_editing(&mut self);

    /// Send all pending model update notifications to the host.
    ///
    /// Called periodically by the DAW when not editing. The plugin may
    /// call back into the host using `ARAModelUpdateControllerInterface`
    /// during this call.
    fn notify_model_updates(&mut self);

    /// Handle a change to the document's properties (name, etc.).
    fn update_document_properties(&mut self, properties: &ARADocumentProperties);

    /// Create a new audio source associated with this document.
    ///
    /// Audio sources represent the raw audio data that playback regions
    /// reference. Sample data access is initially disabled — call
    /// `enable_audio_source_samples_access` to grant access.
    ///
    /// Returns an opaque reference that identifies the source in future
    /// callbacks.
    fn create_audio_source(
        &mut self,
        host_ref: ARAAudioSourceHostRef,
        properties: &ARAAudioSourceProperties,
    ) -> ARAAudioSourceRef;

    /// Update properties of an existing audio source.
    ///
    /// Called when sample rate, channel count, or name changes.
    /// All properties are provided; the plugin determines which changed.
    fn update_audio_source_properties(
        &mut self,
        source: ARAAudioSourceRef,
        properties: &ARAAudioSourceProperties,
    );

    /// Called when audio sample data or content information for a source changes.
    ///
    /// `range` is `None` if the entire source is affected. The plugin should
    /// invalidate any cached analysis for the affected range.
    fn update_audio_source_content(
        &mut self,
        source: ARAAudioSourceRef,
        range: Option<&ARAContentTimeRange>,
        flags: ARAContentUpdateFlags,
    );

    /// Grant or revoke access to audio sample data for a source.
    ///
    /// When `enable` is non-zero, the plugin may read audio samples
    /// through the host's audio access controller. When zero, sample
    /// access is revoked and any cached samples should be freed.
    fn enable_audio_source_samples_access(&mut self, source: ARAAudioSourceRef, enable: ARABool);

    /// Manage undo history state for an audio source.
    ///
    /// `deactivate` is non-zero when the host is about to purge undo
    /// history that references this source.
    fn deactivate_audio_source_for_undo_history(
        &mut self,
        source: ARAAudioSourceRef,
        deactivate: ARABool,
    );

    /// Destroy an audio source and free its resources.
    fn destroy_audio_source(&mut self, source: ARAAudioSourceRef);

    /// Host requests analysis of specific content types for an audio source.
    ///
    /// This is where you plug in your analysis engine. The content types
    /// are a subset of the plugin's `analyzeableContentTypes` from the
    /// [`ARAFactory`]. When analysis completes, notify the host via
    /// `ARAModelUpdateControllerInterface`.
    ///
    /// `count` is the number of entries in `content_types`.
    fn request_audio_source_content_analysis(
        &mut self,
        source: ARAAudioSourceRef,
        count: ARASize,
        content_types: *const ARAContentType,
    ) -> ARABool;

    /// Check whether analysis data is available for a given content type.
    ///
    /// Returns non-zero if the plugin has analysis results ready.
    fn is_audio_source_content_available(
        &self,
        source: ARAAudioSourceRef,
        content_type: ARAContentType,
    ) -> ARABool;

    /// Create a musical context (tempo, time signature, key).
    fn create_musical_context(
        &mut self,
        host_ref: ARAMusicalContextHostRef,
        properties: &ARAMusicalContextProperties,
    ) -> ARAMusicalContextRef;

    /// Update properties of an existing musical context.
    fn update_musical_context_properties(
        &mut self,
        ctx: ARAMusicalContextRef,
        properties: &ARAMusicalContextProperties,
    );

    /// Called when musical context content changes.
    fn update_musical_context_content(
        &mut self,
        ctx: ARAMusicalContextRef,
        range: Option<&ARAContentTimeRange>,
        flags: ARAContentUpdateFlags,
    );

    /// Destroy a musical context.
    fn destroy_musical_context(&mut self, ctx: ARAMusicalContextRef);

    /// Create a region sequence (time-ordered regions on a track).
    fn create_region_sequence(
        &mut self,
        host_ref: ARARegionSequenceHostRef,
        properties: &ARARegionSequenceProperties,
    ) -> ARARegionSequenceRef;

    /// Update properties of an existing region sequence.
    fn update_region_sequence_properties(
        &mut self,
        seq: ARARegionSequenceRef,
        properties: &ARARegionSequenceProperties,
    );

    /// Destroy a region sequence.
    fn destroy_region_sequence(&mut self, seq: ARARegionSequenceRef);

    /// Create a playback region linked to an audio modification.
    ///
    /// Playback regions define where and how audio modifications appear
    /// in the DAW timeline. Each playback region references an audio
    /// modification, which in turn references an audio source.
    fn create_playback_region(
        &mut self,
        host_ref: ARAPlaybackRegionHostRef,
        audio_modification_ref: ARAAudioModificationRef,
        properties: &ARAPlaybackRegionProperties,
    ) -> ARAPlaybackRegionRef;

    /// Update properties of an existing playback region.
    fn update_playback_region_properties(
        &mut self,
        region: ARAPlaybackRegionRef,
        properties: &ARAPlaybackRegionProperties,
    );

    /// Destroy a playback region.
    fn destroy_playback_region(&mut self, region: ARAPlaybackRegionRef);

    /// Serialize document state for DAW project save.
    ///
    /// Write analysis data through the provided archive writer.
    /// `filter` is `None` if all objects should be stored, or a
    /// filter specifying which objects to include.
    ///
    /// Returns non-zero on success.
    fn store_objects_to_archive(
        &mut self,
        archive_writer_host_ref: ARAArchiveWriterHostRef,
        filter: *const ARAStoreObjectsFilter,
    ) -> ARABool;

    /// Deserialize document state from DAW project load.
    ///
    /// Read analysis data through the provided archive reader.
    /// `filter` is `None` if all objects should be restored, or a
    /// filter specifying which objects to include.
    ///
    /// Returns non-zero on success.
    fn restore_objects_from_archive(
        &mut self,
        archive_reader_host_ref: ARAArchiveReaderHostRef,
        filter: *const ARARestoreObjectsFilter,
    ) -> ARABool;
}

// ────────────────────────────────────────────────────────────────────
// Vtable + Instance Builder
// ────────────────────────────────────────────────────────────────────

struct ControllerState {
    controller: Box<dyn DocumentController>,
}

/// Build a C-compatible [`ARADocumentControllerInstance`] from a trait object.
///
/// Returns a heap-allocated instance that the DAW will destroy by calling
/// the `destroyDocumentController` vtable entry. The returned pointer
/// should be returned from your `ARAFactory::createDocumentControllerWithDocument`
/// callback.
///
/// ## Safety
///
/// The returned pointer must eventually be freed by the DAW calling
/// `destroyDocumentController`. If the DAW never calls it, the memory
/// will leak.
///
/// ## Example
///
/// ```rust,ignore
/// use ara2_bridge::*;
/// use ara2_bridge_sys::*;
///
/// unsafe extern "C" fn factory_callback(
///     _host: *const ARADocumentControllerHostInstance,
///     _props: *const ARADocumentProperties,
/// ) -> *const ARADocumentControllerInstance {
///     let controller = Box::new(MyPlugin);
///     build_document_controller_instance(controller)
/// }
/// ```
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
    r: ARADocumentControllerRef,
    p: *const ARADocumentProperties,
) {
    state(r).controller.update_document_properties(&*p);
}

unsafe extern "C" fn create_audio_source_cb(
    r: ARADocumentControllerRef,
    h: ARAAudioSourceHostRef,
    p: *const ARAAudioSourceProperties,
) -> ARAAudioSourceRef {
    state(r).controller.create_audio_source(h, &*p)
}

unsafe extern "C" fn update_audio_source_props_cb(
    r: ARADocumentControllerRef,
    s: ARAAudioSourceRef,
    p: *const ARAAudioSourceProperties,
) {
    state(r).controller.update_audio_source_properties(s, &*p);
}

unsafe extern "C" fn update_audio_source_content_cb(
    r: ARADocumentControllerRef,
    s: ARAAudioSourceRef,
    range: *const ARAContentTimeRange,
    flags: ARAContentUpdateFlags,
) {
    let opt = if range.is_null() { None } else { Some(&*range) };
    state(r)
        .controller
        .update_audio_source_content(s, opt, flags);
}

unsafe extern "C" fn enable_audio_access_cb(
    r: ARADocumentControllerRef,
    s: ARAAudioSourceRef,
    enable: ARABool,
) {
    state(r)
        .controller
        .enable_audio_source_samples_access(s, enable);
}

unsafe extern "C" fn deactivate_audio_undo_cb(
    r: ARADocumentControllerRef,
    s: ARAAudioSourceRef,
    deactivate: ARABool,
) {
    state(r)
        .controller
        .deactivate_audio_source_for_undo_history(s, deactivate);
}

unsafe extern "C" fn destroy_audio_source_cb(r: ARADocumentControllerRef, s: ARAAudioSourceRef) {
    state(r).controller.destroy_audio_source(s);
}

unsafe extern "C" fn request_audio_analysis_cb(
    r: ARADocumentControllerRef,
    s: ARAAudioSourceRef,
    count: ARASize,
    types: *const ARAContentType,
) {
    state(r)
        .controller
        .request_audio_source_content_analysis(s, count, types);
}

unsafe extern "C" fn is_audio_content_avail_cb(
    r: ARADocumentControllerRef,
    s: ARAAudioSourceRef,
    ct: ARAContentType,
) -> ARABool {
    state(r).controller.is_audio_source_content_available(s, ct)
}

unsafe extern "C" fn create_musical_ctx_cb(
    r: ARADocumentControllerRef,
    h: ARAMusicalContextHostRef,
    p: *const ARAMusicalContextProperties,
) -> ARAMusicalContextRef {
    state(r).controller.create_musical_context(h, &*p)
}

unsafe extern "C" fn update_musical_ctx_props_cb(
    r: ARADocumentControllerRef,
    c: ARAMusicalContextRef,
    p: *const ARAMusicalContextProperties,
) {
    state(r)
        .controller
        .update_musical_context_properties(c, &*p);
}

unsafe extern "C" fn update_musical_ctx_content_cb(
    r: ARADocumentControllerRef,
    c: ARAMusicalContextRef,
    range: *const ARAContentTimeRange,
    flags: ARAContentUpdateFlags,
) {
    let opt = if range.is_null() { None } else { Some(&*range) };
    state(r)
        .controller
        .update_musical_context_content(c, opt, flags);
}

unsafe extern "C" fn destroy_musical_ctx_cb(r: ARADocumentControllerRef, c: ARAMusicalContextRef) {
    state(r).controller.destroy_musical_context(c);
}

unsafe extern "C" fn create_region_seq_cb(
    r: ARADocumentControllerRef,
    h: ARARegionSequenceHostRef,
    p: *const ARARegionSequenceProperties,
) -> ARARegionSequenceRef {
    state(r).controller.create_region_sequence(h, &*p)
}

unsafe extern "C" fn update_region_seq_props_cb(
    r: ARADocumentControllerRef,
    s: ARARegionSequenceRef,
    p: *const ARARegionSequenceProperties,
) {
    state(r)
        .controller
        .update_region_sequence_properties(s, &*p);
}

unsafe extern "C" fn destroy_region_seq_cb(r: ARADocumentControllerRef, s: ARARegionSequenceRef) {
    state(r).controller.destroy_region_sequence(s);
}

unsafe extern "C" fn create_playback_region_cb(
    r: ARADocumentControllerRef,
    m: ARAAudioModificationRef,
    h: ARAPlaybackRegionHostRef,
    p: *const ARAPlaybackRegionProperties,
) -> ARAPlaybackRegionRef {
    state(r).controller.create_playback_region(h, m, &*p)
}

unsafe extern "C" fn update_playback_region_props_cb(
    r: ARADocumentControllerRef,
    reg: ARAPlaybackRegionRef,
    p: *const ARAPlaybackRegionProperties,
) {
    state(r)
        .controller
        .update_playback_region_properties(reg, &*p);
}

unsafe extern "C" fn destroy_playback_region_cb(
    r: ARADocumentControllerRef,
    reg: ARAPlaybackRegionRef,
) {
    state(r).controller.destroy_playback_region(reg);
}

unsafe extern "C" fn store_objects_cb(
    r: ARADocumentControllerRef,
    wr: ARAArchiveWriterHostRef,
    f: *const ARAStoreObjectsFilter,
) -> ARABool {
    state(r).controller.store_objects_to_archive(wr, f)
}

unsafe extern "C" fn restore_objects_cb(
    r: ARADocumentControllerRef,
    rr: ARAArchiveReaderHostRef,
    f: *const ARARestoreObjectsFilter,
) -> ARABool {
    state(r).controller.restore_objects_from_archive(rr, f)
}

// ────────────────────────────────────────────────────────────────────
// Host-side traits — what the DAW implements for ARA2 plugins
// ────────────────────────────────────────────────────────────────────

/// Host-side playback region interface.
///
/// The plugin calls these methods to notify the DAW about content
/// analysis results and rendering progress on this playback region.
pub trait PlaybackRegionHost {
    /// Plugin has completed content analysis for this region.
    fn notify_content_analysis_completed(
        &mut self,
        region: ARAPlaybackRegionRef,
        content: *mut std::ffi::c_void,
    );

    /// Plugin requests that the host start rendering this region.
    fn request_playback(&mut self, region: ARAPlaybackRegionRef);
}

/// Host-side model update interface.
///
/// The plugin calls these methods to tell the DAW that model objects
/// (audio sources, musical contexts, etc.) have changed.
pub trait ModelUpdateController {
    /// Notify the host that an audio source's content has changed.
    fn notify_audio_source_content_changed(
        &mut self,
        source: ARAAudioSourceRef,
        content: *const std::ffi::c_void,
    );

    /// Notify the host that new model objects are available.
    fn notify_model_update(&mut self);

    /// Notify the host that the plugin session has been restored from archive.
    fn notify_restored_from_archive(&mut self);
}

/// Host-side archive reader interface.
///
/// The plugin calls these methods during `restore_objects_from_archive()`
/// to read back serialized state from the DAW project file.
pub trait ArchiveReaderHost {
    /// Read bytes from the archive into `buffer`.
    /// Returns the number of bytes actually read.
    fn read(&mut self, buffer: &mut [u8]) -> usize;

    /// Return the number of bytes available to read.
    fn size(&self) -> usize;
}

/// Host-side archive writer interface.
///
/// The plugin calls these methods during `store_objects_to_archive()`
/// to save plugin state into the DAW project file.
pub trait ArchiveWriterHost {
    /// Write bytes to the archive.
    fn write(&mut self, data: &[u8]);
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal plugin implementation for testing
    struct TestPlugin {
        destroyed: bool,
        editing: bool,
    }

    impl DocumentController for TestPlugin {
        fn destroy(&mut self) {
            self.destroyed = true;
        }
        fn get_factory(&self) -> *const ARAFactory {
            std::ptr::null()
        }
        fn begin_editing(&mut self) {
            self.editing = true;
        }
        fn end_editing(&mut self) {
            self.editing = false;
        }
        fn notify_model_updates(&mut self) {}
        fn update_document_properties(&mut self, _: &ARADocumentProperties) {}
        fn create_audio_source(
            &mut self,
            _: ARAAudioSourceHostRef,
            _: &ARAAudioSourceProperties,
        ) -> ARAAudioSourceRef {
            std::ptr::null_mut()
        }
        fn update_audio_source_properties(
            &mut self,
            _: ARAAudioSourceRef,
            _: &ARAAudioSourceProperties,
        ) {
        }
        fn update_audio_source_content(
            &mut self,
            _: ARAAudioSourceRef,
            _: Option<&ARAContentTimeRange>,
            _: ARAContentUpdateFlags,
        ) {
        }
        fn enable_audio_source_samples_access(&mut self, _: ARAAudioSourceRef, _: ARABool) {}
        fn deactivate_audio_source_for_undo_history(&mut self, _: ARAAudioSourceRef, _: ARABool) {}
        fn destroy_audio_source(&mut self, _: ARAAudioSourceRef) {}
        fn request_audio_source_content_analysis(
            &mut self,
            _: ARAAudioSourceRef,
            _: ARASize,
            _: *const ARAContentType,
        ) -> ARABool {
            1
        }
        fn is_audio_source_content_available(
            &self,
            _: ARAAudioSourceRef,
            _: ARAContentType,
        ) -> ARABool {
            1
        }
        fn create_musical_context(
            &mut self,
            _: ARAMusicalContextHostRef,
            _: &ARAMusicalContextProperties,
        ) -> ARAMusicalContextRef {
            std::ptr::null_mut()
        }
        fn update_musical_context_properties(
            &mut self,
            _: ARAMusicalContextRef,
            _: &ARAMusicalContextProperties,
        ) {
        }
        fn update_musical_context_content(
            &mut self,
            _: ARAMusicalContextRef,
            _: Option<&ARAContentTimeRange>,
            _: ARAContentUpdateFlags,
        ) {
        }
        fn destroy_musical_context(&mut self, _: ARAMusicalContextRef) {}
        fn create_region_sequence(
            &mut self,
            _: ARARegionSequenceHostRef,
            _: &ARARegionSequenceProperties,
        ) -> ARARegionSequenceRef {
            std::ptr::null_mut()
        }
        fn update_region_sequence_properties(
            &mut self,
            _: ARARegionSequenceRef,
            _: &ARARegionSequenceProperties,
        ) {
        }
        fn destroy_region_sequence(&mut self, _: ARARegionSequenceRef) {}
        fn create_playback_region(
            &mut self,
            _: ARAPlaybackRegionHostRef,
            _: ARAAudioModificationRef,
            _: &ARAPlaybackRegionProperties,
        ) -> ARAPlaybackRegionRef {
            std::ptr::null_mut()
        }
        fn update_playback_region_properties(
            &mut self,
            _: ARAPlaybackRegionRef,
            _: &ARAPlaybackRegionProperties,
        ) {
        }
        fn destroy_playback_region(&mut self, _: ARAPlaybackRegionRef) {}
        fn store_objects_to_archive(
            &mut self,
            _: ARAArchiveWriterHostRef,
            _: *const ARAStoreObjectsFilter,
        ) -> ARABool {
            1
        }
        fn restore_objects_from_archive(
            &mut self,
            _: ARAArchiveReaderHostRef,
            _: *const ARARestoreObjectsFilter,
        ) -> ARABool {
            1
        }
    }

    #[test]
    fn test_plugin_lifecycle() {
        let mut plugin = TestPlugin {
            destroyed: false,
            editing: false,
        };
        assert!(!plugin.destroyed);
        plugin.begin_editing();
        assert!(plugin.editing);
        plugin.end_editing();
        assert!(!plugin.editing);
        plugin.destroy();
        assert!(plugin.destroyed);
    }

    #[test]
    fn test_build_instance_creates_vtable() {
        let plugin = Box::new(TestPlugin {
            destroyed: false,
            editing: false,
        });
        let instance = build_document_controller_instance(plugin);
        // The instance has a valid vtable with the correct structSize
        assert!(unsafe { (*(*instance).documentControllerInterface).structSize } > 0);
    }
}
