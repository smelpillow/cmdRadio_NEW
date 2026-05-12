use std::io::BufReader;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

use crate::player::stream::{
    HttpStream, IcyMetadata, IcyMetadataHandle, IcyStream, PlaybackProgressHandle, RadioStream,
};

pub struct RadioPlayer {
    output_stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    paused: bool,
    volume: f32,
    icy_metadata: Option<IcyMetadataHandle>,
    playback_progress: Option<PlaybackProgressHandle>,
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
            playback_progress: None,
        })
    }

    pub fn play_from_url(&mut self, url: &str, timeout: Duration) -> Result<(), String> {
        self.stop();
        self.ensure_output()?;

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(timeout)
            .timeout_read(timeout)
            .timeout_write(timeout)
            .build();

        let response = agent
            .get(url)
            .set("Icy-MetaData", "1")
            .call()
            .map_err(|e| format!("http request failed: {e}"))?;

        let icy_metaint = response
            .header("icy-metaint")
            .and_then(|v| v.trim().parse::<usize>().ok());

        let metadata_handle = Arc::new(Mutex::new(None));
        let playback_progress = Arc::new(AtomicU64::new(current_epoch_secs()));
        let reader = response.into_reader();

        let stream = if let Some(metaint) = icy_metaint.filter(|v| *v > 0) {
            self.icy_metadata = Some(Arc::clone(&metadata_handle));
            RadioStream::Icy(IcyStream::new(
                Box::new(reader),
                metaint,
                metadata_handle,
                Some(Arc::clone(&playback_progress)),
            ))
        } else {
            self.icy_metadata = Some(metadata_handle);
            RadioStream::Http(HttpStream::new(
                Box::new(reader),
                Some(Arc::clone(&playback_progress)),
            ))
        };

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
        self.playback_progress = Some(playback_progress);
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
        self.playback_progress = None;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn has_active_stream(&self) -> bool {
        self.sink.is_some()
    }

    pub fn is_stream_ended(&self) -> bool {
        self.sink.as_ref().map(|sink| sink.empty()).unwrap_or(false)
    }

    pub fn last_audio_progress_epoch_secs(&self) -> Option<u64> {
        self.playback_progress
            .as_ref()
            .map(|p| p.load(Ordering::Relaxed))
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn adjust_volume(&mut self, delta: f32) -> f32 {
        self.volume = (self.volume + delta).clamp(0.0, 1.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
        self.volume
    }

    pub fn current_metadata(&self) -> Option<IcyMetadata> {
        self.icy_metadata
            .as_ref()
            .map(|h| h.lock().unwrap_or_else(|e| e.into_inner()))
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

pub fn current_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
