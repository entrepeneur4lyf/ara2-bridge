use ara2_bridge_core::{
    AraError, ContentReader, ContentReaderBackend, ContentReaderGate, ContentReaderLease,
    DynamicContentReader, Notes, Tempo,
};
use ara2_bridge_sys::{ARAContentNote, ARAContentType};
use std::cell::Cell;
use std::ffi::c_void;
use std::rc::Rc;

#[derive(Debug)]
struct RotatingScratchPeer<'a> {
    _lease: ContentReaderLease<'a>,
    events: Vec<ARAContentNote>,
    scratch: ARAContentNote,
    destroys: Rc<Cell<usize>>,
    raw_type: ARAContentType,
    count_override: Option<i32>,
}

// SAFETY: the peer owns its lease and scratch event; returned pointers refer to scratch until the
// next mutable backend call, and `destroy` only increments the observable one-shot test counter.
unsafe impl ContentReaderBackend for RotatingScratchPeer<'_> {
    fn raw_content_type(&self) -> ARAContentType {
        self.raw_type
    }

    fn event_count(&mut self) -> Result<i32, AraError> {
        self.count_override.map_or_else(
            || {
                i32::try_from(self.events.len())
                    .map_err(|_| AraError::InvalidArgument("too many events"))
            },
            Ok,
        )
    }

    unsafe fn event_data(&mut self, index: i32) -> Result<(*const c_void, usize), AraError> {
        self.scratch = *self
            .events
            .get(index as usize)
            .ok_or(AraError::InvalidArgument("event index"))?;
        Ok((
            (&raw const self.scratch).cast::<c_void>(),
            std::mem::size_of::<ARAContentNote>(),
        ))
    }

    fn destroy(&mut self) {
        self.destroys.set(self.destroys.get() + 1);
    }
}

fn note(pitch: i32, start: f64) -> ARAContentNote {
    ARAContentNote {
        frequency: 440.0,
        pitchNumber: pitch,
        volume: 0.5,
        startPosition: start,
        attackDuration: 0.0,
        noteDuration: 1.0,
        signalDuration: 1.0,
    }
}

fn peer<'a>(
    lease: ContentReaderLease<'a>,
    events: Vec<ARAContentNote>,
    destroys: Rc<Cell<usize>>,
) -> RotatingScratchPeer<'a> {
    RotatingScratchPeer {
        _lease: lease,
        events,
        scratch: note(0, 0.0),
        destroys,
        raw_type: 10,
        count_override: None,
    }
}

#[test]
fn owned_iteration_survives_next_peer_data_call_and_destroys_once() {
    let gate = ContentReaderGate::new();
    let destroys = Rc::new(Cell::new(0));
    let peer = peer(
        gate.acquire().unwrap(),
        vec![note(60, 0.0), note(64, 1.0)],
        Rc::clone(&destroys),
    );
    let mut reader = ContentReader::<Notes>::new(peer).unwrap();
    assert!(gate.require_idle().is_err());
    let first = reader.event(0).unwrap();
    let _second = reader.event(1).unwrap();
    assert_eq!(first.pitch_number(), 60);
    drop(reader);
    assert_eq!(destroys.get(), 1);
    assert!(gate.require_idle().is_ok());
}

#[test]
fn lending_access_is_bounded_non_reentrant_and_zero_copy_for_note_fields() {
    let gate = ContentReaderGate::new();
    let destroys = Rc::new(Cell::new(0));
    let peer = peer(
        gate.acquire().unwrap(),
        vec![note(60, 0.0), note(64, 1.0)],
        Rc::clone(&destroys),
    );
    assert!(gate.acquire().is_err());
    let mut reader = ContentReader::<Notes>::new(peer).unwrap();
    let observed = reader
        .with_event(1, |event| {
            assert!(gate.require_idle().is_err());
            assert_eq!(event.to_owned().pitch_number(), 64);
            (event.pitch_number(), event.start_position())
        })
        .unwrap();
    assert_eq!(observed, (64, 1.0));
    assert!(reader.event(2).is_err());
    drop(reader);
    assert_eq!(destroys.get(), 1);
}

#[test]
fn invalid_creation_and_sequence_errors_still_destroy_once() {
    let gate = ContentReaderGate::new();
    let destroys = Rc::new(Cell::new(0));
    let mut wrong_kind = peer(
        gate.acquire().unwrap(),
        vec![note(60, 0.0)],
        Rc::clone(&destroys),
    );
    wrong_kind.raw_type = 20;
    assert!(ContentReader::<Notes>::new(wrong_kind).is_err());
    assert_eq!(destroys.get(), 1);
    assert!(gate.require_idle().is_ok());

    let peer = peer(
        gate.acquire().unwrap(),
        vec![note(64, 1.0), note(60, 0.0)],
        Rc::clone(&destroys),
    );
    let mut reader = ContentReader::<Notes>::new(peer).unwrap();
    assert!(reader.event(1).is_err());
    drop(reader);
    assert_eq!(destroys.get(), 2);
}

#[test]
fn dynamic_readers_downcast_only_after_exact_kind_validation() {
    let gate = ContentReaderGate::new();
    let destroys = Rc::new(Cell::new(0));
    let peer = peer(
        gate.acquire().unwrap(),
        vec![note(60, 0.0)],
        Rc::clone(&destroys),
    );
    let dynamic = DynamicContentReader::new(peer).unwrap();
    assert!(dynamic.is::<Notes>());
    assert!(!dynamic.is::<Tempo>());
    let mut reader = dynamic.downcast::<Notes>().expect("matching kind");
    assert_eq!(reader.next().unwrap().unwrap().pitch_number(), 60);
    assert!(reader.next().is_none());
    drop(reader);
    assert_eq!(destroys.get(), 1);
}

#[test]
fn panic_during_lending_destroys_during_unwind() {
    let gate = ContentReaderGate::new();
    let destroys = Rc::new(Cell::new(0));
    let peer = peer(
        gate.acquire().unwrap(),
        vec![note(60, 0.0)],
        Rc::clone(&destroys),
    );
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut reader = ContentReader::<Notes>::new(peer).unwrap();
        let _: Result<(), AraError> = reader.with_event(0, |_| panic!("stop"));
    }));
    assert!(result.is_err());
    assert_eq!(destroys.get(), 1);
    assert!(gate.require_idle().is_ok());
}
