use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use rodio::Source;
use rodio::source::SeekError;
use serde::Serialize;

pub const EQ_BAND_COUNT: usize = 10;

pub const EQ_FREQS: [f32; EQ_BAND_COUNT] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub const EQ_MIN_DB: f32 = -12.0;
pub const EQ_MAX_DB: f32 = 12.0;

const EQ_QS: [f32; EQ_BAND_COUNT] = [0.7, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.4, 1.2, 1.0];
const EQ_LOW_SHELF: usize = 0;
const EQ_HIGH_SHELF: usize = EQ_BAND_COUNT - 1;
const EQ_SHAPE: f32 = 1.7;

const EQ_FLAT_DB: f32 = 0.05;
const EQ_RAMP_SECS: f32 = 0.04;
const EQ_LIMIT_KNEE: f32 = 0.9;
const EQ_LIMIT_CEIL: f32 = 0.999;
const EQ_PREAMP_TRIM: f32 = 0.5;

#[derive(Clone, Copy)]
struct EqConfig {
    enabled: bool,
    gains: [f32; EQ_BAND_COUNT],
}

impl Default for EqConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            gains: [0.0; EQ_BAND_COUNT],
        }
    }
}

static EQ_CONFIG: once_cell::sync::Lazy<Mutex<EqConfig>> =
    once_cell::sync::Lazy::new(|| Mutex::new(EqConfig::default()));

static EQ_GEN: AtomicU64 = AtomicU64::new(0);

