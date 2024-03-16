use std::cell::RefCell;
use std::rc::Rc;

//==============================================================================
// Framework glue
//==============================================================================
pub mod generators;
pub mod effects;

//==============================================================================
pub type Frequency = f64;
pub type Ms = f64;
pub type Mono = f64;
pub type Stereo = (f64, f64);

pub trait DSPGenMono {
    fn tick(&mut self, nb_connected: usize) -> Option<Mono>;
}

pub trait DSPFxMono {
    fn tick(&mut self, sample: Mono) -> Mono;
}

pub trait DSPFxStereo {
    fn tick(&mut self, sample: Mono) -> Mono;
}

pub struct DSPBuilder {
    sample_rate: u64,
}
impl DSPBuilder {
    pub fn new(sample_rate: u64) -> Self {
        Self { sample_rate }
    }
}

//==============================================================================
pub struct Parameter {
    pub value: f64,
    modulators: Vec<Rc<RefCell<dyn DSPGenMono>>>,
}
impl Parameter {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            modulators: vec![],
        }
    }
    pub fn add_modulator(&mut self, modulator: Rc<RefCell<dyn DSPGenMono>>) {
        self.modulators.push(modulator);
    }
    pub fn real_value(&self) -> f64 {
        let mut value = self.value;
        for modulator in &self.modulators {
            let mut count = Rc::strong_count(modulator);
            if count > 1 {
                count -= 1;
            }
            value += modulator.borrow_mut().tick(count).unwrap_or(0.);
        }
        value
    }
}

