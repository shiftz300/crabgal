use std::io::{Error, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use bevy::asset::io::file::FileAssetReader;
use bevy::asset::io::{
    AssetReader, AssetReaderError, AssetReaderFuture, AssetSourceBuilder, PathStream, Reader,
    ReaderNotSeekableError, STACK_FUTURE_SIZE, SeekableReader, StackFuture,
};
use crabgal_loader::{ContentBackend, ContentMount, HexzArchive, HexzCursor, HexzFile};
use futures_lite::io::{AsyncRead, AsyncSeek};

/// Creates Bevy's default source as a deterministic read-only overlay. Sources
/// are configured from low to high priority; the final source wins. Filesystem
/// mounts stay zero-copy and Hexz mounts remain encrypted on disk.
pub(crate) fn overlay_source(mounts: Vec<ContentMount>) -> AssetSourceBuilder {
    AssetSourceBuilder::new(move || Box::new(OverlayAssetReader::new(mounts.clone())))
}

enum MountedReader {
    FileSystem(FileAssetReader),
    Hexz(HexzAssetReader),
}

impl MountedReader {
    fn new(mount: ContentMount) -> Self {
        match mount.backend() {
            ContentBackend::FileSystem(_) => Self::FileSystem(FileAssetReader::new(
                mount.filesystem_root().expect("filesystem mount root"),
            )),
            ContentBackend::Hexz(archive) => Self::Hexz(HexzAssetReader::new(
                archive.clone(),
                mount.prefix().to_owned(),
            )),
        }
    }

    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        match self {
            Self::FileSystem(reader) => reader
                .read(path)
                .await
                .map(|reader| Box::new(reader) as Box<dyn Reader>),
            Self::Hexz(reader) => reader
                .read(path)
                .await
                .map(|reader| Box::new(reader) as Box<dyn Reader>),
        }
    }

    async fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        match self {
            Self::FileSystem(reader) => reader
                .read_meta(path)
                .await
                .map(|reader| Box::new(reader) as Box<dyn Reader>),
            Self::Hexz(reader) => reader
                .read_meta(path)
                .await
                .map(|reader| Box::new(reader) as Box<dyn Reader>),
        }
    }

    async fn read_directory(&self, path: &Path) -> Result<Box<PathStream>, AssetReaderError> {
        match self {
            Self::FileSystem(reader) => reader.read_directory(path).await,
            Self::Hexz(reader) => reader.read_directory(path).await,
        }
    }

    async fn is_directory(&self, path: &Path) -> Result<bool, AssetReaderError> {
        match self {
            Self::FileSystem(reader) => reader.is_directory(path).await,
            Self::Hexz(reader) => reader.is_directory(path).await,
        }
    }
}

struct OverlayAssetReader {
    readers: Vec<MountedReader>,
}

impl OverlayAssetReader {
    fn new(mounts: Vec<ContentMount>) -> Self {
        Self {
            readers: mounts.into_iter().rev().map(MountedReader::new).collect(),
        }
    }
}

impl AssetReader for OverlayAssetReader {
    fn read<'a>(&'a self, path: &'a Path) -> impl AssetReaderFuture<Value: Reader + 'a> {
        async move {
            for reader in &self.readers {
                match reader.read(path).await {
                    Ok(value) => return Ok(value),
                    Err(AssetReaderError::NotFound(_)) => {}
                    Err(error) => return Err(error),
                }
            }
            Err(AssetReaderError::NotFound(path.to_owned()))
        }
    }

    fn read_meta<'a>(&'a self, path: &'a Path) -> impl AssetReaderFuture<Value: Reader + 'a> {
        async move {
            for reader in &self.readers {
                match reader.read(path).await {
                    Ok(_) => return reader.read_meta(path).await,
                    Err(AssetReaderError::NotFound(_)) => {}
                    Err(error) => return Err(error),
                }
            }
            Err(AssetReaderError::NotFound(path.to_owned()))
        }
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        for reader in &self.readers {
            match reader.read_directory(path).await {
                Ok(value) => return Ok(value),
                Err(AssetReaderError::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Err(AssetReaderError::NotFound(path.to_owned()))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        for reader in &self.readers {
            match reader.is_directory(path).await {
                Ok(true) => return Ok(true),
                Ok(false) | Err(AssetReaderError::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(false)
    }
}

struct HexzAssetReader {
    archive: HexzArchive,
    prefix: PathBuf,
}

impl HexzAssetReader {
    fn new(archive: HexzArchive, prefix: PathBuf) -> Self {
        Self { archive, prefix }
    }

    fn resolve(&self, path: &Path) -> PathBuf {
        self.prefix.join(path)
    }
}

impl AssetReader for HexzAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let resolved = self.resolve(path);
        if !self.archive.contains_file(&resolved) {
            return Err(AssetReaderError::NotFound(path.to_owned()));
        }
        self.archive
            .open_file(&resolved)
            .map(HexzStreamReader::new)
            .map_err(asset_io_error)
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        Err::<HexzStreamReader, _>(AssetReaderError::NotFound(path.to_owned()))
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let resolved = self.resolve(path);
        if !self.archive.is_directory(&resolved) {
            return Err(AssetReaderError::NotFound(path.to_owned()));
        }
        let prefix = self.prefix.clone();
        let entries = self
            .archive
            .read_directory(&resolved)
            .into_iter()
            .filter_map(move |entry| entry.strip_prefix(&prefix).ok().map(Path::to_owned));
        Ok(Box::new(futures_lite::stream::iter(entries)))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        Ok(self.archive.is_directory(&self.resolve(path)))
    }
}

struct HexzStreamReader {
    cursor: HexzCursor,
}

impl HexzStreamReader {
    fn new(file: HexzFile) -> Self {
        Self {
            cursor: file.cursor(),
        }
    }
}

impl AsyncRead for HexzStreamReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Read::read(&mut self.cursor, buffer))
    }
}

impl AsyncSeek for HexzStreamReader {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        position: SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        Poll::Ready(Seek::seek(&mut self.cursor, position))
    }
}

impl Reader for HexzStreamReader {
    fn read_to_end<'a>(
        &'a mut self,
        buffer: &'a mut Vec<u8>,
    ) -> StackFuture<'a, std::io::Result<usize>, STACK_FUTURE_SIZE> {
        StackFuture::from(async move { self.cursor.read_remaining_into(buffer) })
    }

    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        Ok(self)
    }
}

fn asset_io_error(error: anyhow::Error) -> AssetReaderError {
    AssetReaderError::from(Error::other(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use bevy::tasks::block_on;

    use super::*;

    #[test]
    fn later_asset_root_overrides_earlier_root() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("crabgal-overlay-{nonce}"));
        let base = root.join("base");
        let patch = root.join("patch");
        fs::create_dir_all(&base).unwrap();
        fs::create_dir_all(&patch).unwrap();
        fs::write(base.join("shared.txt"), "base").unwrap();
        fs::write(patch.join("shared.txt"), "patch").unwrap();
        let overlay = OverlayAssetReader::new(vec![
            ContentMount::new(ContentBackend::FileSystem(base), "").unwrap(),
            ContentMount::new(ContentBackend::FileSystem(patch), "").unwrap(),
        ]);

        let bytes = block_on(async {
            let mut reader = overlay.read(Path::new("shared.txt")).await.unwrap();
            let mut bytes = Vec::new();
            Reader::read_to_end(&mut reader, &mut bytes).await.unwrap();
            bytes
        });
        assert_eq!(bytes, b"patch");
        let _ = fs::remove_dir_all(root);
    }
}
