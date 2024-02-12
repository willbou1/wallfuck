use num::complex::Complex;
use std::f64::consts::PI;

fn fft_internal(inputs: &Vec<Complex<f64>>, inverse: bool) -> Vec<Complex<f64>> {
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
        inputs: &Vec<Complex<f64>>,
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

pub fn fft(samples: &Vec<f64>) -> Vec<Complex<f64>> {
    let complex_samples = samples.clone().iter()
        .map(|sample| Complex::new(*sample, 0.))
        .collect();
    fft_internal(&complex_samples, false)
}

pub fn ifft(frequencies: &Vec<Complex<f64>>) -> Vec<f64> {
    let a = fft_internal(frequencies, true);
    a.iter()
        .map(|complex_sample| complex_sample.re / frequencies.len() as f64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn fft_number_samples_not_square_of_2() {
        let samples: Vec<f64> = vec![1., 2., 3., 4., 5., 6., 7., 8., 9.];
        fft(&samples);
    }

    #[test]
    fn fft_0_samples() {
        let samples: Vec<f64> = Vec::new();
        let actual = fft(&samples);
        assert!(actual.len() == 0);
    }

    #[test]
    fn fft_32_samples() {
        let delta = 0.001f64;
        let samples: Vec<f64> = vec![
            41., 42., 16., 31., 33., 18., 20., 22., 
            15., 33., 44., 11., 19., 27., 32., 21., 
            41., 47., 17., 39., 45., 17., 5., 2., 
            10., 9., 32., 38., 26., 0., 19., 34.
        ];
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
        ];
        let actual = fft(&samples);
        for it in actual.iter().zip(expected.iter()) {
            let (af, ef) = it;
            assert!(af.re >= ef.re - delta && af.re <= ef.re + delta);
            assert!(af.im >= ef.im - delta && af.im <= ef.im + delta);
        }
    }

    #[test]
    fn fft_ifft_32_samples() {
        let delta = 0.0000000001f64;
        let samples: Vec<f64> = vec![
            41., 42., 16., 31., 33., 18., 20., 22., 
            15., 33., 44., 11., 19., 27., 32., 21., 
            41., 47., 17., 39., 45., 17., 5., 2., 
            10., 9., 32., 38., 26., 0., 19., 34.
        ];
        let complex_frequencies = fft(&samples);
        let actual = ifft(&complex_frequencies);
        for it in actual.iter().zip(samples.iter()) {
            let (aa, ea) = it;
            assert!(aa >= &(ea - delta));
            assert!(aa >= &(ea - delta));
        }
    }
}
