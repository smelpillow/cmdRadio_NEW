use std::io::BufReader;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::logger;
use crate::player::stream::{
    HttpStream, IcyMetadata, IcyMetadataHandle, IcyStream, PlaybackProgressHandle, RadioStream,
};
use crate::player::waveform::{WaveformHandle, WaveformSource, WaveformState};

pub struct RadioPlayer {
    output_stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    bound_output_device_name: Option<String>,
    sink: Option<Sink>,
    paused: bool,
    volume: f32,
    stream_bitrate_kbps: Option<u32>,
    stream_content_type: Option<String>,
    waveform: Option<WaveformHandle>,
    icy_metadata: Option<IcyMetadataHandle>,
    playback_progress: Option<PlaybackProgressHandle>,
}

impl RadioPlayer {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            output_stream: None,
            stream_handle: None,
            bound_output_device_name: None,
            sink: None,
            paused: false,
            volume: 1.0,
            stream_bitrate_kbps: None,
            stream_content_type: None,
            waveform: None,
            icy_metadata: None,
            playback_progress: None,
        })
    }

    pub fn play_from_url(&mut self, url: &str, timeout: Duration) -> Result<(), String> {
        logger::info(&format!(
            "play_from_url requested: url={} timeout_secs={}",
            url,
            timeout.as_secs().max(1)
        ));
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
        self.stream_bitrate_kbps = response
            .header("icy-br")
            .and_then(|v| v.trim().parse::<u32>().ok());
        self.stream_content_type = response.header("content-type").map(|v| v.trim().to_string());

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

        let waveform = Arc::new(Mutex::new(WaveformState::new()));
        let waveform_source = WaveformSource::new(decoder.convert_samples::<f32>(), Arc::clone(&waveform));

        let handle = self
            .stream_handle
            .as_ref()
            .ok_or_else(|| String::from("audio stream handle not available"))?;

        let sink = Sink::try_new(handle).map_err(|e| format!("failed to create sink: {e}"))?;
        sink.set_volume(self.volume);
        sink.append(waveform_source);
        sink.play();

        self.sink = Some(sink);
        self.paused = false;
        self.playback_progress = Some(playback_progress);
        self.waveform = Some(waveform);
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
        logger::info("playback stopped");
        self.paused = false;
        self.icy_metadata = None;
        self.playback_progress = None;
        self.stream_bitrate_kbps = None;
        self.stream_content_type = None;
        self.waveform = None;
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

    pub fn set_volume(&mut self, volume: f32) -> f32 {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
        self.volume
    }

    pub fn default_output_device_changed(&self) -> bool {
        let Some(bound_name) = self.bound_output_device_name.as_ref() else {
            return false;
        };

        let Some(current_name) = current_default_output_device_name() else {
            return false;
        };

        current_name != *bound_name
    }

    pub fn invalidate_output_device(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        logger::warn("invalidating audio output device and sink for rebind");
        self.output_stream = None;
        self.stream_handle = None;
        self.bound_output_device_name = None;
        self.paused = false;
    }

    pub fn stream_bitrate_kbps(&self) -> Option<u32> {
        self.stream_bitrate_kbps
    }

    pub fn stream_content_type(&self) -> Option<&str> {
        self.stream_content_type.as_deref()
    }

    pub fn waveform_levels(&self) -> (f32, f32) {
        self.waveform
            .as_ref()
            .and_then(|handle| handle.lock().ok().map(|state| state.levels()))
            .unwrap_or((0.0, 0.0))
    }

    pub fn current_metadata(&self) -> Option<IcyMetadata> {
        let handle = self.icy_metadata.as_ref()?;
        let guard = handle.try_lock().ok()?;
        guard.clone()
    }

    fn ensure_output(&mut self) -> Result<(), String> {
        let current_default_name = current_default_output_device_name();
        let has_output = self.output_stream.is_some() && self.stream_handle.is_some();
        let device_changed = match (
            self.bound_output_device_name.as_deref(),
            current_default_name.as_deref(),
        ) {
            (Some(bound), Some(current)) => bound != current,
            _ => false,
        };

        if !has_output || device_changed {
            if device_changed {
                logger::warn("default output device change detected in ensure_output");
            }
            self.output_stream = None;
            self.stream_handle = None;
            let (stream, handle) = OutputStream::try_default()
                .map_err(|e| format!("audio output init failed: {e}"))?;
            self.output_stream = Some(stream);
            self.stream_handle = Some(handle);
            self.bound_output_device_name = current_default_output_device_name();
            if let Some(name) = self.bound_output_device_name.as_deref() {
                logger::info(&format!("audio output bound to default device: {}", name));
            } else {
                logger::warn("audio output bound, but default device name unavailable");
            }
        }
        Ok(())
    }
}

fn current_default_output_device_name() -> Option<String> {
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    device.name().ok()
}

pub fn current_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
