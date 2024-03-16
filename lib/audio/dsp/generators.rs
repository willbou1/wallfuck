use super::*;

use std::cell::RefCell;
use std::rc::Rc;
use rand::{thread_rng, Rng};
use rand::rngs::ThreadRng;

use super::effects::FxChain;

//==============================================================================
// Framework glue
//==============================================================================
impl DSPBuilder {
    pub fn build_chain(&self,
        module: Rc<RefCell<dyn DSPGenMono>>,
        ) -> Rc<RefCell<Chain>>
    {
        Rc::new(RefCell::new(Chain::new(module)))
    }
    pub fn build_parallel(&self) -> Rc<RefCell<Parallel>> {
        Rc::new(RefCell::new(Parallel::new()))
    }
    pub fn build_noise(&self,
        kind: NoiseKind,
        amplitude: f64,
        ) -> Rc<RefCell<Noise>>
    {
        Rc::new(RefCell::new(Noise::new(kind, amplitude)))
    }
    pub fn build_oscillator(&self,
        kind: WaveKind,
        frequency: Frequency,
        amplitude: f64,
        ) -> Rc<RefCell<Oscillator>>
    {
        Rc::new(RefCell::new(Oscillator::new(kind, frequency, amplitude, self.sample_rate)))
    }
    pub fn build_adsr(&self,
        attack: u64,
        attack_curve: f64,
        peak: f64,
        decay: u64,
        decay_curve: f64,
        sustain: f64,
        release: u64,
        release_curve: f64,
        ) -> Rc<RefCell<ADSR>>
    {
        Rc::new(RefCell::new(ADSR::new(attack, attack_curve, peak, decay,
                    decay_curve, sustain, release, release_curve, self.sample_rate)))
    }
}



//==============================================================================
// Complex singal generators
//==============================================================================
pub struct Chain {
    module: Rc<RefCell<dyn DSPGenMono>>,
    pub fx_chain: FxChain,
    pub enabled: Parameter,

    multi_hold: Mono,
    multi_index: usize,
}
impl Chain {
    pub fn new(module: Rc<RefCell<dyn DSPGenMono>>) -> Self {
        Self {
            module,
            fx_chain: FxChain::new(),
            enabled: Parameter::new(1.),
            multi_hold: 0.,
            multi_index: 0,
        }
    }
}
impl DSPGenMono for Chain {
    fn tick(&mut self, nb_connected: usize) -> Option<Mono> {
        if self.enabled.real_value() == 0. {
            return None;
        }

        self.multi_index += 1;
        if self.multi_index >= nb_connected {
            self.multi_index = 0;
        }
        if self.multi_index != 0 {
            return Some(self.multi_hold);
        }

        let mut count = Rc::strong_count(&self.module);
        if count > 1 {
            count -= 1;
        }
        match self.module.borrow_mut().tick(count) {
            Some(sample) => {
                self.multi_hold = self.fx_chain.tick(sample);
                Some(self.multi_hold)
            },
            None => None,
        }
    }
}

//==============================================================================
pub struct Parallel {
    modules: Vec<Rc<RefCell<dyn DSPGenMono>>>,
    pub enabled: Parameter,

    multi_hold: Mono,
    multi_index: usize,
}
impl Parallel {
    fn new() -> Self {
        Self {
            modules: vec![],
            enabled: Parameter::new(1.),
            multi_hold: 0.,
            multi_index: 0,
        }
    }
    pub fn add(&mut self, module: Rc<RefCell<dyn DSPGenMono>>) {
        self.modules.push(module);
    }
}
impl DSPGenMono for Parallel {
    fn tick(&mut self, nb_connected: usize) -> Option<Mono> {
        if self.enabled.real_value() == 0. {
            return None;
        }

        self.multi_index += 1;
        if self.multi_index >= nb_connected {
            self.multi_index = 0;
        }
        if self.multi_index != 0 {
            return Some(self.multi_hold);
        }

        let mut nb_enabled = 0;
        self.multi_hold = 0.;
        for module in &self.modules {
            let mut count = Rc::strong_count(module);
            if count > 1 {
                count -= 1;
            }

            if let Some(sample) = module.borrow_mut().tick(count) {
                self.multi_hold += sample;
                nb_enabled += 1;
            }
        }
        if nb_enabled > 1 {
            self.multi_hold != self.modules.len() as f64;
        }
        Some(self.multi_hold)
    }
}



//==============================================================================
// Simple singal generators
//==============================================================================
pub enum NoiseKind {
    White,
}
pub struct Noise {
    kind: NoiseKind,
    pub amplitude: Parameter,
    pub enabled: Parameter,
    rng: ThreadRng,

    multi_hold: Mono,
    multi_index: usize,
}
impl Noise {
    fn new(kind: NoiseKind, amplitude: f64) -> Self {
        Self {
            kind,
            multi_hold: 0.,
            multi_index: 0,
            amplitude: Parameter::new(amplitude),
            enabled: Parameter::new(1.),
            rng: thread_rng(),
        }
    }
}
impl DSPGenMono for Noise {
    fn tick(&mut self, nb_connected: usize) -> Option<Mono> {
        let amplitude = self.amplitude.real_value();
        if self.enabled.real_value() == 0. {
            return None;
        }

        self.multi_index += 1;
        if self.multi_index >= nb_connected {
            self.multi_index = 0;
        }
        if self.multi_index != 0 {
            return Some(self.multi_hold);
        }

        self.multi_hold = match self.kind {
            NoiseKind::White => if amplitude != 0. {
                self.rng.gen_range(-amplitude..amplitude)
            } else { 0. },
        };
        Some(self.multi_hold)
    }
}

