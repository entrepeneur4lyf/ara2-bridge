use ara2_bridge_core::{
    read_ixml, read_ixml_with_limit, replace_ara_in_path, rewrite_ixml, AraChunkSet, AudioFileError,
};
use std::io::{Cursor, Seek, SeekFrom, Write};

const FULL_XML: &[u8] = include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/full-2.3.xml");
const LEGACY_XML: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/chunks/legacy-missing-distinct.xml");
const WAVE: &[u8] = include_bytes!("../../ara2-bridge-testkit/fixtures/audio/wave-unknown-odd.wav");
const RF64: &[u8] = include_bytes!("../../ara2-bridge-testkit/fixtures/audio/rf64-ds64.wav");
const BW64: &[u8] = include_bytes!("../../ara2-bridge-testkit/fixtures/audio/bw64-ds64.wav");
const AIFF: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/audio/aiff-unknown-odd.aiff");
const AIFC: &[u8] =
    include_bytes!("../../ara2-bridge-testkit/fixtures/audio/aifc-unknown-odd.aifc");

#[test]
fn wave_rewrite_preserves_unknown_chunks_and_padding() {
    let output = rewrite(WAVE);
    assert_eq!(chunk_le(&output, *b"JUNK"), chunk_le(WAVE, *b"JUNK"));
    assert!(chunk_position(&output, *b"JUNK") < chunk_position(&output, *b"iXML"));
    assert_eq!(
        u32::from_le_bytes(output[4..8].try_into().unwrap()) as usize + 8,
        output.len()
    );
    assert_eq!(
        AraChunkSet::from_audio(&output).unwrap().unwrap(),
        full_set()
    );
}

#[test]
fn rf64_and_bw64_update_ds64_without_demoting_the_container() {
    for input in [RF64, BW64] {
        assert_eq!(
            AraChunkSet::from_audio(input).unwrap().unwrap(),
            AraChunkSet::parse(LEGACY_XML).unwrap()
        );
        let output = rewrite(input);
        assert_eq!(&output[..4], &input[..4]);
        assert_eq!(&output[4..8], &u32::MAX.to_le_bytes());
        assert_eq!(
            u64::from_le_bytes(output[20..28].try_into().unwrap()) as usize + 8,
            output.len()
        );
        assert_eq!(chunk_le(&output, *b"JUNK"), chunk_le(input, *b"JUNK"));
        let ixml = output
            .windows(4)
            .rposition(|window| window == b"iXML")
            .unwrap();
        assert_eq!(&output[ixml + 4..ixml + 8], &u32::MAX.to_le_bytes());
        assert_eq!(
            u64::from_le_bytes(output[52..60].try_into().unwrap()),
            u64::try_from(FULL_XML.len()).unwrap()
        );
        assert_eq!(
            AraChunkSet::from_audio(&output).unwrap().unwrap(),
            full_set()
        );
    }
}

#[test]
fn aiff_and_aifc_rewrite_big_endian_sizes_and_preserve_odd_padding() {
    for input in [AIFF, AIFC] {
        let output = rewrite(input);
        assert_eq!(&output[8..12], &input[8..12]);
        let unknown = if &input[8..12] == b"AIFF" {
            *b"ANNO"
        } else {
            *b"APPL"
        };
        assert_eq!(chunk_be(&output, unknown), chunk_be(input, unknown));
        assert_eq!(
            u32::from_be_bytes(output[4..8].try_into().unwrap()) as usize + 8,
            output.len()
        );
        assert_eq!(
            AraChunkSet::from_audio(&output).unwrap().unwrap(),
            full_set()
        );
    }
}

#[test]
fn ambiguous_ixml_wave64_and_truncation_are_typed_errors() {
    let mut duplicate = WAVE.to_vec();
    push_le_chunk(&mut duplicate, *b"iXML", FULL_XML);
    let size = u32::try_from(duplicate.len() - 8).unwrap();
    duplicate[4..8].copy_from_slice(&size.to_le_bytes());
    assert!(matches!(
        AraChunkSet::from_audio(&duplicate),
        Err(AudioFileError::AmbiguousIxml)
    ));

    let wave64 = b"riff\x2e\x91\xcf\x11\xa5\xd6\x28\xdb\x04\xc1\x00\x00";
    assert!(matches!(
        AraChunkSet::from_audio(wave64),
        Err(AudioFileError::Unsupported("Wave64"))
    ));
    assert!(matches!(
        AraChunkSet::from_audio(&WAVE[..WAVE.len() - 1]),
        Err(AudioFileError::Invalid(_))
    ));
}

