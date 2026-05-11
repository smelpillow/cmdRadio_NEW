use std::io::BufReader;
use std::sync::{Arc, Mutex};

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

use crate::player::stream::{HttpStream, IcyMetadata, IcyMetadataHandle};

pub struct RadioPlayer {
    output_stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    paused: bool,
    volume: f32,
    icy_metadata: Option<IcyMetadataHandle>,
}

impl RadioPlayer {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            output_stream: None,
            stream_handle: None,
            sink: None,
            paused: false,
            volume: 1.0,
            icy_metadata: None,
        })
    }

    pub fn play_from_url(&mut self, url: &str) -> Result<(), String> {
        self.stop();
        self.ensure_output()?;

        let response = ureq::get(url)
            .set("Icy-MetaData", "1")
            .call()
            .map_err(|e| format!("http request failed: {e}"))?;

        let _icy_metaint = response
            .header("icy-metaint")
            .and_then(|v| v.parse::<usize>().ok());

        self.icy_metadata = Some(Arc::new(Mutex::new(None)));

        let reader = response.into_reader();
        let stream = HttpStream::new(Box::new(reader));
        let decoder = Decoder::new(BufReader::new(stream))
            .map_err(|e| format!("decoder error. stream may require unsupported codec: {e}"))?;

        let handle = self
            .stream_handle
            .as_ref()
            .ok_or_else(|| String::from("audio stream handle not available"))?;

        let sink = Sink::try_new(handle).map_err(|e| format!("failed to create sink: {e}"))?;
        sink.set_volume(self.volume);
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
        self.icy_metadata = None;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn has_active_stream(&self) -> bool {
        self.sink.is_some()
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn adjust_volume(&mut self, delta: f32) -> f32 {
        self.volume = (self.volume + delta).clamp(0.0, 2.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
        self.volume
    }

    pub fn current_metadata(&self) -> Option<IcyMetadata> {
        self.icy_metadata
            .as_ref()
            .and_then(|h| h.lock().ok())
            .and_then(|m| m.clone())
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
