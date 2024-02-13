use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;
use std::cmp;
use std::marker::PhantomData;
use std::any::Any;

const SAMPLE_RATE: i64 = 44100;

pub type Freq = f64;

pub trait DSPValue {}
impl DSPValue for f64 {}
impl DSPValue for (f64, f64) {}

pub trait DSP<I, O> where
    I: DSPValue, O:DSPValue,
{
    fn tick(&mut self, input: I) -> O;
}



//==============================================================================
// Simple DSPs
//==============================================================================
pub struct Absolute {}
impl Absolute {
    pub fn new() -> Self {
        Self {}
    }
}
impl DSP<f64, f64> for Absolute {
    fn tick(&mut self, sample: f64) -> f64 {
        sample.abs()
    }
}

pub struct Operator<F> {
    operator: F,
}
impl<F> Operator<F> where
    F: Fn(f64) -> bool,
{
    pub fn new(operator: F) -> Self {
        Self { operator }
    }
}
impl<F> DSP<f64, f64> for Operator<F> where
    F: Fn(f64) -> bool,
{
    fn tick(&mut self, sample: f64) -> f64 {
        (self.operator)(sample) as u64 as f64
    }
}

pub struct Amp {
    scalar: f64,
}
impl Amp {
    pub fn new(scalar: f64) -> Self {
        Self { scalar }
    }
}
impl DSP<f64, f64> for Amp {
    fn tick(&mut self, sample: f64) -> f64 {
        self.scalar * sample
    }
}

pub struct DownSample {
    factor: u8,
    hold: f64,
    step: u8,
}
impl DownSample {
    pub fn new(factor: u8) -> Self {
        Self {
            factor,
            hold: 0.,
            step: 0,
        }
    }
}
impl DSP<f64, f64> for DownSample {
    fn tick(&mut self, sample: f64) -> f64 {
        if self.step % self.factor == 0 {
            self.hold = sample;
            self.step = 0;
        }
        self.step += 1;
        self.hold
    }
}

