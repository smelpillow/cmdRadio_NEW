use std::io::{ErrorKind, Read, Result as IoResult, Seek, SeekFrom};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub type PlaybackProgressHandle = Arc<AtomicU64>;

fn mark_progress(progress: &Option<PlaybackProgressHandle>) {
    if let Some(progress) = progress {
        progress.store(current_epoch_secs(), Ordering::Relaxed);
    }
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub struct HttpStream {
    inner: Box<dyn Read + Send + Sync>,
    progress: Option<PlaybackProgressHandle>,
}

pub struct PrefixedReader {
    prefix: std::io::Cursor<Vec<u8>>,
    inner: Box<dyn Read + Send + Sync>,
}

impl PrefixedReader {
    pub fn new(prefix: Vec<u8>, inner: Box<dyn Read + Send + Sync>) -> Self {
        Self {
            prefix: std::io::Cursor::new(prefix),
            inner,
        }
    }
}

impl Read for PrefixedReader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let prefix_bytes = self.prefix.read(buf)?;
        if prefix_bytes > 0 {
            return Ok(prefix_bytes);
        }
        self.inner.read(buf)
    }
}

impl Seek for PrefixedReader {
    fn seek(&mut self, _pos: SeekFrom) -> IoResult<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "HTTP stream is not seekable",
        ))
    }
}

impl HttpStream {
    pub fn new(
        inner: Box<dyn Read + Send + Sync>,
        progress: Option<PlaybackProgressHandle>,
    ) -> Self {
        Self { inner, progress }
    }
}

impl Read for HttpStream {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            mark_progress(&self.progress);
        }
        Ok(n)
    }
}

impl Seek for HttpStream {
    fn seek(&mut self, _pos: SeekFrom) -> IoResult<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "HTTP stream is not seekable",
        ))
    }
}

pub struct IcyStream {
    inner: Box<dyn Read + Send + Sync>,
    metaint: usize,
    bytes_until_metadata: usize,
    metadata: IcyMetadataHandle,
    progress: Option<PlaybackProgressHandle>,
}

impl IcyStream {
    pub fn new(
        inner: Box<dyn Read + Send + Sync>,
        metaint: usize,
        metadata: IcyMetadataHandle,
        progress: Option<PlaybackProgressHandle>,
    ) -> Self {
        Self {
            inner,
            metaint,
            bytes_until_metadata: metaint,
            metadata,
            progress,
        }
    }

    fn consume_metadata_block(&mut self) -> IoResult<()> {
        let mut length_byte = [0_u8; 1];
        self.inner.read_exact(&mut length_byte)?;
        let metadata_len = usize::from(length_byte[0]) * 16;

        if metadata_len == 0 {
            return Ok(());
        }

        let mut block = vec![0_u8; metadata_len];
        self.inner.read_exact(&mut block)?;

        let parsed = parse_icy_metadata(&block);

        if let Ok(mut guard) = self.metadata.lock() {
            *guard = parsed;
        }

        Ok(())
    }
}

impl Read for IcyStream {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut written = 0_usize;

        while written < buf.len() {
            if self.bytes_until_metadata == 0 {
                match self.consume_metadata_block() {
                    Ok(()) => {
                        self.bytes_until_metadata = self.metaint;
                    }
                    Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                        return Ok(written);
                    }
                    Err(err) => return Err(err),
                }
                continue;
            }

            let chunk = (buf.len() - written).min(self.bytes_until_metadata);
            let n = self.inner.read(&mut buf[written..written + chunk])?;
            if n == 0 {
                return Ok(written);
            }

            mark_progress(&self.progress);

            written += n;
            self.bytes_until_metadata -= n;
        }

        Ok(written)
    }
}

impl Seek for IcyStream {
    fn seek(&mut self, _pos: SeekFrom) -> IoResult<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "HTTP stream is not seekable",
        ))
    }
}

pub enum RadioStream {
    Http(HttpStream),
    Icy(IcyStream),
}

impl Read for RadioStream {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        match self {
            Self::Http(stream) => stream.read(buf),
            Self::Icy(stream) => stream.read(buf),
        }
    }
}

