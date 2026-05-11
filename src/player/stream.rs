use std::io::{ErrorKind, Read, Result as IoResult, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

pub struct HttpStream {
    inner: Box<dyn Read + Send + Sync>,
}

impl HttpStream {
    pub fn new(inner: Box<dyn Read + Send + Sync>) -> Self {
        Self { inner }
    }
}

impl Read for HttpStream {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.inner.read(buf)
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
}

impl IcyStream {
    pub fn new(
        inner: Box<dyn Read + Send + Sync>,
        metaint: usize,
        metadata: IcyMetadataHandle,
    ) -> Self {
        Self {
            inner,
            metaint,
            bytes_until_metadata: metaint,
            metadata,
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

        if let Ok(mut guard) = self.metadata.lock() {
            *guard = parse_icy_metadata(&block);
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

#[derive(Debug, Clone)]
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
