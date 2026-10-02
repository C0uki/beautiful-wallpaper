//! Turning a window of audio into the bars a visualiser draws.
//!
//! A plain radix-2 FFT over a Hann-windowed block, then log-spaced bands from
//! 40 Hz to 16 kHz — the spacing an ear hears as even — each the loudest bin
//! it holds, on a 60 dB scale. Small enough to own rather than depend on.

use std::f32::consts::PI;

/// Samples per analysis: a power of two for the FFT, about 21 ms at 48 kHz.
pub const WINDOW: usize = 1024;

/// `count` bars, each 0–1, for mono `samples` in -1–1 at `sample_rate`.
pub fn bars(samples: &[f32], sample_rate: u32, count: usize) -> Vec<f32> {
    let n = samples.len().next_power_of_two().max(2);
    let mut re = vec![0f32; n];
    let mut im = vec![0f32; n];
    let span = samples.len().saturating_sub(1).max(1) as f32;
    for (i, sample) in samples.iter().enumerate() {
        let hann = 0.5 - 0.5 * (2.0 * PI * i as f32 / span).cos();
        re[i] = sample * hann;
    }
    fft(&mut re, &mut im);

    let bin = sample_rate as f32 / n as f32;
    let low = 40f32;
    let high = (sample_rate as f32 / 2.0).min(16_000.0);
    // A full-scale sine peaks at n/4 once the Hann window has halved it.
    let full_scale = n as f32 / 4.0;
    (0..count)
        .map(|band| {
            let edge = |at: usize| low * (high / low).powf(at as f32 / count as f32);
            let first = ((edge(band) / bin).floor() as usize).min(n / 2 - 1);
            let last = ((edge(band + 1) / bin).ceil() as usize).clamp(first + 1, n / 2);
            let peak = (first..last)
                .map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt())
                .fold(0.0, f32::max);
            let decibels = 20.0 * (peak / full_scale).max(1e-9).log10();
            ((decibels + 60.0) / 60.0).clamp(0.0, 1.0)
        })
        .collect()
}

/// In-place iterative radix-2 FFT. `re.len()` must be a power of two.
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let mut len = 2;
    while len <= n {
        let angle = -2.0 * PI / len as f32;
        let (step_re, step_im) = (angle.cos(), angle.sin());
        for start in (0..n).step_by(len) {
            let (mut w_re, mut w_im) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let t_re = re[b] * w_re - im[b] * w_im;
                let t_im = re[b] * w_im + im[b] * w_re;
                re[b] = re[a] - t_re;
                im[b] = im[a] - t_im;
                re[a] += t_re;
                im[a] += t_im;
                let next = w_re * step_re - w_im * step_im;
                w_im = w_re * step_im + w_im * step_re;
                w_re = next;
            }
        }
        len <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f32, amplitude: f32) -> Vec<f32> {
        (0..WINDOW)
            .map(|i| amplitude * (2.0 * PI * hz * i as f32 / 48_000.0).sin())
            .collect()
    }

    #[test]
    fn silence_draws_no_bars() {
        assert!(bars(&[0.0; WINDOW], 48_000, 32)
            .iter()
            .all(|&bar| bar == 0.0));
    }

    #[test]
    fn a_tone_lights_the_band_it_is_in_and_not_the_far_ones() {
        let drawn = bars(&sine(1_000.0, 0.5), 48_000, 32);
        let loudest = (0..drawn.len())
            .max_by(|&a, &b| drawn[a].total_cmp(&drawn[b]))
            .unwrap();
        // 1 kHz on a log scale from 40 Hz to 16 kHz.
        let expected = ((1_000f32 / 40.0).ln() / (16_000f32 / 40.0).ln() * 32.0) as usize;
        assert!(loudest.abs_diff(expected) <= 1, "{loudest} vs {expected}");
        // -6 dB of full scale, on a 60 dB scale.
        assert!((drawn[loudest] - 0.9).abs() < 0.05, "{}", drawn[loudest]);
        assert!(drawn[0] < 0.3 && drawn[31] < 0.3, "{drawn:?}");
    }

    #[test]
    fn a_louder_tone_draws_a_taller_bar() {
        let quiet = bars(&sine(440.0, 0.05), 48_000, 16);
        let loud = bars(&sine(440.0, 0.8), 48_000, 16);
        let tallest = |drawn: &[f32]| drawn.iter().copied().fold(0.0, f32::max);
        assert!(tallest(&loud) > tallest(&quiet));
    }
}
