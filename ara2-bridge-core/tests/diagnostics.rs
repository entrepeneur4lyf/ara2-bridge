use ara2_bridge_core::{
    AraError, BoundedDiagnosticSink, Diagnostic, DiagnosticSink, DocumentId, InstanceId,
};

#[test]
fn diagnostic_retains_error_location_and_identity() {
    let diagnostic = Diagnostic::new(AraError::InvalidState("not editing"))
        .at("ARADocumentControllerInterface", "createAudioSource")
        .with_document(DocumentId::new(3))
        .with_instance(InstanceId::new(7));

    assert_eq!(
        diagnostic.interface(),
        Some("ARADocumentControllerInterface")
    );
    assert_eq!(diagnostic.method(), Some("createAudioSource"));
    assert_eq!(diagnostic.document(), Some(DocumentId::new(3)));
    assert_eq!(diagnostic.instance(), Some(InstanceId::new(7)));
    assert_eq!(diagnostic.message(), "invalid state: not editing");
    assert!(matches!(
        diagnostic.error(),
        AraError::InvalidState("not editing")
    ));
}

#[test]
fn bounded_sink_evicts_oldest_and_accepts_owned_messages() {
    let sink = BoundedDiagnosticSink::new(2).unwrap();
    sink.record(Diagnostic::new(AraError::Peer("first")));
    sink.record(
        Diagnostic::new(AraError::Peer("second")).with_message(String::from("owned second")),
    );
    sink.record(Diagnostic::new(AraError::Peer("third")));

    let diagnostics = sink.snapshot();
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].message(), "owned second");
    assert_eq!(diagnostics[1].message(), "peer failure: third");
    assert_eq!(sink.capacity(), 2);
}

#[test]
fn zero_capacity_is_rejected() {
    assert!(matches!(
        BoundedDiagnosticSink::new(0),
        Err(AraError::InvalidArgument(
            "diagnostic capacity must be nonzero"
        ))
    ));
}
