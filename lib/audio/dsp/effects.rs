#![allow(dead_code)]

use super::*;

use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;
use std::cmp;

//==============================================================================
// Framework glue
//==============================================================================
impl DSPBuilder {
    pub fn build_fx_chain(&self) -> Rc<RefCell<FxChain>> {
        Rc::new(RefCell::new(FxChain::new()))
    }
    pub fn build_down_sample(&self, factor: u8) -> Rc<RefCell<DownSample>> {
        Rc::new(RefCell::new(DownSample::new(factor)))
    }
    pub fn build_first_order_filter(&self,
        kind: FirstOrderFilterKind,
        cut_off: Frequency,
        ) -> Rc<RefCell<FirstOrderFilter>>
    {
        Rc::new(RefCell::new(FirstOrderFilter::new(kind, cut_off, self.sample_rate)))
    }
    pub fn build_second_order_filter(&self,
        kind: SecondOrderFilterKind,
        cut_off: Frequency,
        curve: f64,
        ) -> Rc<RefCell<SecondOrderFilter>>
    {
        Rc::new(RefCell::new(SecondOrderFilter::new(kind, cut_off, curve, self.sample_rate)))
    }
    pub fn build_moving_average(&self,
        window_size: usize,
        ) -> Rc<RefCell<MovingAverage>>
    {
        Rc::new(RefCell::new(MovingAverage::new(window_size)))
    }
    pub fn build_amplifier(&self, amplitude: f64) -> Rc<RefCell<Amplifier>>
    {
        Rc::new(RefCell::new(Amplifier::new(amplitude)))
    }
    pub fn build_operator<F>(&self,
        operator: F,
        ) -> Rc<RefCell<Operator<F>>> where
        F: Fn(f64) -> f64
    {
        Rc::new(RefCell::new(Operator::new(operator)))
    }
    pub fn build_slide(&self, slide_up: f64, slide_down: f64) -> Rc<RefCell<Slide>> {
        Rc::new(RefCell::new(Slide::new(slide_up, slide_down)))
    }
}



//==============================================================================
// Complex effects
//==============================================================================
pub struct FxChain {
    effects: VecDeque<Rc<RefCell<dyn DSPMonoEffect>>>,
    pub enabled: Parameter,
}
impl FxChain {
    pub fn new() -> Self {
        Self {
            effects: VecDeque::new(),
            enabled: Parameter::new(1.),
        }
    }
    pub fn insert(&mut self, effect: Rc<RefCell<dyn DSPMonoEffect>>) {
        self.effects.push_front(effect);
    }
    pub fn append(&mut self, effect: Rc<RefCell<dyn DSPMonoEffect>>) {
        self.effects.push_back(effect);
    }
}
impl DSPMonoEffect for FxChain {
    fn tick(&mut self, mut sample: Mono) -> Mono {
        if self.enabled.real_value() == 0. {
            return sample;
        }

        for effect in &self.effects {
            sample = effect.borrow_mut().tick(sample);
        }
        sample
    }
}



//==============================================================================
// Simple effects
//==============================================================================
pub struct Amplifier {
    pub amplitude: Parameter,
    pub enabled: Parameter,
}
impl Amplifier {
    pub fn new(amplitude: f64) -> Self {
        Self {
            amplitude: Parameter::new(amplitude),
            enabled: Parameter::new(1.),
        }
    }
}
impl DSPMonoEffect for Amplifier {
    fn tick(&mut self, sample: f64) -> f64 {
        let amplitude = self.amplitude.real_value();
        if self.enabled.real_value() == 0. {
            return sample;
        }
        amplitude * sample
    }
}

//==============================================================================
pub struct Operator<F> {
    operator: F,
    pub enabled: Parameter,
}
impl<F> Operator<F> where
    F: Fn(f64) -> f64,
{
    pub fn new(operator: F) -> Self {
        Self {
            operator,
            enabled: Parameter::new(1.),
        }
    }
}
impl<F> DSPMonoEffect for Operator<F> where
    F: Fn(f64) -> f64,
{
    fn tick(&mut self, sample: f64) -> f64 {
        if self.enabled.real_value() == 0. {
            return sample;
        }

        (self.operator)(sample)
    }
}

