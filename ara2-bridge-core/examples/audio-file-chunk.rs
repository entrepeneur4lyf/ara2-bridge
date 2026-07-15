use ara2_bridge_core::{replace_ara_in_path, AraChunkSet};

const LEGACY_CHUNK: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/legacy-missing-distinct.xml");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chunk = AraChunkSet::parse(LEGACY_CHUNK)?;
    let entry = chunk.get("com.example.archive").expect("fixture entry");
    println!("archive bytes: {}", entry.archive_data().len());

    if let Some(path) = std::env::args_os().nth(1) {
        replace_ara_in_path(path, &chunk)?;
    } else {
        println!("pass a WAVE/AIFF path to atomically write this chunk");
    }
    Ok(())
}