pub struct MovingAverage {
    window_size: usize,
    processed: usize,
    index: usize,
    buffer: Vec<f64>,
}
impl MovingAverage {
    pub fn new(window_size: usize) -> Self{
        Self {
            window_size,
            processed: 0,
            buffer: vec![0.; window_size],
            index: 0,
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
impl DSP<f64, f64> for MovingAverage {
    fn tick(&mut self, sample: f64) -> f64 {
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

// Simple design : https://www.youtube.com/watch?v=I8_E1ppC3-Q
pub struct LowPass {
    frequency: Freq,
    registry: f64,
}
impl LowPass {
    pub fn new(frequency: Freq) -> Self {
        Self {
            frequency,
            registry: 0.,
        }
    }
}
impl DSP<f64, f64> for LowPass {
    fn tick(&mut self, sample: f64) -> f64 {
        self.registry = 0.9 * sample + 0.9 * self.registry;
        self.registry
    }
}



//==============================================================================
// Complex DSPs
//==============================================================================
pub struct Mono {
    module: Rc<RefCell<dyn DSP<f64, f64>>>,
}
impl Mono {
    pub fn new(module: Rc<RefCell<dyn DSP<f64, f64>>>) -> Self {
        Self { module }
    }
}
impl DSP<(f64, f64), f64> for Mono {
    fn tick(&mut self, sample_pair: (f64, f64)) -> f64 {
        self.module.borrow_mut().tick(0.5 * sample_pair.0 + 0.5 * sample_pair.1)
    }
}

pub struct Chain<IO>
{
    modules: VecDeque<Rc<RefCell<dyn DSP<IO, IO>>>>,
}
impl<IO> Chain<IO> where
    IO: DSPValue,
{
    pub fn new() -> Self {
        Self {
            modules: VecDeque::new(),
        }
    }
    pub fn push_back(&mut self,
        module: Rc<RefCell<dyn DSP<IO, IO>>>)
    {
        self.modules.push_back(module);
    }
    pub fn push_front(&mut self,
        module: Rc<RefCell<dyn DSP<IO, IO>>>)
    {
        self.modules.push_front(module);
    }
}
impl<IO> DSP<IO, IO> for Chain<IO> where
    IO: DSPValue
{
    fn tick(&mut self, mut sample: IO) -> IO {
        for module in &mut self.modules {
            sample = module.borrow_mut().tick(sample);
        }
        sample
    }
}



//==============================================================================
// tests
//==============================================================================
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn absolute_4_ticks() {
        let mut abs = Absolute::new();
        let mut actual = abs.tick(4.);
        assert_eq!(actual, 4.);
        actual = abs.tick(-6.);
        assert_eq!(actual, 6.);
        actual = abs.tick(-16.);
        assert_eq!(actual, 16.);
        actual = abs.tick(42.);
        assert_eq!(actual, 42.);
    }

    #[test]
    fn operator_2_ticks() {
        let mut operator = Operator::new(|sample| sample > 3.);
        let mut actual = operator.tick(4.);
        assert_eq!(actual, 1.);
        actual = operator.tick(1.);
        assert_eq!(actual, 0.);
    }

    #[test]
    fn amp_3_ticks() {
        let mut amp = Amp::new(2.);
        let mut actual = amp.tick(4.);
        assert_eq!(actual, 8.);
        actual = amp.tick(-6.);
        assert_eq!(actual, -12.);
        actual = amp.tick(-16.);
        assert_eq!(actual, -32.);
    }

    #[test]
    fn moving_average_5_ticks() {
        let mut moving_average = MovingAverage::new(3);
        let mut actual = moving_average.tick(4.);
        assert_eq!(actual, 4.);
        actual = moving_average.tick(-6.);
        assert_eq!(actual, -1.);
        actual = moving_average.tick(-16.);
        assert_eq!(actual, -6.);
        actual = moving_average.tick(46.);
        assert_eq!(actual, 8.);
        actual = moving_average.tick(3.);
        assert_eq!(actual, 11.);
    }

    #[test]
    fn chain_absolute_amp_4_ticks() {
        let absolute = Rc::new(RefCell::new(Absolute::new()));
        let amp = Rc::new(RefCell::new(Amp::new(2.)));
        let mut chain: Chain<f64> = Chain::new();
        chain.push_back(absolute.clone());
        chain.push_back(amp.clone());
        let mut actual = chain.tick(4.);
        assert_eq!(actual, 8.);
        actual = chain.tick(-6.);
        assert_eq!(actual, 12.);
        actual = chain.tick(-16.);
        assert_eq!(actual, 32.);

        amp.borrow_mut().scalar = 0.5;
        actual = chain.tick(-10.);
        assert_eq!(actual, 5.);
    }

    #[test]
    fn down_sample_factor_1_2_ticks() {
        let mut down_sample = DownSample::new(1);
        let mut actual = down_sample.tick(4.);
        assert_eq!(actual, 4.);
        actual = down_sample.tick(-6.);
        assert_eq!(actual, -6.);
    }

    #[test]
    fn down_sample_factor_2_4_ticks() {
        let mut down_sample = DownSample::new(2);
        let mut actual = down_sample.tick(4.);
        assert_eq!(actual, 4.);
        actual = down_sample.tick(-6.);
        assert_eq!(actual, 4.);
        actual = down_sample.tick(-16.);
        assert_eq!(actual, -16.);
        actual = down_sample.tick(42.);
        assert_eq!(actual, -16.);
    }

    #[test]
    fn mono_2_ticks() {
        let amp = Rc::new(RefCell::new(Amp::new(1.)));
        let mut mono = Mono::new(amp.clone());
        let mut actual = mono.tick((2., 4.));
        assert_eq!(actual, 3.);
        actual = mono.tick((6., 10.));
        assert_eq!(actual, 8.);
    }
}
