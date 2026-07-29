use std::collections::VecDeque;
use std::time::Duration;

use rodio::Source;

/// Wraps a decoded source and renders it to a stereo pair with a
/// constant-power stereo pan (from `azimuth`) and an interaural-time-
/// difference delay line (from `itd_ms`), so `HrtfParams`' directional
/// fields actually reach the speakers instead of being computed and
/// discarded in favor of a single overall volume.
///
/// The inner source is downmixed to mono per output frame (regardless of
/// its original channel count) before panning: HRTF positions a single
/// point source in space, so a pre-existing stereo image in the source
/// data isn't meaningful to preserve here.
pub struct PannedSource<S: Source<Item = f32>> {
    inner: S,
    source_channels: u16,
    sample_rate: u32,
    left_gain: f32,
    right_gain: f32,
    left_delay: VecDeque<f32>,
    right_delay: VecDeque<f32>,
    emit_right_next: bool,
}

impl<S: Source<Item = f32>> PannedSource<S> {
    /// `azimuth` in radians (negative = left, positive = right, as produced
    /// by `SpatialAudioEngine::compute_hrtf`). `itd_ms` is the signed
    /// interaural time difference in milliseconds (same sign convention).
    pub fn new(inner: S, azimuth: f32, itd_ms: f32) -> Self {
        let source_channels = inner.channels().max(1);
        let sample_rate = inner.sample_rate();

        // Constant-power (equal-power) pan law: map azimuth to a pan
        // position in [-1, 1] and split it across a quarter-turn so
        // left_gain^2 + right_gain^2 stays ~1.0 at every pan position.
        let pan = (azimuth / std::f32::consts::FRAC_PI_2).clamp(-1.0, 1.0);
        let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
        let left_gain = angle.cos();
        let right_gain = angle.sin();

        // Positive azimuth (source to the right) means the right ear
        // hears it first, so the left channel is the delayed one, and
        // vice versa.
        let delay_samples = ((itd_ms.abs() / 1000.0) as f64 * sample_rate as f64).round() as usize;
        let (left_delay_len, right_delay_len) = if itd_ms >= 0.0 {
            (delay_samples, 0)
        } else {
            (0, delay_samples)
        };

        Self {
            inner,
            source_channels,
            sample_rate,
            left_gain,
            right_gain,
            left_delay: VecDeque::from(vec![0.0f32; left_delay_len]),
            right_delay: VecDeque::from(vec![0.0f32; right_delay_len]),
            emit_right_next: false,
        }
    }
}

impl<S: Source<Item = f32>> Iterator for PannedSource<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.emit_right_next {
            self.emit_right_next = false;
            return Some(self.right_delay.pop_front().unwrap_or(0.0));
        }

        let mut sum = 0.0f32;
        let mut got_any = false;
        for _ in 0..self.source_channels {
            match self.inner.next() {
                Some(s) => {
                    sum += s;
                    got_any = true;
                }
                None => break,
            }
        }
        if !got_any {
            return None;
        }
        let mono = sum / self.source_channels as f32;

        self.left_delay.push_back(mono * self.left_gain);
        self.right_delay.push_back(mono * self.right_gain);
        self.emit_right_next = true;
        Some(self.left_delay.pop_front().unwrap_or(0.0))
    }
}

impl<S: Source<Item = f32>> Source for PannedSource<S> {
    fn current_frame_len(&self) -> Option<usize> {
        // Channel count/sample rate are fixed for this source's lifetime,
        // so there's no upcoming format change to signal.
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ConstMono {
        remaining: usize,
        value: f32,
    }

    impl Iterator for ConstMono {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            if self.remaining == 0 {
                return None;
            }
            self.remaining -= 1;
            Some(self.value)
        }
    }

    impl Source for ConstMono {
        fn current_frame_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> u16 {
            1
        }
        fn sample_rate(&self) -> u32 {
            48_000
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn center_azimuth_has_equal_gain() {
        let source = ConstMono {
            remaining: 4,
            value: 1.0,
        };
        let mut panned = PannedSource::new(source, 0.0, 0.0);
        let l = panned.next().unwrap();
        let r = panned.next().unwrap();
        assert!((l - r).abs() < 1e-5, "left={l} right={r}");
    }

    #[test]
    fn hard_right_azimuth_silences_left_channel() {
        let source = ConstMono {
            remaining: 4,
            value: 1.0,
        };
        let mut panned = PannedSource::new(source, std::f32::consts::FRAC_PI_2, 0.0);
        let l = panned.next().unwrap();
        let r = panned.next().unwrap();
        assert!(l < 1e-3, "left={l}");
        assert!(r > 0.99, "right={r}");
    }

    #[test]
    fn positive_itd_delays_left_channel() {
        let source = ConstMono {
            remaining: 8,
            value: 1.0,
        };
        // At 48kHz, 1ms is 48 samples; use a small itd so the test stays fast.
        let mut panned = PannedSource::new(source, 0.0, 1000.0 / 48_000.0);
        let (first_l, _first_r) = (panned.next().unwrap(), panned.next().unwrap());
        assert_eq!(first_l, 0.0, "left channel should start with the delay pad");
    }

    #[test]
    fn channels_always_stereo() {
        let source = ConstMono {
            remaining: 2,
            value: 0.5,
        };
        let panned = PannedSource::new(source, 0.0, 0.0);
        assert_eq!(panned.channels(), 2);
    }
}
