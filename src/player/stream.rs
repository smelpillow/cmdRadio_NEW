use std::io::{Read, Result as IoResult, Seek, SeekFrom};
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

pub type IcyMetadataHandle = Arc<Mutex<Option<IcyMetadata>>>;