//==============================================================================
pub enum WaveKind {
    Sine,
    Triangle,
    Square,
    Saw,
}
pub struct Oscillator {
    kind: WaveKind,
    pub frequency: Parameter,
    pub amplitude: Parameter,
    pub enabled: Parameter,
    step: u64,
    sample_rate: u64,

    multi_hold: Mono,
    multi_index: usize,
}
impl Oscillator {
    fn new(
        kind: WaveKind,
        frequency: Frequency,
        amplitude: f64,
        sample_rate: u64
        ) -> Self {
        Self {
            kind,
            frequency: Parameter::new(frequency),
            amplitude: Parameter::new(amplitude),
            enabled: Parameter::new(1.),
            sample_rate,
            step: 0,
            multi_hold: 0.,
            multi_index: 0,
        }
    }
}
impl DSPGenMono for Oscillator {
    fn tick(&mut self, nb_connected: usize) -> Option<Mono> {
        let amplitude = self.amplitude.real_value();
        let frequency = self.frequency.real_value();
        if self.enabled.real_value() == 0. {
            return None;
        }

        self.multi_index += 1;
        if self.multi_index >= nb_connected {
            self.multi_index = 0;
        }
        if self.multi_index != 0 {
            return Some(self.multi_hold);
        }

        let nb_samples_cycle = (self.sample_rate as f64 / frequency) as u64;
        self.multi_hold = match self.kind {
            WaveKind::Sine =>
                (self.step as f64 * 2f64 * std::f64::consts::PI / self.sample_rate as f64 * frequency).sin(),
            WaveKind::Square =>
                if self.step < nb_samples_cycle / 2 { 1. }
                else { -1. },
            WaveKind::Saw => (2. / nb_samples_cycle as f64) * self.step as f64 - 1.,
            WaveKind::Triangle => if self.step < nb_samples_cycle / 2 {
                (2. / (nb_samples_cycle as f64 / 2.)) * self.step as f64 - 1.
            } else {
                (-2. / (nb_samples_cycle as f64 / 2.)) * (self.step as f64 - nb_samples_cycle as f64) - 1.
            },
        } * amplitude;
        self.step += 1;
        if self.step >= nb_samples_cycle {
            self.step = 0;
        }
        Some(self.multi_hold)
    }
}

//==============================================================================
pub enum ADSRState {
    Attack(u64),
    Decay(u64),
    Sustain,
    Release(u64),
    Off,
}
pub struct ADSR {
    pub state: ADSRState,
    pub enabled: Parameter,
    attack: u64,
    attack_curve: f64,
    peak: f64,
    decay: u64,
    decay_curve: f64,
    sustain: f64,
    release: u64,
    release_curve: f64,
    sample_rate: u64,

    multi_hold: Mono,
    multi_index: usize,
}
impl ADSR {
    fn new(
        attack: u64,
        attack_curve: f64,
        peak: f64,
        decay: u64,
        decay_curve: f64,
        sustain: f64,
        release: u64,
        release_curve: f64,
        sample_rate: u64,
        ) -> Self {
        Self {
            attack,
            attack_curve,
            peak,
            decay,
            decay_curve,
            sustain,
            release,
            release_curve,
            sample_rate,
            state: ADSRState::Off,
            enabled: Parameter::new(1.),
            multi_hold: 0.,
            multi_index: 0,
        }
    }
}
impl DSPGenMono for ADSR {
    fn tick(&mut self, nb_connected: usize) -> Option<Mono> {
        if self.enabled.real_value() == 0. {
            return None;
        }

        self.multi_index += 1;
        if self.multi_index >= nb_connected {
            self.multi_index = 0;
        }
        if self.multi_index != 0 {
            return Some(self.multi_hold);
        }

        let nb_samples_ms = |ms: u64| (ms as f64 / 1000. * self.sample_rate as f64).round();
        self.multi_hold = match self.state {
            ADSRState::Attack(step) => {
                let nb_samples = nb_samples_ms(self.attack);
                let new_sample = self.peak * (step as f64).powf(self.attack_curve)
                    / nb_samples.powf(self.attack_curve);
                self.state = if step as f64 >= nb_samples {
                    ADSRState::Decay(0)
                } else {
                    ADSRState::Attack(step + 1)
                };
                new_sample
            },
            ADSRState::Decay(step) => {
                let nb_samples = nb_samples_ms(self.decay);
                let new_sample = -(self.peak - self.sustain) * (step as f64).powf(self.decay_curve)
                    / nb_samples.powf(self.decay_curve) + self.peak;
                self.state = if step as f64 >= nb_samples {
                    ADSRState::Sustain
                } else {
                    ADSRState::Decay(step + 1)
                };
                new_sample
            },
            ADSRState::Sustain => self.sustain,
            ADSRState::Release(step) => {
                let nb_samples = nb_samples_ms(self.release);
                let new_sample = -self.sustain * (step as f64).powf(self.release_curve)
                    / nb_samples.powf(self.release_curve) + self.sustain;
                self.state = if step as f64 >= nb_samples {
                    ADSRState::Off
                } else {
                    ADSRState::Release(step + 1)
                };
                new_sample
            },
            ADSRState::Off => 0.,
        };
        Some(self.multi_hold)
    }
}
