use std::io::{Read, Result as IoResult, Seek, SeekFrom};

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
