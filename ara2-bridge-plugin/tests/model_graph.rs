use ara2_bridge_core::{
    ApiGeneration, AraBool, AraError, AudioModificationProperties, AudioSourceProperties,
    DocumentProperties, MusicalContextProperties, PlaybackRegionProperties,
    RegionSequenceProperties,
};
use ara2_bridge_plugin::{
    AudioModifications, AudioSources, DocumentLifecycle, HostContentScope, MusicalContexts,
    PlaybackRegions, PluginRuntime, RegionSequences,
};

#[test]
fn playback_region_requires_live_modification_and_sequence_before_delegation() {
    let mut runtime = fixture_runtime(Delegate::default());
    let mut other = fixture_runtime(Delegate::default());
    let foreign_modification = {
        let mut edit = other.begin_editing().unwrap();
        let source = edit
            .create_audio_source(source_properties("foreign"))
            .unwrap();
        edit.create_audio_modification(source, modification_properties("foreign"))
            .unwrap()
    };

    let (context, sequence, source, modification) = {
        let mut edit = runtime.begin_editing().unwrap();
        let context = edit
            .create_musical_context(
                MusicalContextProperties::new(Some("context"), 0, None).unwrap(),
            )
            .unwrap();
        let context_ref = edit.musical_context_ref(context).unwrap();
        let sequence = edit
            .create_region_sequence(
                context,
                RegionSequenceProperties::new(Some("sequence"), 0, context_ref, None).unwrap(),
            )
            .unwrap();
        let source = edit
            .create_audio_source(source_properties("source"))
            .unwrap();
        let modification = edit
            .create_audio_modification(source, modification_properties("modification"))
            .unwrap();
        (context, sequence, source, modification)
    };
    let calls_before = runtime.delegate().playback_creates;
    let sequence_ref = runtime.region_sequence_ref(sequence).unwrap();
    let properties =
        PlaybackRegionProperties::for_ara2(0, 0.0, 1.0, 0.0, 1.0, sequence_ref, None, None)
            .unwrap();
    let mut edit = runtime.begin_editing().unwrap();
    assert!(edit
        .create_playback_region(foreign_modification, sequence, properties)
        .is_err());
    assert_eq!(edit.delegate().playback_creates, calls_before);

    let sequence_ref = edit.region_sequence_ref(sequence).unwrap();
    let region = edit
        .create_playback_region(
            modification,
            sequence,
            PlaybackRegionProperties::for_ara2(0, 0.0, 1.0, 0.0, 1.0, sequence_ref, None, None)
                .unwrap(),
        )
        .unwrap();
    assert!(edit.destroy_audio_modification(modification).is_err());
    edit.destroy_playback_region(region).unwrap();
    edit.destroy_audio_modification(modification).unwrap();
    assert!(edit.destroy_audio_modification(modification).is_err());
    edit.destroy_audio_source(source).unwrap();
    edit.destroy_region_sequence(sequence).unwrap();
    edit.destroy_musical_context(context).unwrap();
    edit.finish().unwrap();
    runtime.destroy().unwrap();
}

#[test]
fn failed_creation_rolls_back_without_exposing_a_live_object() {
    let mut runtime = fixture_runtime(Delegate {
        fail_next_source: true,
        ..Delegate::default()
    });
    let mut edit = runtime.begin_editing().unwrap();
    assert!(edit
        .create_audio_source(source_properties("rejected"))
        .is_err());
    assert_eq!(edit.live_audio_source_count(), 0);
    assert_eq!(edit.delegate().source_creates, 1);
    assert!(edit.delegate().source_received_provisional_identity);
}

fn fixture_runtime(delegate: Delegate) -> PluginRuntime<Delegate> {
    PluginRuntime::new(
        delegate,
        ApiGeneration::V23Final,
        DocumentProperties::new(Some("document")).unwrap(),
    )
    .unwrap()
}

fn source_properties(id: &str) -> AudioSourceProperties {
    AudioSourceProperties::new(Some(id), id, 48_000, 48_000.0, 2, AraBool::new(false)).unwrap()
}

fn modification_properties(id: &str) -> AudioModificationProperties {
    AudioModificationProperties::new(Some(id), id).unwrap()
}

#[derive(Default)]
struct Delegate {
    source_creates: usize,
    playback_creates: usize,
    fail_next_source: bool,
    source_received_provisional_identity: bool,
}

impl DocumentLifecycle for Delegate {
    type Document = ();

    fn create_document(
        &mut self,
        _: &ara2_bridge_plugin::CreateContext,
        _: DocumentProperties,
    ) -> Result<Self::Document, AraError> {
        Ok(())
    }
}

impl MusicalContexts for Delegate {
    type MusicalContext = ();

    fn create_musical_context(
        &mut self,
        _: &ara2_bridge_plugin::CreateContext,
        _: MusicalContextProperties,
        _: &HostContentScope<'_, '_>,
    ) -> Result<Self::MusicalContext, AraError> {
        Ok(())
    }
}

impl RegionSequences for Delegate {
    type RegionSequence = ();

    fn create_region_sequence(
        &mut self,
        _: &ara2_bridge_plugin::CreateContext,
        _: RegionSequenceProperties,
    ) -> Result<Self::RegionSequence, AraError> {
        Ok(())
    }
}

impl AudioSources for Delegate {
    type AudioSource = ();

    fn create_audio_source(
        &mut self,
        context: &ara2_bridge_plugin::CreateContext,
        _: AudioSourceProperties,
        _: &HostContentScope<'_, '_>,
    ) -> Result<Self::AudioSource, AraError> {
        self.source_creates += 1;
        self.source_received_provisional_identity = context.object_handle().is_some();
        if std::mem::take(&mut self.fail_next_source) {
            Err(AraError::Peer("fixture rejected source"))
        } else {
            Ok(())
        }
    }
}

impl AudioModifications for Delegate {
    type AudioModification = ();

    fn create_audio_modification(
        &mut self,
        _: &ara2_bridge_plugin::CreateContext,
        _: AudioModificationProperties,
    ) -> Result<Self::AudioModification, AraError> {
        Ok(())
    }

    fn clone_audio_modification(
        &mut self,
        _: &ara2_bridge_plugin::CreateContext,
        _: &Self::AudioModification,
        _: AudioModificationProperties,
    ) -> Result<Self::AudioModification, AraError> {
        Ok(())
    }
}

impl PlaybackRegions for Delegate {
    type PlaybackRegion = ();

    fn create_playback_region(
        &mut self,
        _: &ara2_bridge_plugin::CreateContext,
        _: PlaybackRegionProperties,
    ) -> Result<Self::PlaybackRegion, AraError> {
        self.playback_creates += 1;
        Ok(())
    }
}
