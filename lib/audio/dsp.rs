pub type Frequency = f64;
use std::cell::RefCell;
use std::rc::Rc;
pub type Ms = f64;
pub type Mono = f64;
pub type Stereo = (f64, f64);

pub trait DSPGenMono {
    fn tick(&mut self, nb_connected: usize) -> Mono;
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
    pub fn build_oscillator(&self,
        wave_type: WaveType,
        frequency: Frequency,
        amplitude: f64,
        ) -> Rc<RefCell<Oscillator>> {
        Rc::new(RefCell::new(Oscillator::new(wave_type, frequency, amplitude, self.sample_rate)))
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
        ) -> Rc<RefCell<ADSR>> {
        Rc::new(RefCell::new(ADSR::new(attack, attack_curve, peak, decay, decay_curve, sustain, release, release_curve, self.sample_rate)))
    }

    pub fn build_down_sample(&self, factor: u8) -> DownSample {
        DownSample::new(factor)
    }
}

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
            value += modulator.borrow_mut().tick(count);
        }
        value
    }
}

pub enum WaveType {
    Sine,
    Triangle,
    Square,
    Saw,
}
pub struct Oscillator {
    wave_type: WaveType,
    pub frequency: Parameter,
    pub amplitude: Parameter,
    step: u64,
    sample_rate: u64,

    multi_hold: Mono,
    multi_index: usize,
}
impl Oscillator {
    fn new(
        wave_type: WaveType,
        frequency: Frequency,
        amplitude: f64,
        sample_rate: u64
        ) -> Self {
        Self {
            wave_type,
            frequency: Parameter::new(frequency),
            amplitude: Parameter::new(amplitude),
            sample_rate,
            step: 0,
            multi_hold: 0.,
            multi_index: 0,
        }
    }
}
impl DSPGenMono for Oscillator {
    fn tick(&mut self, nb_connected: usize) -> Mono {
        self.multi_index += 1;
        if self.multi_index >= nb_connected {
            self.multi_index = 0;
        }
        if self.multi_index != 0 {
            return self.multi_hold;
        }

        let amplitude = self.amplitude.real_value();
        let frequency = self.frequency.real_value();

        let nb_samples_cycle = (self.sample_rate as f64 / frequency) as u64;
        let new_sample = match self.wave_type {
            WaveType::Sine => (self.step as f64 * 2f64 * std::f64::consts::PI / self.sample_rate as f64 * frequency).sin(),
            WaveType::Square =>
                if self.step < nb_samples_cycle / 2 { 1. }
                else { -1. },
            WaveType::Saw => (2. / nb_samples_cycle as f64) * self.step as f64 - 1.,
            WaveType::Triangle => if self.step < nb_samples_cycle / 2 {
                (2. / (nb_samples_cycle as f64 / 2.)) * self.step as f64 - 1.
            } else {
                (-2. / (nb_samples_cycle as f64 / 2.)) * (self.step as f64 - nb_samples_cycle as f64) - 1.
            },
        };
        self.step += 1;
        if self.step >= nb_samples_cycle {
            self.step = 0;
        }
        self.multi_hold = new_sample * amplitude;
        self.multi_hold
    }
}

pub enum ADSRState {
    Attack(u64),
    Decay(u64),
    Sustain,
    Release(u64),
    Off,
}
pub struct ADSR {
    pub state: ADSRState,
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
            multi_hold: 0.,
            multi_index: 0,
        }
    }
}
impl DSPGenMono for ADSR {
    fn tick(&mut self, nb_connected: usize) -> Mono {
        self.multi_index += 1;
        if self.multi_index >= nb_connected {
            self.multi_index = 0;
        }
        if self.multi_index != 0 {
            return self.multi_hold;
        }

        self.multi_hold = match self.state {
            ADSRState::Attack(step) => {
                let nb_samples = (self.attack as f64 / 1000. * self.sample_rate as f64).round();
                let new_sample = self.peak * (step as f64).powf(self.attack_curve) / nb_samples.powf(self.attack_curve);
                self.state = if step as f64 >= nb_samples {
                    ADSRState::Decay(0)
                } else {
                    ADSRState::Attack(step + 1)
                };
                new_sample
            },
            ADSRState::Decay(step) => {
                let nb_samples = (self.decay as f64 / 1000. * self.sample_rate as f64).round();
                let new_sample = -(self.peak - self.sustain) * (step as f64).powf(self.decay_curve) / nb_samples.powf(self.decay_curve) + self.peak;
                self.state = if step as f64 >= nb_samples {
                    ADSRState::Sustain
                } else {
                    ADSRState::Decay(step + 1)
                };
                new_sample
            },
            ADSRState::Sustain => self.sustain,
            ADSRState::Release(step) => {
                let nb_samples = (self.release as f64 / 1000. * self.sample_rate as f64).round();
                let new_sample = -self.sustain * (step as f64).powf(self.release_curve) / nb_samples.powf(self.release_curve) + self.sustain;
                self.state = if step as f64 >= nb_samples {
                    ADSRState::Off
                } else {
                    ADSRState::Release(step + 1)
                };
                new_sample
            },
            ADSRState::Off => 0.,
        };
        self.multi_hold
    }
}

pub struct DownSample {
    factor: u8,
    hold: f64,
    step: u8,
}
impl DownSample {
    fn new(factor: u8) -> Self {
        Self {
            factor,
            hold: 0.,
            step: 0,
        }
    }
}
impl DSPFxMono for DownSample {
    fn tick(&mut self, sample: Mono) -> Mono {
        if self.step % self.factor == 0 {
            self.hold = sample;
            self.step = 0;
        }
        self.step += 1;
        self.hold
    }
}