fn eq_config() -> MutexGuard<'static, EqConfig> {
    EQ_CONFIG.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn set_eq(enabled: bool, gains: &[f32]) {
    let normalized = normalize_eq_bands(gains);
    {
        let mut cfg = eq_config();
        cfg.enabled = enabled;
        cfg.gains.copy_from_slice(&normalized);
    }
    EQ_GEN.fetch_add(1, Ordering::Release);
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EqDesign {
    pub freqs: [f32; EQ_BAND_COUNT],
    pub qs: [f32; EQ_BAND_COUNT],
    pub low_shelf: usize,
    pub high_shelf: usize,
    pub shape: f32,
    pub flat_db: f32,
    pub preamp_trim: f32,
    pub min_db: f32,
    pub max_db: f32,
}

pub fn eq_design() -> EqDesign {
    EqDesign {
        freqs: EQ_FREQS,
        qs: EQ_QS,
        low_shelf: EQ_LOW_SHELF,
        high_shelf: EQ_HIGH_SHELF,
        shape: EQ_SHAPE,
        flat_db: EQ_FLAT_DB,
        preamp_trim: EQ_PREAMP_TRIM,
        min_db: EQ_MIN_DB,
        max_db: EQ_MAX_DB,
    }
}

pub fn normalize_eq_bands(gains: &[f32]) -> Vec<f32> {
    (0..EQ_BAND_COUNT)
        .map(|i| {
            let raw = gains.get(i).copied().unwrap_or(0.0);
            if raw.is_finite() {
                raw.clamp(EQ_MIN_DB, EQ_MAX_DB)
            } else {
                0.0
            }
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Coeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Coeffs {
    const FLAT: Self = Self {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    fn peaking(w0: f32, q: f32, gain_db: f32) -> [f32; 6] {
        let a = 10f32.powf(gain_db / 40.0);
        let cos = w0.cos();
        let alpha = w0.sin() / (2.0 * q);
        [
            1.0 + alpha * a,
            -2.0 * cos,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cos,
            1.0 - alpha / a,
        ]
    }

    fn shelf(w0: f32, shape: f32, gain_db: f32, high: bool) -> [f32; 6] {
        let amp = 10f64.powf(f64::from(gain_db) / 20.0);
        let m = f64::from(shape);
        let k = (f64::from(w0) * 0.5).tan();
        let k2 = k * k;
        let mid = m * amp.sqrt() * k;
        let (b0, b1, b2) = if high {
            (amp + mid + k2, 2.0 * (k2 - amp), amp - mid + k2)
        } else {
            (
                1.0 + mid + amp * k2,
                2.0 * (amp * k2 - 1.0),
                1.0 - mid + amp * k2,
            )
        };
        let (a0, a1, a2) = (1.0 + m * k + k2, 2.0 * (k2 - 1.0), 1.0 - m * k + k2);
        let inv = (1.0 / k2).min(f32::MAX as f64);
        [b0, b1, b2, a0, a1, a2].map(|v| (v * inv) as f32)
    }

    fn design(band: usize, gain_db: f32, sample_rate: f32) -> Self {
        let freq = EQ_FREQS[band];
        if gain_db.abs() < EQ_FLAT_DB || sample_rate <= 0.0 || freq >= sample_rate * 0.49 {
            return Self::FLAT;
        }

        let w0 =
            (2.0 * std::f32::consts::PI * freq / sample_rate).min(std::f32::consts::PI * 0.999);
        let high = band == EQ_HIGH_SHELF;
        let [b0, b1, b2, a0, a1, a2] = if matches!(band, EQ_LOW_SHELF | EQ_HIGH_SHELF) {
            Self::shelf(w0, EQ_SHAPE, gain_db, high)
        } else {
            Self::peaking(w0, EQ_QS[band], gain_db)
        };

        let coeffs = Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        };
        if [coeffs.b0, coeffs.b1, coeffs.b2, coeffs.a1, coeffs.a2]
            .iter()
            .any(|v| !v.is_finite())
            || coeffs.a1.abs() > 2.0
            || coeffs.a2.abs() >= 1.0
        {
            return Self::FLAT;
        }
        coeffs
    }

    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            b0: self.b0 + (other.b0 - self.b0) * t,
            b1: self.b1 + (other.b1 - self.b1) * t,
            b2: self.b2 + (other.b2 - self.b2) * t,
            a1: self.a1 + (other.a1 - self.a1) * t,
            a2: self.a2 + (other.a2 - self.a2) * t,
        }
    }
}

struct EqBand {
    cur: Coeffs,
    target: Coeffs,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl EqBand {
    fn settled(band: usize, gains: &[f32; EQ_BAND_COUNT], sample_rate: f32) -> Self {
        let coeffs = Coeffs::design(band, gains[band], sample_rate);
        Self {
            cur: coeffs,
            target: coeffs,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn aim(&mut self, band: usize, gains: &[f32; EQ_BAND_COUNT], sample_rate: f32) {
        self.target = Coeffs::design(band, gains[band], sample_rate);
    }

    #[inline]
    fn glide(&mut self, step: f32) {
        self.cur = self.cur.lerp(self.target, step);
    }

    #[inline]
    fn settle(&mut self) {
        self.cur = self.target;
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.cur.b0 * x + self.cur.b1 * self.x1 + self.cur.b2 * self.x2
            - self.cur.a1 * self.y1
            - self.cur.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

fn preamp_gain(gains: &[f32; EQ_BAND_COUNT]) -> f32 {
    let peak = gains
        .iter()
        .copied()
        .filter(|g| *g > 0.0)
        .fold(0.0f32, f32::max);
    if peak <= 0.0 {
        return 1.0;
    }
    10f32.powf(-(peak * EQ_PREAMP_TRIM) / 20.0)
}

#[inline]
fn soft_limit(x: f32) -> f32 {
    let mag = x.abs();
    if mag <= EQ_LIMIT_KNEE {
        return x;
    }
    let head = EQ_LIMIT_CEIL - EQ_LIMIT_KNEE;
    let shaped = EQ_LIMIT_KNEE + head * (1.0 - (-(mag - EQ_LIMIT_KNEE) / head).exp());
    x.signum() * shaped.min(EQ_LIMIT_CEIL)
}

pub(crate) struct EqualizerSource<S> {
    inner: S,
    bands: Vec<[EqBand; EQ_BAND_COUNT]>,
    gains: [f32; EQ_BAND_COUNT],
    generation: u64,
    channels: usize,
    sample_rate: u32,
    frame: u64,
    ramp: u32,
    wet: f32,
    wet_target: f32,
    preamp: f32,
    preamp_target: f32,
}

impl<S> EqualizerSource<S>
where
    S: Source<Item = f32>,
{
    pub(crate) fn new(inner: S) -> Self {
        let sample_rate = inner.sample_rate().max(1);
        let channels = inner.channels().max(1) as usize;
        let cfg = *eq_config();
        let preamp = preamp_gain(&cfg.gains);
        let wet = if cfg.enabled { 1.0 } else { 0.0 };
        let mut source = Self {
            inner,
            bands: Vec::new(),
            gains: cfg.gains,
            generation: EQ_GEN.load(Ordering::Acquire),
            channels,
            sample_rate,
            frame: 0,
            ramp: 0,
            wet,
            wet_target: wet,
            preamp,
            preamp_target: preamp,
        };
        source.build();
        source
    }

    fn build(&mut self) {
        let sample_rate = self.sample_rate as f32;
        self.bands = (0..self.channels)
            .map(|_| std::array::from_fn(|band| EqBand::settled(band, &self.gains, sample_rate)))
            .collect();
    }

    #[inline]
    fn refresh(&mut self) {
        if EQ_GEN.load(Ordering::Acquire) == self.generation {
            return;
        }
        self.generation = EQ_GEN.load(Ordering::Acquire);
        let cfg = *eq_config();
        let sample_rate = self.inner.sample_rate().max(1);
        let channels = self.inner.channels().max(1) as usize;

        self.wet_target = if cfg.enabled { 1.0 } else { 0.0 };
        self.preamp_target = preamp_gain(&cfg.gains);

        if sample_rate != self.sample_rate || channels != self.channels {
            self.sample_rate = sample_rate;
            self.channels = channels;
            self.gains = cfg.gains;
            self.wet = self.wet_target;
            self.preamp = self.preamp_target;
            self.build();
            self.ramp = 0;
            return;
        }

        self.gains = cfg.gains;
        let rate = sample_rate as f32;
        for chain in self.bands.iter_mut() {
            for (band, eq) in chain.iter_mut().enumerate() {
                eq.aim(band, &self.gains, rate);
            }
        }
        self.ramp = (rate * EQ_RAMP_SECS * channels as f32).round().max(1.0) as u32;
    }
}

impl<S> Iterator for EqualizerSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        self.refresh();

        let channel = (self.frame % self.channels as u64) as usize;
        self.frame = self.frame.wrapping_add(1);

        if self.ramp == 0 && self.wet == 0.0 {
            return Some(sample);
        }
        let Some(chain) = self.bands.get_mut(channel) else {
            return Some(sample);
        };

        if self.ramp > 0 {
            let step = 1.0 / self.ramp as f32;
            self.wet += (self.wet_target - self.wet) * step;
            self.preamp += (self.preamp_target - self.preamp) * step;
            for band in chain.iter_mut() {
                band.glide(step);
            }
            self.ramp -= 1;
            if self.ramp == 0 {
                self.wet = self.wet_target;
                self.preamp = self.preamp_target;
                for band in chain.iter_mut() {
                    band.settle();
                }
            }
        }

        let mut filtered = sample * self.preamp;
        for band in chain.iter_mut() {
            filtered = band.process(filtered);
        }

        Some(soft_limit(sample + self.wet * (filtered - sample)))
    }
}

impl<S> Source for EqualizerSource<S>
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
