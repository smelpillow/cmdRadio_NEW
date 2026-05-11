use std::io::BufReader;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

use crate::player::stream::HttpStream;

pub struct RadioPlayer {
    output_stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    paused: bool,
}

impl RadioPlayer {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            output_stream: None,
            stream_handle: None,
            sink: None,
            paused: false,
        })
    }

    pub fn play_from_url(&mut self, url: &str) -> Result<(), String> {
        self.stop();
        self.ensure_output()?;

        let response = ureq::get(url)
            .call()
            .map_err(|e| format!("http request failed: {e}"))?;

        let reader = response.into_reader();
        let stream = HttpStream::new(Box::new(reader));
        let decoder = Decoder::new(BufReader::new(stream))
            .map_err(|e| format!("decoder error. stream may require unsupported codec: {e}"))?;

        let handle = self
            .stream_handle
            .as_ref()
            .ok_or_else(|| String::from("audio stream handle not available"))?;

        let sink = Sink::try_new(handle).map_err(|e| format!("failed to create sink: {e}"))?;
        sink.append(decoder);
        sink.play();

        self.sink = Some(sink);
        self.paused = false;
        Ok(())
    }

    pub fn toggle_pause(&mut self) -> Result<bool, String> {
        let sink = self
            .sink
            .as_ref()
            .ok_or_else(|| String::from("nothing is playing"))?;

        if self.paused {
            sink.play();
            self.paused = false;
        } else {
            sink.pause();
            self.paused = true;
        }
        Ok(self.paused)
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.paused = false;
    }

    fn ensure_output(&mut self) -> Result<(), String> {
        if self.output_stream.is_none() || self.stream_handle.is_none() {
            let (stream, handle) = OutputStream::try_default()
                .map_err(|e| format!("audio output init failed: {e}"))?;
            self.output_stream = Some(stream);
            self.stream_handle = Some(handle);
        }
        Ok(())
    }
}
