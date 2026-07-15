use ara2_bridge_core::{AraError, Diagnostic, Lifecycle};
use std::sync::Arc;

#[test]
fn edit_restore_and_poison_transitions_are_checked() {
    let state = Lifecycle::new_on_current_thread();
    let edit = state.begin_editing().unwrap();
    assert!(matches!(
        state.begin_editing(),
        Err(AraError::InvalidState("editing is already active"))
    ));
    edit.finish().unwrap();

    state.poison(Diagnostic::new(AraError::Poisoned));
    assert!(matches!(state.begin_editing(), Err(AraError::Poisoned)));
    assert!(state.begin_teardown().is_ok());
}

#[test]
fn ara1_and_ara2_restore_sequences_are_distinct_and_balanced() {
    let state = Lifecycle::new_on_current_thread();
    let legacy = state.begin_ara1_restore().unwrap();
    assert!(state.begin_editing().is_err());
    legacy.finish().unwrap();

    let restore = state.begin_ara2_restore().unwrap();
    assert!(state.begin_editing().is_err());
    restore.finish().unwrap();

    let edit = state.begin_editing().unwrap();
    assert!(state.begin_ara1_restore().is_err());
    edit.finish().unwrap();
}

#[test]
fn sample_content_and_teardown_states_are_checked() {
    let state = Lifecycle::new_on_current_thread();
    let samples = state.begin_sample_access().unwrap();
    let content = state.begin_content_call().unwrap();
    assert!(state.begin_content_call().is_err());
    assert!(state.begin_teardown().is_err());
    content.finish().unwrap();
    samples.finish().unwrap();
    state.begin_teardown().unwrap().finish().unwrap();
    assert!(state.begin_editing().is_err());
}

#[test]
fn model_operations_reject_the_wrong_thread() {
    let state = Arc::new(Lifecycle::new_on_current_thread());
    let worker = Arc::clone(&state);
    let error = std::thread::spawn(move || match worker.begin_editing() {
        Ok(_) => panic!("wrong-thread edit unexpectedly succeeded"),
        Err(error) => error,
    })
    .join()
    .unwrap();
    assert!(matches!(
        error,
        AraError::InvalidThread("operation requires the ARA model thread")
    ));
}
