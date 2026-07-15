//! Required document-model capability traits.

use crate::{CreateContext, HostContentScope};
use ara2_bridge_core::{
    AraError, AudioModificationProperties, AudioSourceProperties, ContentTimeRange,
    ContentUpdateScopes, DocumentProperties, MusicalContextProperties, PlaybackRegionProperties,
    RegionSequenceProperties,
};

/// Creates, updates, and tears down application-owned document state.
pub trait DocumentLifecycle {
    /// Application state retained for the document-controller lifetime.
    type Document: Send + 'static;

    /// Creates the document state after the runtime has validated host input.
    fn create_document(
        &mut self,
        context: &CreateContext,
        properties: DocumentProperties,
    ) -> Result<Self::Document, AraError>;

    /// Applies copied document display properties.
    fn update_document(
        &mut self,
        _document: &mut Self::Document,
        _properties: DocumentProperties,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Observes the start of a host editing session before graph callbacks are accepted.
    fn begin_editing(&mut self, _document: &mut Self::Document) -> Result<(), AraError> {
        Ok(())
    }

    /// Finalizes deferred model work with host access valid for any live context or source.
    fn end_editing(
        &mut self,
        _document: &mut Self::Document,
        _host: &HostContentScope<'_, '_>,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Observes document teardown after all child identities are invalidated.
    fn destroy_document(&mut self, _document: Self::Document) {}
}

/// Owns application state for musical contexts.
pub trait MusicalContexts {
    /// Application state retained for one musical context.
    type MusicalContext: Send + 'static;

    /// Creates state for a provisionally registered musical-context identity.
    fn create_musical_context(
        &mut self,
        context: &CreateContext,
        properties: MusicalContextProperties,
        host: &HostContentScope<'_, '_>,
    ) -> Result<Self::MusicalContext, AraError>;

    /// Applies copied musical-context properties.
    fn update_musical_context(
        &mut self,
        _state: &mut Self::MusicalContext,
        _properties: MusicalContextProperties,
        _host: &HostContentScope<'_, '_>,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Applies a host-originated musical-context content change.
    fn update_musical_context_content(
        &mut self,
        _state: &mut Self::MusicalContext,
        _range: Option<ContentTimeRange>,
        _flags: ContentUpdateScopes,
        _host: &HostContentScope<'_, '_>,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Observes teardown after the identity is invalidated.
    fn destroy_musical_context(&mut self, _state: Self::MusicalContext) {}
}

/// Owns application state for region sequences.
pub trait RegionSequences {
    /// Application state retained for one region sequence.
    type RegionSequence: Send + 'static;

    /// Creates state for a provisionally registered region-sequence identity.
    fn create_region_sequence(
        &mut self,
        context: &CreateContext,
        properties: RegionSequenceProperties,
    ) -> Result<Self::RegionSequence, AraError>;

    /// Applies copied region-sequence properties.
    fn update_region_sequence(
        &mut self,
        _state: &mut Self::RegionSequence,
        _properties: RegionSequenceProperties,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Observes teardown after the identity is invalidated.
    fn destroy_region_sequence(&mut self, _state: Self::RegionSequence) {}
}

/// Owns application state for host audio sources.
pub trait AudioSources {
    /// Application state retained for one audio source.
    type AudioSource: Send + 'static;

    /// Creates state for a provisionally registered audio-source identity.
    fn create_audio_source(
        &mut self,
        context: &CreateContext,
        properties: AudioSourceProperties,
        host: &HostContentScope<'_, '_>,
    ) -> Result<Self::AudioSource, AraError>;

    /// Applies copied audio-source properties.
    fn update_audio_source(
        &mut self,
        _state: &mut Self::AudioSource,
        _properties: AudioSourceProperties,
        _host: &HostContentScope<'_, '_>,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Applies a host-originated audio-source content change.
    fn update_audio_source_content(
        &mut self,
        _state: &mut Self::AudioSource,
        _range: Option<ContentTimeRange>,
        _flags: ContentUpdateScopes,
        _host: &HostContentScope<'_, '_>,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Enables or synchronously revokes host sample access for this source.
    fn enable_audio_source_samples_access(
        &mut self,
        _state: &mut Self::AudioSource,
        _enable: bool,
        _host: &HostContentScope<'_, '_>,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Observes undo-history activation changes under the current source's audio-access scope.
    fn deactivate_audio_source(
        &mut self,
        _state: &mut Self::AudioSource,
        _deactivate: bool,
        _host: &HostContentScope<'_, '_>,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Observes teardown after the identity is invalidated.
    fn destroy_audio_source(
        &mut self,
        _state: Self::AudioSource,
        _host: &HostContentScope<'_, '_>,
    ) {
    }
}

/// Owns application state for audio modifications and clones.
pub trait AudioModifications {
    /// Application state retained for one audio modification.
    type AudioModification: Send + 'static;

    /// Creates state for a provisionally registered audio-modification identity.
    fn create_audio_modification(
        &mut self,
        context: &CreateContext,
        properties: AudioModificationProperties,
    ) -> Result<Self::AudioModification, AraError>;

    /// Clones application state into a distinct provisional identity.
    fn clone_audio_modification(
        &mut self,
        context: &CreateContext,
        source: &Self::AudioModification,
        properties: AudioModificationProperties,
    ) -> Result<Self::AudioModification, AraError>;

    /// Applies copied audio-modification properties.
    fn update_audio_modification(
        &mut self,
        _state: &mut Self::AudioModification,
        _properties: AudioModificationProperties,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Observes undo-history activation changes after graph-order validation.
    fn deactivate_audio_modification(
        &mut self,
        _state: &mut Self::AudioModification,
        _deactivate: bool,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Observes teardown after the identity is invalidated.
    fn destroy_audio_modification(&mut self, _state: Self::AudioModification) {}
}

/// Owns application state for playback regions.
pub trait PlaybackRegions {
    /// Application state retained for one playback region.
    type PlaybackRegion: Send + 'static;

    /// Creates state for a provisionally registered playback-region identity.
    fn create_playback_region(
        &mut self,
        context: &CreateContext,
        properties: PlaybackRegionProperties,
    ) -> Result<Self::PlaybackRegion, AraError>;

    /// Applies copied playback-region properties.
    fn update_playback_region(
        &mut self,
        _state: &mut Self::PlaybackRegion,
        _properties: PlaybackRegionProperties,
    ) -> Result<(), AraError> {
        Ok(())
    }

    /// Observes teardown after the identity is invalidated.
    fn destroy_playback_region(&mut self, _state: Self::PlaybackRegion) {}
}
