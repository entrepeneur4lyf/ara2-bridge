use ara2_bridge_core::{AraChunkSet, ChunkError, ChunkLimits};

const LEGACY: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/legacy-missing-distinct.xml");
const FULL: &[u8] = include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/full-2.3.xml");
const NAMESPACE: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/namespace-qualified.xml");
const MULTI: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/multi-entry-order.xml");
const UNRELATED: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/unrelated-ordering.xml");

#[test]
fn legacy_chunk_defaults_optional_booleans_to_false() {
    let set = AraChunkSet::parse(LEGACY).unwrap();
    let entry = set.get("com.example.archive").unwrap();
    assert!(!entry.open_automatically());
    assert!(!entry.create_distinct_audio_modification());
    assert!(entry.archive_data().is_empty());
}

#[test]
fn full_chunk_decodes_metadata_and_mime_base64() {
    let set = AraChunkSet::parse(FULL).unwrap();
    let entry = set.get("full.archive").unwrap();
    assert!(entry.open_automatically());
    assert!(entry.create_distinct_audio_modification());
    assert_eq!(entry.archive_data(), &[1, 2, 3, 4]);
    assert_eq!(
        entry.suggested_plug_in().unwrap().plug_in_name.as_deref(),
        Some("Example")
    );
}

#[test]
fn duplicates_invalid_text_entities_and_limits_are_typed_errors() {
    let duplicate = br#"<BWFXML><ARA><audioSources><audioSource><documentArchiveID>a</documentArchiveID><persistentID>x</persistentID><archiveData></archiveData><archiveData></archiveData></audioSource></audioSources></ARA></BWFXML>"#;
    assert_eq!(
        AraChunkSet::parse(duplicate),
        Err(ChunkError::DuplicateElement("archiveData"))
    );
    let entity = br#"<!DOCTYPE x [<!ENTITY e "boom">]><BWFXML><ARA><audioSources/></ARA></BWFXML>"#;
    assert!(AraChunkSet::parse(entity).is_err());
    assert!(AraChunkSet::parse_with_limits(
        FULL,
        ChunkLimits {
            max_xml_bytes: 8,
            ..ChunkLimits::default()
        }
    )
    .is_err());
}

#[test]
fn namespace_and_dictionary_order_survive_canonical_round_trip() {
    let namespaced = AraChunkSet::parse(NAMESPACE).unwrap();
    assert_eq!(
        namespaced.archive_ids().collect::<Vec<_>>(),
        ["first", "second"]
    );
    let namespaced_output = String::from_utf8(namespaced.emit()).unwrap();
    assert!(namespaced_output.contains("<ix:audioSources>"));
    assert!(namespaced_output.contains(
        "<ix:createDistinctAudioModification>false</ix:createDistinctAudioModification>"
    ));

    let set = AraChunkSet::parse(MULTI).unwrap();
    let emitted = set.emit();
    let text = std::str::from_utf8(&emitted).unwrap();
    assert!(text.contains("<openAutomatically>false</openAutomatically>"));
    assert!(
        text.contains("<createDistinctAudioModification>false</createDistinctAudioModification>")
    );
    assert!(!text.contains("\nYQ=="));
    let reparsed = AraChunkSet::parse(&emitted).unwrap();
    assert_eq!(
        reparsed.archive_ids().collect::<Vec<_>>(),
        ["zeta", "alpha", "middle"]
    );
}

#[test]
fn unrelated_nodes_attributes_and_relative_order_survive_rewrite() {
    let emitted = AraChunkSet::parse(UNRELATED).unwrap().emit();
    let text = std::str::from_utf8(&emitted).unwrap();
    let project = text.find("<PROJECT>before</PROJECT>").unwrap();
    let ara = text.find("<ARA>").unwrap();
    let vendor_before = text.find("<vendorBefore code=\"1\"/>").unwrap();
    let sources = text.find("<audioSources vendorSources=\"keep\">").unwrap();
    let dictionary_before = text.find("<vendorDictionaryBefore rank=\"0\"/>").unwrap();
    let source = text.find("<audioSource customSource=\"yes\">").unwrap();
    let source_before = text.find("<vendorSourceBefore rank=\"1\"/>").unwrap();
    let archive_id = text
        .find("<documentArchiveID vendorId=\"keep\">ordered</documentArchiveID>")
        .unwrap();
    let suggestion = text
        .find("<suggestedPlugIn vendorSuggested=\"keep\">")
        .unwrap();
    let vendor_suggestion = text.find("<vendorSuggestion rank=\"2\"/>").unwrap();
    let source_after = text.find("<vendorSourceAfter rank=\"3\"/>").unwrap();
    let dictionary_after = text.find("<vendorDictionaryAfter rank=\"4\"/>").unwrap();
    let vendor_after = text.find("<vendorAfter code=\"2\"/>").unwrap();
    let note = text.find("<NOTE>after</NOTE>").unwrap();
    assert!(project < ara && ara < vendor_before && vendor_before < sources);
    assert!(sources < dictionary_before && dictionary_before < source);
    assert!(source < source_before && source_before < archive_id);
    assert!(archive_id < suggestion && suggestion < vendor_suggestion);
    assert!(vendor_suggestion < source_after && source_after < dictionary_after);
    assert!(sources < vendor_after && vendor_after < note);
    assert!(text.contains("<BWFXML before=\"yes\">"));
    assert!(text.contains("<archiveData vendorData=\"keep\">AA==</archiveData>"));
}