//==============================================================================
pub struct DownSample {
    hold: f64,
    step: u64,
    pub factor: Parameter,
    pub enabled: Parameter,
}
impl DownSample {
    fn new(factor: u8) -> Self {
        Self {
            hold: 0.,
            step: 0,
            factor: Parameter::new(factor as f64),
            enabled: Parameter::new(1.),
        }
    }
}
impl DSPMonoEffect for DownSample {
    fn tick(&mut self, sample: Mono) -> Mono {
        let factor = self.factor.real_value() as u64;
        if self.enabled.real_value() == 0. {
            return sample;
        }

        if self.step % factor == 0 {
            self.hold = sample;
            self.step = 0;
        }
        self.step += 1;
        self.hold
    }
}

//==============================================================================
pub enum FirstOrderFilterKind {
    AllPass,
    LowPass,
    HighPass,
}
pub struct FirstOrderFilter {
    kind: FirstOrderFilterKind,
    pub cut_off: Parameter,
    pub enabled: Parameter,
    old_cut_off: f64,
    coefficient: f64,
    buffer: f64,
    sample_rate: u64,
}
impl FirstOrderFilter {
    fn new(kind: FirstOrderFilterKind, cut_off: Frequency, sample_rate: u64) -> Self {
        Self {
            kind,
            sample_rate,
            old_cut_off: cut_off + 1.,
            coefficient: 0.,
            buffer: 0.,
            cut_off: Parameter::new(cut_off),
            enabled: Parameter::new(1.),
        }
    }
}
impl DSPMonoEffect for FirstOrderFilter {
    fn tick(&mut self, sample: Mono) -> Mono {
        let cut_off = self.cut_off.real_value();
        if self.enabled.real_value() == 0. {
            return sample;
        }

        if cut_off != self.old_cut_off {
            self.old_cut_off = cut_off;
            let tan = (std::f64::consts::PI * cut_off / self.sample_rate as f64).tan();
            self.coefficient = (tan - 1.) / (tan + 1.);
        }

        let all_pass = self.coefficient * sample + self.buffer;
        self.buffer = sample - self.coefficient * all_pass;
        match self.kind {
            FirstOrderFilterKind::AllPass => all_pass,
            FirstOrderFilterKind::HighPass => (sample - all_pass) / 2.,
            FirstOrderFilterKind::LowPass => (sample + all_pass) / 2.,
        }
    }
}

