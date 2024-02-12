use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;
use std::cmp;

const SAMPLE_RATE: i64 = 44100;

type Frequency = f64;

pub trait DspMono {
    fn tick(&mut self, sample: f64) -> f64;
}

pub trait DspStereo {
    fn tick(&mut self, samples: (f64, f64)) -> (f64, f64);
}

struct Absolute {}
impl Absolute {
    pub fn new() -> Self {
        Absolute {}
    }
}
impl DspMono for Absolute {
    fn tick(&mut self, sample: f64) -> f64 {
        sample.abs()
    }
}

struct Amp {
    scalar: f64,
}
impl Amp {
    pub fn new(scalar: f64) -> Self {
        Amp { scalar }
    }
}
impl DspMono for Amp {
    fn tick(&mut self, sample: f64) -> f64 {
        self.scalar * sample
    }
}

struct MovingAverage {
    window_size: usize,
    processed: usize,
    index: usize,
    buffer: Vec<f64>,
}
impl MovingAverage {
    pub fn new(window_size: usize) -> Self{
        MovingAverage {
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
            let real_index =
                if self.index as i64 - (i as i64) < 0 {
                    self.window_size - i as usize
                } else {
                    self.index - i as usize
                };
            new_buffer.push(self.buffer[real_index]);
        }
        self.window_size = window_size;
        self.index = self.processed - 1;
    }
}
impl DspMono for MovingAverage {
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
            let real_index =
                if self.index as i64 - (i as i64) < 0 {
                    self.window_size - i as usize
                } else {
                    self.index - i as usize
                };
            sum += self.buffer[real_index];
        }
        sum / self.processed as f64
    }
}

// Simple design : https://www.youtube.com/watch?v=I8_E1ppC3-Q
struct LowPass {
    frequency: Frequency,
    registry: f64,
}
impl LowPass {
    pub fn new(frequency: Frequency) -> Self {
        LowPass {
            frequency,
            registry: 0.,
        }
    }
}
impl DspMono for LowPass {
    fn tick(&mut self, sample: f64) -> f64 {
        self.registry = 0.9 * sample + 0.9 * self.registry;
        self.registry
    }
}

struct Chain {
    modules: VecDeque<Rc<RefCell<dyn DspMono>>>,
}
impl Chain {
    pub fn new() -> Self {
        Chain {
            modules: VecDeque::new(),
        }
    }
    pub fn push_back(&mut self, module: Rc<RefCell<dyn DspMono>>) {
        self.modules.push_back(module);
    }
    pub fn push_front(&mut self, module: Rc<RefCell<dyn DspMono>>) {
        self.modules.push_front(module);
    }
}
impl DspMono for Chain {
    fn tick(&mut self, mut sample: f64) -> f64 {
        for module in &mut self.modules {
            sample = module.borrow_mut().tick(sample);
        }
        sample
    }
}