#[test]
fn stream_failure_never_mutates_the_input() {
    let original = WAVE.to_vec();
    let mut input = Cursor::new(original.clone());
    let mut output = FailingWriter::new(24);
    assert!(rewrite_ixml(&mut input, &mut output, Some(FULL_XML)).is_err());
    assert_eq!(input.into_inner(), original);
}

#[test]
fn rewrite_can_remove_and_reinsert_the_ixml_chunk() {
    let mut without = Cursor::new(Vec::new());
    rewrite_ixml(&mut Cursor::new(WAVE), &mut without, None).unwrap();
    let without = without.into_inner();
    assert!(AraChunkSet::from_audio(&without).unwrap().is_none());
    assert_eq!(chunk_le(&without, *b"JUNK"), chunk_le(WAVE, *b"JUNK"));

    let mut restored = Cursor::new(Vec::new());
    rewrite_ixml(&mut Cursor::new(&without), &mut restored, Some(FULL_XML)).unwrap();
    assert_eq!(
        AraChunkSet::from_audio(&restored.into_inner())
            .unwrap()
            .unwrap(),
        full_set()
    );
}

#[test]
fn path_replacement_is_atomic_preserves_permissions_and_refuses_symlinks() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("audio.wav");
    std::fs::write(&path, WAVE).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions).unwrap();

    replace_ara_in_path(&path, &full_set()).unwrap();
    assert!(std::fs::metadata(&path).unwrap().permissions().readonly());
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        AraChunkSet::from_audio(&bytes).unwrap().unwrap(),
        full_set()
    );

    #[cfg(unix)]
    {
        let link = temporary.path().join("link.wav");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(matches!(
            replace_ara_in_path(&link, &full_set()),
            Err(error) if matches!(error.source(), AudioFileError::SymlinkRefused)
        ));
    }
}

fn full_set() -> AraChunkSet {
    AraChunkSet::parse(FULL_XML).unwrap()
}

fn rewrite(input: &[u8]) -> Vec<u8> {
    let mut source = Cursor::new(input);
    let mut output = Cursor::new(Vec::new());
    rewrite_ixml(&mut source, &mut output, Some(FULL_XML)).unwrap();
    output.into_inner()
}

fn chunk_position(input: &[u8], id: [u8; 4]) -> usize {
    input.windows(4).position(|window| window == id).unwrap()
}

fn chunk_le(input: &[u8], id: [u8; 4]) -> &[u8] {
    chunk(input, id, u32::from_le_bytes)
}

fn chunk_be(input: &[u8], id: [u8; 4]) -> &[u8] {
    chunk(input, id, u32::from_be_bytes)
}

fn chunk(input: &[u8], id: [u8; 4], decode: fn([u8; 4]) -> u32) -> &[u8] {
    let start = chunk_position(input, id);
    let size = decode(input[start + 4..start + 8].try_into().unwrap()) as usize;
    let end = start + 8 + size + (size & 1);
    &input[start..end]
}

fn push_le_chunk(output: &mut Vec<u8>, id: [u8; 4], data: &[u8]) {
    output.extend_from_slice(&id);
    output.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
    output.extend_from_slice(data);
    if data.len() & 1 != 0 {
        output.push(0);
    }
}

struct FailingWriter {
    inner: Cursor<Vec<u8>>,
    remaining: usize,
}

impl FailingWriter {
    fn new(remaining: usize) -> Self {
        Self {
            inner: Cursor::new(Vec::new()),
            remaining,
        }
    }
}

impl Write for FailingWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::Error::other("injected write failure"));
        }
        let count = buffer.len().min(self.remaining);
        self.remaining -= count;
        self.inner.write(&buffer[..count])
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl Seek for FailingWriter {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(position)
    }
}

#[test]
fn read_ixml_returns_the_exact_embedded_chunk() {
    let mut input = Cursor::new(WAVE);
    assert_eq!(read_ixml(&mut input).unwrap().unwrap(), LEGACY_XML);
    assert!(matches!(
        read_ixml_with_limit(&mut Cursor::new(WAVE), 8),
        Err(AudioFileError::Limit("iXML payload"))
    ));
}