//==============================================================================
pub enum SecondOrderFilterKind {
    AllPass,
    BandStop,
    BandPass,
}
pub struct SecondOrderFilter {
    kind: SecondOrderFilterKind,
    pub cut_off: Parameter,
    pub curve: Parameter,
    pub enabled: Parameter,
    old_cut_off: f64,
    old_curve: f64,
    d: f64,
    c: f64,
    buffer: [f64; 2],
    sample_rate: u64,
}
impl SecondOrderFilter {
    fn new(
        kind: SecondOrderFilterKind,
        cut_off: Frequency,
        curve: f64,
        sample_rate: u64,
        ) -> Self
    {
        Self {
            kind,
            sample_rate,
            old_cut_off: cut_off + 1.,
            old_curve: curve + 1.,
            c: 0.,
            d: 0.,
            buffer: [0., 0.],
            cut_off: Parameter::new(cut_off),
            curve: Parameter::new(curve),
            enabled: Parameter::new(1.),
        }
    }
}
impl DSPMonoEffect for SecondOrderFilter {
    fn tick(&mut self, sample: Mono) -> Mono {
        let cut_off = self.cut_off.real_value();
        let curve = self.curve.real_value();
        if self.enabled.real_value() == 0. {
            return sample;
        }

        let sample_rate = self.sample_rate as f64;
        if cut_off != self.old_cut_off {
            self.old_cut_off = cut_off;
            self.d = -(2. * std::f64::consts::PI * cut_off / sample_rate).cos();
        }
        if curve != self.old_curve {
            self.old_curve = curve;
            let tan = (std::f64::consts::PI * curve).tan();
            self.c = (tan - 1.) / (tan + 1.);
        }

        let d_c = self.d * (1. - self.c);
        let v = sample - d_c * self.buffer[0] + self.c * self.buffer[1];
        let all_pass = -self.c * v + d_c * self.buffer[0] + self.buffer[1];
        self.buffer[1] = self.buffer[0];
        self.buffer[0] = v;
        match self.kind {
            SecondOrderFilterKind::AllPass => all_pass,
            SecondOrderFilterKind::BandPass => (sample - all_pass) / 2.,
            SecondOrderFilterKind::BandStop => (sample + all_pass) / 2.,
        }
    }
}
//==============================================================================
pub struct Slide {
    pub slide_up: Parameter,
    pub slide_down: Parameter,
    pub enabled: Parameter,
    buffer: f64,
}
impl Slide {
    fn new(slide_up: f64, slide_down: f64) -> Self {
        Self {
            slide_up: Parameter::new(slide_up),
            slide_down: Parameter::new(slide_down),
            enabled: Parameter::new(1.),
            buffer: 0.,
        }
    }
}
impl DSPMonoEffect for Slide {
    fn tick(&mut self, sample: Mono) -> Mono {
        let slide_up = self.slide_up.real_value();
        let slide_down = self.slide_down.real_value();
        if self.enabled.real_value() == 0. {
            return sample;
        }

        let slide = if sample - self.buffer > 0. {slide_up} else {slide_down};
        self.buffer += (sample - self.buffer) / slide;
        self.buffer
    }
}

//==============================================================================
pub struct MovingAverage {
    window_size: usize,
    processed: usize,
    index: usize,
    buffer: Vec<f64>,
    pub enabled: Parameter,
}
impl MovingAverage {
    fn new(window_size: usize) -> Self{
        Self {
            window_size,
            processed: 0,
            buffer: vec![0.; window_size],
            index: 0,
            enabled: Parameter::new(1.),
        }
    }
    pub fn set_window_size(&mut self, window_size: usize) {
        if self.window_size == window_size {
            return;
        }
        let mut new_buffer = vec![0.; window_size];
        let size_to_copy = cmp::min(self.processed, window_size);
        for i in 0..size_to_copy {
            let signed_index = self.index as i64 - i as i64;
            let real_index =
                if signed_index < 0 {
                    (self.window_size as i64 + signed_index) as usize
                } else {
                    self.index - i as usize
                };
            new_buffer.push(self.buffer[real_index]);
        }
        self.window_size = window_size;
        self.index = self.processed - 1;
    }
}
impl DSPMonoEffect for MovingAverage {
    fn tick(&mut self, sample: Mono) -> Mono {
        if self.enabled.real_value() == 0. {
            return sample;
        }

        self.index = if self.index == self.window_size - 1 {
            0
        } else {
            self.index + 1
        };
        self.buffer[self.index] = sample;
        if self.processed < self.window_size {
            self.processed += 1;
        }
        let mut sum = 0_f64;
        for i in 0..self.processed {
            let signed_index = self.index as i64 - i as i64;
            let real_index =
                if signed_index < 0 {
                    (self.window_size as i64 + signed_index) as usize
                } else {
                    self.index - i as usize
                };
            sum += self.buffer[real_index];
        }
        sum / self.processed as f64
    }
}



