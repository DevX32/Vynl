use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use rodio::Source;
use rodio::source::SeekError;

#[derive(Clone)]
pub(super) struct FadeControl(Arc<FadeOrder>);
pub(super) struct FadeOrder {
    to: AtomicU32,
    ramp: AtomicU32,
    serial: AtomicU32,
    live: AtomicU32,
}

impl FadeControl {
    pub(super) fn settled(gain: f32) -> Self {
        let gain = gain.clamp(0.0, 1.0);
        Self(Arc::new(FadeOrder {
            to: AtomicU32::new(gain.to_bits()),
            ramp: AtomicU32::new(0),
            serial: AtomicU32::new(0),
            live: AtomicU32::new(gain.to_bits()),
        }))
    }

    pub(super) fn ramp_to(&self, to: f32, secs: f64, sample_rate: u32, channels: u16) {
        let samples = if secs.is_finite() && secs > 0.0 && sample_rate > 0 {
            (secs * sample_rate as f64 * channels.max(1) as f64)
                .round()
                .min(u32::MAX as f64) as u32
        } else {
            0
        };
        self.0
            .to
            .store(to.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        self.0.ramp.store(samples, Ordering::Relaxed);
        self.0.serial.fetch_add(1, Ordering::Release);
    }

    pub(super) fn take_order(&self, since: Option<u32>) -> Option<(u32, f32, u32)> {
        for _ in 0..4 {
            let serial = self.0.serial.load(Ordering::Acquire);
            if Some(serial) == since {
                return None;
            }
            let to = f32::from_bits(self.0.to.load(Ordering::Relaxed));
            let ramp = self.0.ramp.load(Ordering::Relaxed);
            if self.0.serial.load(Ordering::Acquire) == serial {
                return Some((serial, to, ramp));
            }
        }
        None
    }

    pub(super) fn current(&self) -> f32 {
        f32::from_bits(self.0.live.load(Ordering::Relaxed))
    }

    pub(super) fn publish(&self, gain: f32) {
        self.0
            .live
            .store(gain.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
}

pub(super) struct FadeSource<S> {
    inner: S,
    control: FadeControl,
    gain: f32,
    to: f32,
    step: f32,
    left: u32,
    serial: Option<u32>,
}

impl<S> FadeSource<S>
where
    S: Source<Item = f32>,
{
    pub(super) fn new(inner: S, control: FadeControl) -> Self {
        let gain = control.current();
        Self {
            inner,
            control,
            serial: None,
            gain,
            to: gain,
            step: 0.0,
            left: 0,
        }
    }

    pub(super) fn retarget(&mut self) {
        let Some((serial, to, ramp)) = self.control.take_order(self.serial) else {
            return;
        };
        self.serial = Some(serial);
        self.to = to;
        self.left = ramp;
        self.step = if ramp > 0 {
            (to - self.gain) / ramp as f32
        } else {
            0.0
        };
    }

    #[inline]
    pub(super) fn glide(&mut self, sample: f32) -> f32 {
        if self.left > 0 {
            self.gain += self.step;
            self.left -= 1;
            if self.left == 0 {
                self.gain = self.to;
                self.control.publish(self.gain);
            }
        } else if self.gain != self.to {
            self.gain = self.to;
            self.control.publish(self.gain);
        }
        sample * self.gain
    }
}

impl<S> Iterator for FadeSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.retarget();
        let sample = self.inner.next()?;
        Some(self.glide(sample))
    }
}

impl<S> Source for FadeSource<S>
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

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(pos)
    }
}
