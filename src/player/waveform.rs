use std::cmp::max;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::Source;

const WAVEFORM_WIDTH: usize = 48;

pub type WaveformHandle = Arc<Mutex<WaveformState>>;

#[derive(Debug)]
pub struct WaveformState {
    bars: Vec<f32>,
    next_index: usize,
}

impl WaveformState {
    pub fn new() -> Self {
        Self {
            bars: vec![0.0; WAVEFORM_WIDTH],
            next_index: 0,
        }
    }

    pub fn push_bucket(&mut self, peak: f32) {
        if self.bars.is_empty() {
            return;
        }

        self.bars[self.next_index] = peak.clamp(0.0, 1.0);
        self.next_index = (self.next_index + 1) % self.bars.len();
    }

    pub fn levels(&self) -> (f32, f32) {
        if self.bars.is_empty() {
            return (0.0, 0.0);
        }

        let last_index = if self.next_index == 0 {
            self.bars.len() - 1
        } else {
            self.next_index - 1
        };

        let peak = self.bars[last_index].clamp(0.0, 1.0);
        let avg = (self.bars.iter().copied().sum::<f32>() / self.bars.len() as f32).clamp(0.0, 1.0);
        (peak, avg)
    }
}

pub struct WaveformSource<S> {
    inner: S,
    waveform: WaveformHandle,
    channels: u16,
    frames_per_bucket: usize,
    channel_sample_index: u16,
    frames_in_bucket: usize,
    bucket_peak: f32,
}

impl<S> WaveformSource<S>
where
    S: Source<Item = f32>,
{
    pub fn new(inner: S, waveform: WaveformHandle) -> Self {
        let channels = inner.channels().max(1);
        let sample_rate = max(inner.sample_rate() as usize, 1);
        let frames_per_bucket = max(sample_rate / WAVEFORM_WIDTH.max(1), 1);

        Self {
            inner,
            waveform,
            channels,
            frames_per_bucket,
            channel_sample_index: 0,
            frames_in_bucket: 0,
            bucket_peak: 0.0,
        }
    }

    fn commit_bucket(&mut self) {
        if let Ok(mut guard) = self.waveform.lock() {
            guard.push_bucket(self.bucket_peak);
        }

        self.bucket_peak = 0.0;
    }
}

impl<S> Iterator for WaveformSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        self.bucket_peak = self.bucket_peak.max(sample.abs().clamp(0.0, 1.0));

        self.channel_sample_index += 1;
        if self.channel_sample_index >= self.channels {
            self.channel_sample_index = 0;
            self.frames_in_bucket += 1;

            if self.frames_in_bucket >= self.frames_per_bucket {
                self.frames_in_bucket = 0;
                self.commit_bucket();
            }
        }

        Some(sample)
    }
}

impl<S> Source for WaveformSource<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}