//==============================================================================
// Tests
//==============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, Rng};
    use crate::audio::fft::*;

    const SAMPLE_RATE: u64 = 44100;
    const NB_SAMPLES_FFT_TESTS: usize = 4096; // 2 ^ 12

    fn prepare_fft_test() -> (Vec<f64>, FourierTransform) {
        let mut noise = vec![0.; NB_SAMPLES_FFT_TESTS];
        let mut rng = thread_rng();
        for sample in &mut noise {
            *sample = rng.gen_range((-1.)..(1.));
        }
        let mut fourier = FourierTransform::new(
            WindowMode::Hann,
            NB_SAMPLES_FFT_TESTS,
            SAMPLE_RATE
        );
        if let Err(_) = fourier.process(&noise) {
            assert!(false);
        }
        (noise, fourier)
    }

    #[test]
    fn fx_chain_2_samples() {
        let dsp_builder = DSPBuilder::new(SAMPLE_RATE);
        let fx_chain = dsp_builder.build_fx_chain();
        let mock_fx_1 = Rc::new(RefCell::new(MockDSPMonoEffect::new()));
        let mock_fx_2 = Rc::new(RefCell::new(MockDSPMonoEffect::new()));
        mock_fx_1.borrow_mut().expect_tick()
            .times(2).returning(|sample| sample + 1.);
        mock_fx_2.borrow_mut().expect_tick()
            .times(2).returning(|sample| sample * 2.);
        fx_chain.borrow_mut().append(mock_fx_2);
        fx_chain.borrow_mut().insert(mock_fx_1);

        let mut actual = fx_chain.borrow_mut().tick(2.);
        assert_eq!(actual, 6.); // (2 + 1) * 2 = 6
        actual = fx_chain.borrow_mut().tick(10.);
        assert_eq!(actual, 22.); // (10 + 1) * 2 = 22
    }

    //==========================================================================
    #[test]
    fn amplifier_3_samples() {
        let dsp_builder = DSPBuilder::new(SAMPLE_RATE);
        let absolute = dsp_builder.build_amplifier(5.);

        let mut actual = absolute.borrow_mut().tick(2.);
        assert_eq!(actual, 10.);
        actual = absolute.borrow_mut().tick(-3.);
        assert_eq!(actual, -15.);
        actual = absolute.borrow_mut().tick(0.);
        assert_eq!(actual, 0.);
    }

    //==========================================================================
    #[test]
    fn operator_3_samples() {
        let dsp_builder = DSPBuilder::new(SAMPLE_RATE);
        let operator = dsp_builder.build_operator(|sample| sample * 4. + 2.);

        let mut actual = operator.borrow_mut().tick(2.);
        assert_eq!(actual, 10.);
        actual = operator.borrow_mut().tick(-3.);
        assert_eq!(actual, -10.);
        actual = operator.borrow_mut().tick(0.);
        assert_eq!(actual, 2.);
    }

    //==========================================================================
    #[test]
    fn down_sample_5_samples() {
        let dsp_builder = DSPBuilder::new(SAMPLE_RATE);
        let down_sample = dsp_builder.build_down_sample(3);

        let mut actual = down_sample.borrow_mut().tick(2.);
        assert_eq!(actual, 2.);
        actual = down_sample.borrow_mut().tick(-3.);
        assert_eq!(actual, 2.);
        actual = down_sample.borrow_mut().tick(0.);
        assert_eq!(actual, 2.);
        actual = down_sample.borrow_mut().tick(10.);
        assert_eq!(actual, 10.);
        actual = down_sample.borrow_mut().tick(9.);
        assert_eq!(actual, 10.);
    }

    //==========================================================================
    #[test]
    fn first_order_filter_low_pass_5KHz_fft() {
        let dsp_builder = DSPBuilder::new(SAMPLE_RATE);
        let filter = dsp_builder.build_first_order_filter(
            FirstOrderFilterKind::LowPass, 5000.
        );

        let (mut noise, mut fourier) = prepare_fft_test();
        let (mut amp_start, _) = fourier.analyse(100.);
        let (mut amp_break, _) = fourier.analyse(5000.);
        let (mut amp_nyquist, _) = fourier.analyse(22050.);

        for sample in &mut noise {
            *sample = filter.borrow_mut().tick(*sample);
        }
        if let Err(_) = fourier.process(&noise) {
            assert!(false);
        }
        amp_start = (fourier.analyse(100.).0 - amp_start) / amp_start;
        amp_break = (fourier.analyse(5000.).0 - amp_break) / amp_break;
        amp_nyquist = (fourier.analyse(22050.).0 - amp_nyquist) / amp_nyquist;

        println!("Reduction in phase at 100Hz : {}", amp_start);
        println!("Reduction in phase at 5KHz : {}", amp_break);
        println!("Reduction in phase at 22.05KHz : {}", amp_nyquist);
        assert!(amp_start.abs() <= 0.01);
        assert!((0.5 - amp_break.abs()) <= 0.01);
        assert!((1. - amp_nyquist.abs()) <= 0.01);
    }

    //==========================================================================
    #[test]
    fn second_order_filter_band_stop_5KHz_fft() {
        let dsp_builder = DSPBuilder::new(SAMPLE_RATE);
        let filter = dsp_builder.build_second_order_filter(
            SecondOrderFilterKind::BandStop, 5000., 0.022
        );

        let (mut noise, mut fourier) = prepare_fft_test();
        let (mut amp_start, _) = fourier.analyse(100.);
        let (mut amp_break, _) = fourier.analyse(5000.);
        let (mut amp_nyquist, _) = fourier.analyse(22050.);

        for sample in &mut noise {
            *sample = filter.borrow_mut().tick(*sample);
        }
        if let Err(_) = fourier.process(&noise) {
            assert!(false);
        }
        amp_start = (fourier.analyse(100.).0 - amp_start) / amp_start;
        amp_break = (fourier.analyse(5000.).0 - amp_break) / amp_break;
        amp_nyquist = (fourier.analyse(22050.).0 - amp_nyquist) / amp_nyquist;

        println!("Reduction in phase at 100Hz : {}", amp_start);
        println!("Reduction in phase at 5KHz : {}", amp_break);
        println!("Reduction in phase at 22.05KHz : {}", amp_nyquist);
        assert!(amp_start.abs() <= 0.01);
        assert!(1. - amp_break.abs() <= 0.01);
        assert!(amp_nyquist.abs() <= 0.01);
    }

    #[test]
    fn slide_1_for_up_down_4_samples() {
        let dsp_builder = DSPBuilder::new(SAMPLE_RATE);
        let slide = dsp_builder.build_slide(1., 1.);

        let mut actual = slide.borrow_mut().tick(2.);
        assert_eq!(actual, 2.);
        actual = slide.borrow_mut().tick(3.);
        assert_eq!(actual, 3.);
        actual = slide.borrow_mut().tick(1.);
        assert_eq!(actual, 1.);
        actual = slide.borrow_mut().tick(-4.);
        assert_eq!(actual, -4.);
    }

    #[test]
    fn slide_distinct_up_down_4_samples() {
        const SLIDE_UP: f64 = 1. / 2.;
        const SLIDE_DOWN: f64 = 1. / 3.;
        let dsp_builder = DSPBuilder::new(SAMPLE_RATE);
        let slide = dsp_builder.build_slide(SLIDE_UP, SLIDE_DOWN);

        let mut actual = slide.borrow_mut().tick(2.);
        assert_eq!(actual, 0. + (2. - 0.) / SLIDE_UP);
        actual = slide.borrow_mut().tick(3.);
        assert_eq!(actual, 4. + (3. - 4.) / SLIDE_DOWN);
        actual = slide.borrow_mut().tick(1.);
        assert_eq!(actual, 1. + (1. - 1.) / SLIDE_DOWN);
        actual = slide.borrow_mut().tick(-4.);
        assert_eq!(actual, 1. + (-4. - 1.) / SLIDE_DOWN);
    }
}

