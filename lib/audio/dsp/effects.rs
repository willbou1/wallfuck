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
    pub fn build_absolute(&self) -> Rc<RefCell<Absolute>>
    {
        Rc::new(RefCell::new(Absolute::new()))
    }
    pub fn build_operator<F>(&self,
        operator: F,
        ) -> Rc<RefCell<Operator<F>>> where
        F: Fn(f64) -> f64
    {
        Rc::new(RefCell::new(Operator::new(operator)))
    }
}



//==============================================================================
// Complex mono effects
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
// Simple mono effects
//==============================================================================
pub struct Absolute {
    pub enabled: Parameter,
}
impl Absolute {
    pub fn new() -> Self {
        Self {
            enabled: Parameter::new(1.),
        }
    }
}
impl DSPMonoEffect for Absolute {
    fn tick(&mut self, sample: f64) -> f64 {
        sample.abs()
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
    factor: u8,
    hold: f64,
    step: u8,
    pub enabled: Parameter,
}
impl DownSample {
    fn new(factor: u8) -> Self {
        Self {
            factor,
            hold: 0.,
            step: 0,
            enabled: Parameter::new(1.),
        }
    }
}
impl DSPMonoEffect for DownSample {
    fn tick(&mut self, sample: Mono) -> Mono {
        if self.enabled.real_value() == 0. {
            return sample;
        }

        if self.step % self.factor == 0 {
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
        self.buffer[0] = v;
        self.buffer[1] = self.buffer[0];
        match self.kind {
            SecondOrderFilterKind::AllPass => all_pass,
            SecondOrderFilterKind::BandPass => (sample - all_pass) / 2.,
            SecondOrderFilterKind::BandStop => (sample + all_pass) / 2.,
        }
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

