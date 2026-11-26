use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use super::{SessionControl, StreamSession};

const MAX_BUFFER_SECS: f64 = 6.0;
pub(super) const DEFAULT_RATE: u32 = 48_000;
pub(super) const DEFAULT_CHANNELS: u16 = 2;

struct BufferState {
    queue: VecDeque<f32>,
    capacity: usize,
    closed: bool,
    error: Option<String>,
    format: Option<(u32, u16)>,
}

pub(super) struct StreamBuffer {
    state: Mutex<BufferState>,
    cv: Condvar,
}

impl StreamBuffer {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(BufferState {
                queue: VecDeque::new(),
                capacity: (MAX_BUFFER_SECS * DEFAULT_RATE as f64 * DEFAULT_CHANNELS as f64)
                    as usize,
                closed: false,
                error: None,
                format: None,
            }),
            cv: Condvar::new(),
        }
    }

    pub(super) fn set_format(&self, rate: u32, channels: u16) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.format.is_none() {
            state.format = Some((rate.max(1), channels.max(1)));
            state.capacity = (MAX_BUFFER_SECS * rate as f64 * channels as f64).max(4096.0) as usize;
        }
        self.cv.notify_all();
    }

    pub(super) fn push(&self, samples: &[f32], control: &SessionControl) -> bool {
        if samples.is_empty() {
            return true;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while state.queue.len() + samples.len() > state.capacity {
            if control.is_cancelled() || state.closed {
                return false;
            }
            let (guard, _) = self
                .cv
                .wait_timeout(state, Duration::from_millis(150))
                .unwrap_or_else(|e| e.into_inner());
            state = guard;
        }
        if control.is_cancelled() || state.closed {
            return false;
        }
        state.queue.extend(samples.iter().copied());
        self.cv.notify_all();
        true
    }

    pub(super) fn close(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.closed = true;
        self.cv.notify_all();
    }

    pub(super) fn fail(&self, message: String) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.error.is_none() {
            state.error = Some(message);
        }
        state.closed = true;
        self.cv.notify_all();
    }

    fn pop(&self) -> Pop {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(sample) = state.queue.pop_front() {
            Pop::Sample(sample)
        } else if state.closed {
            Pop::Closed
        } else {
            Pop::Underrun
        }
    }

    pub(super) fn buffered_secs(&self) -> f64 {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let (rate, channels) = state.format.unwrap_or((DEFAULT_RATE, DEFAULT_CHANNELS));
        state.queue.len() as f64 / (rate as f64 * channels as f64)
    }

    pub(super) fn format(&self) -> Option<(u32, u16)> {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).format
    }

    pub(super) fn error(&self) -> Option<String> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .error
            .clone()
    }

    pub(super) fn is_closed(&self) -> bool {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).closed
    }
}

enum Pop {
    Sample(f32),
    Underrun,
    Closed,
}

pub struct StreamSource {
    buffer: Arc<StreamBuffer>,
    channels: u16,
    sample_rate: u32,
    duration: Option<Duration>,
    underrun_ms: Arc<std::sync::atomic::AtomicU64>,
}

impl StreamSource {
    pub(crate) fn new(session: &StreamSession) -> Self {
        let (rate, channels) = session
            .buffer
            .format()
            .unwrap_or((DEFAULT_RATE, DEFAULT_CHANNELS));
        Self {
            buffer: session.buffer.clone(),
            channels,
            sample_rate: rate,
            duration: session
                .duration
                .filter(|d| d.is_finite() && *d > 0.0)
                .and_then(|d| Duration::try_from_secs_f64(d).ok()),
            underrun_ms: session.underrun_ms.clone(),
        }
    }
}

impl Iterator for StreamSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        match self.buffer.pop() {
            Pop::Sample(sample) => Some(sample),
            Pop::Underrun => {
                self.underrun_ms.store(
                    crate::services::util::now_ms().max(0) as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
                Some(0.0)
            }
            Pop::Closed => None,
        }
    }
}

impl rodio::Source for StreamSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.duration
    }

    fn try_seek(&mut self, _pos: Duration) -> Result<(), rodio::source::SeekError> {
        Err(rodio::source::SeekError::NotSupported {
            underlying_source: "Vynl stream",
        })
    }
}
