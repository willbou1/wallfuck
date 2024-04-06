use num::complex::{Complex, ComplexFloat};
use std::f64::consts::PI;

pub enum WindowMode {
    ZeroPadding,
    Hann,
}

pub struct FourierTransform {
    window_mode: WindowMode,
    size: usize,
    sample_rate: u64,
    buffer: Vec<f64>,
    bins: Vec<Complex<f64>>,
    bins_calculated: bool,
}
impl FourierTransform {
    pub fn new(window_mode: WindowMode, size: usize, sample_rate: u64) -> Self {
        Self {
            window_mode,
            size,
            sample_rate,
            buffer: vec![0.; size],
            bins: Vec::new(),
            bins_calculated: false,
        }
    }
    pub fn process(&mut self, samples: &[f64]) -> Result<(), String> {
        let nb_samples = samples.len();
        if nb_samples > self.size {
            return Err("Too much samples for an FFT of this size".to_string())
        }

        self.buffer[..nb_samples].clone_from_slice(samples);
        self.buffer[nb_samples..].fill(0.);
        match self.window_mode {
            WindowMode::Hann =>
                for (i, s) in self.buffer[..nb_samples].iter_mut().enumerate() {
                    *s = *s * (PI * i as f64 / nb_samples as f64).sin().powi(2);
                },
            _ => (),
        }
        self.bins_calculated = false;
        Ok(())
    }
    pub fn analyse(&self, frequency: f64) -> (f64, f64) {
        let constant = -2. * PI / self.size as f64 * (frequency * self.size as f64 / self.sample_rate as f64);
        let complex = self.buffer.iter().enumerate().fold(Complex::new(0., 0.), |a, (i, s)| 
            a + s * Complex::new(0., constant * i as f64).exp()
        ) / self.size as f64;
        (complex.norm_sqr(), complex.im.atan2(complex.re) * 180. / PI)
    }
    pub fn bins(&mut self) -> &[Complex<f64>] {
        self.compute_bins();
        &self.bins
    }
    pub fn inverse(&mut self) -> Vec<f64> {
        self.compute_bins();
        Self::fft(&self.bins, true).iter()
            .map(|&complex_sample| complex_sample.re).collect()
    }

    fn compute_bins(&mut self) {
        if !self.bins_calculated {
            self.bins_calculated = true;
            self.bins = Self::fft(
                &(self.buffer.iter().map(|&sample| Complex::new(sample, 0.)).collect::<Vec<_>>()),
                false,
            );
            // Normalisation
            for bin in &mut self.bins {
                *bin /= self.size as f64;
            }
        }
    }
    fn fft(inputs: &[Complex<f64>], inverse: bool) -> Vec<Complex<f64>> {
        if inputs.len() == 0 {
            return Vec::new();
        }
        let mut len = inputs.len();
        while len != 1 {
            if len % 2 != 0 {
                panic!("The number of samples provided is not a power of 2");
            }
            len /= 2;
        }
        fn fft_(
            inputs: &[Complex<f64>],
            inverse: bool,
            size: usize,
            step: usize,
            start_index: usize
            ) -> Vec<Complex<f64>>
        {
            let mut result = vec![Complex::new(0., 0.); inputs.len()];
            if size == 1 {
                result[0] = inputs[start_index];
                return result;
            }
            let half_size = size / 2;
            let constant = if inverse {
                    2. * PI / size as f64
                } else {
                    -2. * PI / size as f64
                };
            let g = fft_(inputs, inverse, half_size, step * 2, start_index);
            let h = fft_(inputs, inverse, half_size, step * 2, start_index + step);
            for i in 0..size {
                result[i] = g[i % half_size]
                    + Complex::new(0., constant * i as f64).exp() * h[i % half_size];
            }
            result
        }
        fft_(inputs, inverse, inputs.len(), 1, 0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    static SAMPLES: &'static [f64] = &[
        41., 42., 16., 31., 33., 18., 20., 22., 
        15., 33., 44., 11., 19., 27., 32., 21., 
        41., 47., 17., 39., 45., 17., 5., 2., 
        10., 9., 32., 38., 26., 0., 19., 34.
    ];

    #[test]
    #[should_panic]
    fn fft_number_samples_not_square_of_2() {
        let samples: Vec<f64> = vec![1., 2., 3., 4., 5., 6., 7., 8., 9.];
        let mut fourier = FourierTransform::new(WindowMode::ZeroPadding, 9, 44100);
        fourier.process(&samples);
        fourier.bins();
    }

    #[test]
    fn fft_0_samples() {
        let samples: Vec<f64> = Vec::new();
        let mut fourier = FourierTransform::new(WindowMode::ZeroPadding, 0, 44100);
        fourier.process(&samples);
        let actual = fourier.bins();
        assert!(actual.len() == 0);
    }

    #[test]
    fn fft_32_samples() {
        let delta = 0.001f64;
        let mut fourier = FourierTransform::new(WindowMode::ZeroPadding, 32, 44100);
        fourier.process(&SAMPLES);

        let expected: Vec<Complex<f64>> = vec![
            Complex::new(806., 0.), Complex::new(-21.801, -49.683),
            Complex::new(122.017, -17.125), Complex::new(-16.353, 50.703),
            Complex::new(4.506, -110.075), Complex::new(52.573, -57.875),
            Complex::new(86.86, 78.106), Complex::new(56.461, 18.509),
            Complex::new(45., 5.), Complex::new(-39.959, 94.453),
            Complex::new(51.182, -85.475), Complex::new(-29.244,  -19.497),
            Complex::new(-36.506, -44.075), Complex::new(7.166, 15.341),
            Complex::new(-32.058, -48.706), Complex::new(-8.842, -7.479),
            Complex::new(24., 0.), Complex::new(-8.842, 7.479),
            Complex::new(-32.058, 48.706), Complex::new(7.166, -15.341),
            Complex::new(-36.506, 44.075), Complex::new(-29.244, 19.497),
            Complex::new(51.182, 85.475), Complex::new(-39.959, -94.453),
            Complex::new(45., -5.), Complex::new(56.461, -18.509),
            Complex::new(86.86, -78.106), Complex::new(52.573, 57.875),
            Complex::new(4.506, 110.075), Complex::new(-16.353, -50.703),
            Complex::new(122.017, 17.125), Complex::new(-21.801, 49.683),
        ].iter().map(|complex_frequency| complex_frequency / 32. as f64).collect();
        let actual = fourier.bins();
        for it in actual.iter().zip(expected.iter()) {
            let (af, ef) = it;
            assert!(af.re >= ef.re - delta && af.re <= ef.re + delta);
            assert!(af.im >= ef.im - delta && af.im <= ef.im + delta);
        }
    }

    #[test]
    fn fft_ifft_32_samples() {
        let delta = 0.0000000001f64;
        let mut fourier = FourierTransform::new(WindowMode::ZeroPadding, 32, 44100);
        fourier.process(&SAMPLES);

        let actual = fourier.inverse();
        for (aa, ea) in actual.iter().zip(SAMPLES.iter()) {
            assert!(aa >= &(ea - delta));
            assert!(aa >= &(ea - delta));
        }
    }

    #[test]
    fn fft_specific_frequency() {
        let delta = 0.1f64;
        let mut fourier = FourierTransform::new(WindowMode::ZeroPadding, 32, 44100);
        fourier.process(&SAMPLES);

        let (actual, _) = fourier.analyse(1. * 44100. / 32.);
        let expected_complex = Complex::new(-21.801, -49.683) / 32.;
        assert!((actual - expected_complex.norm_sqr()).abs() <= delta);
    }
}
