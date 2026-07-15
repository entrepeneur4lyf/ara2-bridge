//! Crash-conscious same-directory audio-file replacement.

use super::{rewrite_ixml, AraChunkSet, AudioFileError, ChunkLimits};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// An atomic path rewrite failure with an optional retained output path.
#[derive(Debug, thiserror::Error)]
#[error("atomic audio-file replacement failed: {source}")]
pub struct PathRewriteError {
    source: AudioFileError,
    temporary_path: Option<PathBuf>,
}

impl PathRewriteError {
    /// Returns the underlying container, XML, or I/O failure.
    pub fn source(&self) -> &AudioFileError {
        &self.source
    }

    /// Returns a complete temporary output retained after a final rename failure.
    pub fn temporary_path(&self) -> Option<&Path> {
        self.temporary_path.as_deref()
    }
}

/// Atomically replaces the ARA iXML dictionary in an audio file.
///
/// The original path must not be a symbolic link. Output is written beside the original, checked,
/// synced, assigned the original permissions, and renamed only after all validation succeeds.
pub fn replace_ara_in_path(
    path: impl AsRef<Path>,
    chunk: &AraChunkSet,
) -> Result<(), PathRewriteError> {
    replace_ixml_in_path(path.as_ref(), &chunk.emit())
}

fn replace_ixml_in_path(path: &Path, xml: &[u8]) -> Result<(), PathRewriteError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| PathRewriteError {
        source: AudioFileError::Io(error),
        temporary_path: None,
    })?;
    if metadata.file_type().is_symlink() {
        return Err(PathRewriteError {
            source: AudioFileError::SymlinkRefused,
            temporary_path: None,
        });
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let (temporary_path, mut output) =
        create_temporary(parent, path).map_err(|error| PathRewriteError {
            source: AudioFileError::Io(error),
            temporary_path: None,
        })?;
    let result = (|| -> Result<(), AudioFileError> {
        let mut input = File::open(path)?;
        rewrite_ixml(&mut input, &mut output, Some(xml))?;
        output.sync_all()?;
        output.set_permissions(metadata.permissions())?;
        output.seek(SeekFrom::Start(0))?;
        let actual = AraChunkSet::from_audio_reader(&mut output, ChunkLimits::default())?
            .ok_or(AudioFileError::Invalid("rewritten file has no iXML chunk"))?;
        if actual != AraChunkSet::parse(xml)? {
            return Err(AudioFileError::Invalid(
                "rewritten ARA dictionary failed validation",
            ));
        }
        output.sync_all()?;
        Ok(())
    })();
    if let Err(source) = result {
        drop(output);
        let _ = std::fs::remove_file(&temporary_path);
        return Err(PathRewriteError {
            source,
            temporary_path: None,
        });
    }
    drop(output);
    if let Err(error) = std::fs::rename(&temporary_path, path) {
        return Err(PathRewriteError {
            source: AudioFileError::Io(error),
            temporary_path: Some(temporary_path),
        });
    }
    sync_directory(parent).map_err(|error| PathRewriteError {
        source: AudioFileError::Io(error),
        temporary_path: None,
    })?;
    Ok(())
}

fn create_temporary(parent: &Path, original: &Path) -> std::io::Result<(PathBuf, File)> {
    let name = original
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("audio");
    for _ in 0..128 {
        let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".{name}.ara2-bridge-{}-{sequence}.tmp",
            std::process::id()
        ));
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a unique same-directory temporary file",
    ))
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}
