use std::sync::Arc;

use num::Zero;
use rustfft::{num_complex::Complex, FftPlanner};
use serde::Serialize;

/// Short time fourier transform, which decomposes a signal into the energy levels for each frequency
/// for each timestep.
///
/// Manages internal state to avoid repeat allocations over multiple calls.
pub struct Stft {
    fft: Arc<dyn rustfft::Fft<f32>>,
    window_size: usize,
    hop_size: usize,

    /// Reusable internal complex fft output buffer.
    window_buff: Vec<Complex<f32>>,
    /// Reusable internal fft scratch buffer.
    window_scratch: Vec<Complex<f32>>,
}

impl Stft {
    pub fn new(window_size: usize, hop_size: usize) -> Self {
        let fft = FftPlanner::<f32>::new().plan_fft_forward(window_size);
        let scratch_len = fft.get_inplace_scratch_len();

        Self {
            fft,
            window_size,
            hop_size,

            window_buff: vec![Complex::zero(); window_size],
            window_scratch: vec![Complex::zero(); scratch_len],
        }
    }

    /// Computes the short time fourier transform with hann window smoothing
    /// for `windows` windows starting from `offset`.
    pub fn compute(&mut self, audio_pwr: &[f32], offset: usize, windows: usize) -> Spectrogram {
        let mut fft_outputs = Vec::with_capacity(windows * self.fft.len());
        let unique_freqs = self.window_size / 2 + 1;

        // compute windowed fft for every window
        for i in 0..windows {
            fft_outputs.extend(self.windowed_fft(audio_pwr, offset + i * self.hop_size));
        }

        Spectrogram {
            powers: fft_outputs,
            freq_bins: unique_freqs,
        }
    }

    /// Computes a single window of the fast fourier transform with hann window smoothing.
    /// Starts from `offset` within the audio array.
    fn windowed_fft(&mut self, audio_pwr: &[f32], offset: usize) -> impl Iterator<Item = f32> + '_ {
        // apply window smoothing
        for (i, w) in apodize::hanning_iter(self.window_size).enumerate() {
            self.window_buff[i] = Complex::new(audio_pwr[offset + i] * w as f32, 0.0);
        }

        // compute fft
        self.fft
            .process_with_scratch(&mut self.window_buff, &mut self.window_scratch);

        return self
            .window_buff
            .iter()
            .cloned()
            // ft result is symmetric, only first window_size / 2 + 1 samples are unique
            .take(self.window_size / 2 + 1)
            // square norm of complex fft output
            .map(|c| c.norm_sqr());
    }
}

/// Output of a [`Stft`]. That is, the energy level for each frequency for each timestep.
#[derive(Debug, Serialize)]
pub struct Spectrogram {
    /// The energy levels.
    pub powers: Vec<f32>,
    /// The number of frequencies per timestep.
    pub freq_bins: usize,
}

impl Spectrogram {
    /// Returns the mean of all windows.
    pub fn windows_mean(self) -> Self {
        let mut powers = self.powers[0..self.freq_bins].to_vec();
        for (i, p) in self.powers.iter().skip(self.freq_bins).enumerate() {
            powers[i % self.freq_bins] += p;
        }

        let windows = (self.powers.len() / self.freq_bins) as f32;
        powers.iter_mut().for_each(|power| *power /= windows);

        Self {
            powers,
            freq_bins: self.freq_bins,
        }
    }
}