impl Seek for RadioStream {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        match self {
            Self::Http(stream) => stream.seek(pos),
            Self::Icy(stream) => stream.seek(pos),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcyMetadata {
    pub artist: Option<String>,
    pub title: Option<String>,
}

impl IcyMetadata {
    #[allow(dead_code)]
    pub fn from_stream_title(stream_title: &str) -> Self {
        if let Some((artist, title)) = stream_title.split_once(" - ") {
            Self {
                artist: Some(artist.to_string()),
                title: Some(title.to_string()),
            }
        } else {
            Self {
                artist: None,
                title: Some(stream_title.to_string()),
            }
        }
    }
}

fn parse_icy_metadata(block: &[u8]) -> Option<IcyMetadata> {
    let text = String::from_utf8_lossy(block);
    let cleaned = text.trim_end_matches('\0');
    let key = "StreamTitle='";
    let start = cleaned.find(key)? + key.len();
    let tail = &cleaned[start..];

    let end = tail.find("';").or_else(|| tail.find('\''))?;
    let stream_title = tail[..end].trim();
    if stream_title.is_empty() {
        return None;
    }

    Some(IcyMetadata::from_stream_title(stream_title))
}

pub type IcyMetadataHandle = Arc<Mutex<Option<IcyMetadata>>>;

#[cfg(test)]
mod tests {
    use super::{IcyMetadata, IcyStream, PrefixedReader, parse_icy_metadata};
    use std::io::{Cursor, Read, Result as IoResult};
    use std::sync::{Arc, Mutex};

    struct ChunkedReader {
        inner: Cursor<Vec<u8>>,
        chunk_size: usize,
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            let limit = buf.len().min(self.chunk_size);
            let limited = &mut buf[..limit];
            self.inner.read(limited)
        }
    }

    #[test]
    fn prefixed_reader_replays_prefix_before_inner_data() {
        let mut reader =
            PrefixedReader::new(b"abc".to_vec(), Box::new(Cursor::new(b"def".to_vec())));
        let mut out = [0_u8; 5];

        let first = reader.read(&mut out).unwrap();
        assert_eq!(first, 3);
        assert_eq!(&out[..first], b"abc");

        let second = reader.read(&mut out).unwrap();
        assert_eq!(second, 3);
        assert_eq!(&out[..second], b"def");
    }

    #[test]
    fn parse_icy_metadata_handles_valid_titles_and_ignores_empty_titles() {
        let valid = b"StreamTitle='Artist - Song';StreamUrl='https://example.com';";
        assert_eq!(
            parse_icy_metadata(valid),
            Some(IcyMetadata {
                artist: Some(String::from("Artist")),
                title: Some(String::from("Song")),
            })
        );

        let empty = b"StreamTitle='';StreamUrl='https://example.com';";
        assert!(parse_icy_metadata(empty).is_none());
    }

    #[test]
    fn icy_stream_strips_metadata_from_audio_bytes() {
        let title = "Artist - Song";
        let metadata = format!("StreamTitle='{}';", title);
        let block_len = metadata.len().div_ceil(16) as u8;
        let mut payload = Vec::new();
        payload.extend_from_slice(b"AAAA");
        payload.push(block_len);
        payload.extend_from_slice(metadata.as_bytes());
        payload.resize(
            payload.len() + (block_len as usize * 16 - metadata.len()),
            0,
        );
        payload.extend_from_slice(b"BBBB");

        let handle = Arc::new(Mutex::new(None));
        let mut stream =
            IcyStream::new(Box::new(Cursor::new(payload)), 4, Arc::clone(&handle), None);

        let mut buffer = [0_u8; 8];
        let n = stream.read(&mut buffer).unwrap();

        assert_eq!(&buffer[..n], b"AAAABBBB");

        let locked = handle.lock().unwrap();
        assert_eq!(
            locked.as_ref(),
            Some(&IcyMetadata {
                artist: Some(String::from("Artist")),
                title: Some(String::from("Song")),
            })
        );
    }

    #[test]
    fn icy_stream_handles_fragmented_audio_and_metadata_reads() {
        let metadata = b"StreamTitle='Artist - Fragmented';";
        let block_len = metadata.len().div_ceil(16) as u8;
        let mut payload = Vec::new();
        payload.extend_from_slice(b"ABCD");
        payload.push(block_len);
        payload.extend_from_slice(metadata);
        payload.resize(
            payload.len() + (block_len as usize * 16 - metadata.len()),
            0,
        );
        payload.extend_from_slice(b"EFGH");

        let handle = Arc::new(Mutex::new(None));
        let reader = ChunkedReader {
            inner: Cursor::new(payload),
            chunk_size: 2,
        };
        let mut stream = IcyStream::new(Box::new(reader), 4, Arc::clone(&handle), None);

        let mut buffer = [0_u8; 8];
        let n = stream.read(&mut buffer).unwrap();

        assert_eq!(&buffer[..n], b"ABCDEFGH");
        assert_eq!(
            handle.lock().unwrap().as_ref(),
            Some(&IcyMetadata {
                artist: Some(String::from("Artist")),
                title: Some(String::from("Fragmented")),
            })
        );
    }
}
