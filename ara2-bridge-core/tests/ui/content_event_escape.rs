use ara2_bridge_core::{
    ContentReader, ContentReaderBackend, EventRef, Notes,
};

fn escape<B: ContentReaderBackend>(reader: &mut ContentReader<Notes, B>) -> EventRef<'static, Notes> {
    reader.with_event(0, |event| event).unwrap()
}

fn main() {}
