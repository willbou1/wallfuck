use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;
use std::cmp;
use rand::{thread_rng, Rng};
use rand::rngs::ThreadRng;

//==============================================================================
// Framework details
//==============================================================================
pub type Frequency = f64;
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
    pub fn build_fx_chain(&self) -> Rc<RefCell<FxChain>> {
        Rc::new(RefCell::new(FxChain::new()))
    }
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
        amplitude: f64
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
        Rc::new(RefCell::new(ADSR::new(attack, attack_curve, peak, decay, decay_curve, sustain, release, release_curve, self.sample_rate)))
    }

    pub fn build_down_sample(&self, factor: u8) -> Rc<RefCell<DownSample>> {
        Rc::new(RefCell::new(DownSample::new(factor)))
    }
    pub fn build_filter(&self,
        kind: FilterKind,
        order: FilterOrder,
        cut_off: Frequency
        ) -> Rc<RefCell<Filter>>
    {
        Rc::new(RefCell::new(Filter::new(kind, order, cut_off, self.sample_rate)))
    }
    pub fn build_moving_average(&self,
        window_size: usize
        ) -> Rc<RefCell<MovingAverage>>
    {
        Rc::new(RefCell::new(MovingAverage::new(window_size)))
    }
    pub fn build_absolute(&self) -> Rc<RefCell<Absolute>>
    {
        Rc::new(RefCell::new(Absolute::new()))
    }
    pub fn build_operator<F>(&self,
        operator: F
        ) -> Rc<RefCell<Operator<F>>> where
        F: Fn(f64) -> f64
    {
        Rc::new(RefCell::new(Operator::new(operator)))
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
            value += modulator.borrow_mut().tick(count);
        }
        value
    }
}



//==============================================================================
// Complex singal generators
//==============================================================================
pub struct Chain {
    module: Rc<RefCell<dyn DSPGenMono>>,
    pub fx_chain: FxChain,

    multi_hold: Mono,
    multi_index: usize,
}
impl Chain {
    pub fn new(module: Rc<RefCell<dyn DSPGenMono>>) -> Self {
        Self {
            module,
            fx_chain: FxChain::new(),
            multi_hold: 0.,
            multi_index: 0,
        }
    }
}
impl DSPGenMono for Chain {
    fn tick(&mut self, nb_connected: usize) -> Mono {
        self.multi_index += 1;
        if self.multi_index >= nb_connected {
            self.multi_index = 0;
        }
        if self.multi_index != 0 {
            return self.multi_hold;
        }

        let mut count = Rc::strong_count(&self.module);
        if count > 1 {
            count -= 1;
        }
        self.multi_hold = self.module.borrow_mut().tick(count);
        self.multi_hold = self.fx_chain.tick(self.multi_hold);
        self.multi_hold
    }
}

//==============================================================================
pub struct Parallel {
    modules: Vec<Rc<RefCell<dyn DSPGenMono>>>,

    multi_hold: Mono,
    multi_index: usize,
}
impl Parallel {
    fn new() -> Self {
        Self {
            modules: vec![],
            multi_hold: 0.,
            multi_index: 0,
        }
    }
    pub fn add(&mut self, module: Rc<RefCell<dyn DSPGenMono>>) {
        self.modules.push(module);
    }
}
impl DSPGenMono for Parallel {
    fn tick(&mut self, nb_connected: usize) -> Mono {
        self.multi_index += 1;
        if self.multi_index >= nb_connected {
            self.multi_index = 0;
        }
        if self.multi_index != 0 {
            return self.multi_hold;
        }

        self.multi_hold = 0.;
        for module in &self.modules {
            let mut count = Rc::strong_count(module);
            if count > 1 {
                count -= 1;
            }

            self.multi_hold += module.borrow_mut().tick(count);
        }
        self.multi_hold != self.modules.len() as f64;
        self.multi_hold
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
            rng: thread_rng(),
        }
    }
}
impl DSPGenMono for Noise {
    fn tick(&mut self, nb_connected: usize) -> Mono {
        self.multi_index += 1;
        if self.multi_index >= nb_connected {
            self.multi_index = 0;
        }
        if self.multi_index != 0 {
            return self.multi_hold;
        }

        let amplitude = self.amplitude.real_value();

        self.multi_hold = match self.kind {
            NoiseKind::White => if amplitude != 0. {
                self.rng.gen_range(-amplitude..amplitude)
            } else { 0. },
        };
        self.multi_hold
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
        self.multi_hold = match self.kind {
            WaveKind::Sine => (self.step as f64 * 2f64 * std::f64::consts::PI / self.sample_rate as f64 * frequency).sin(),
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
        self.multi_hold
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

        let nb_samples_ms = |ms: u64| (ms as f64 / 1000. * self.sample_rate as f64).round();
        self.multi_hold = match self.state {
            ADSRState::Attack(step) => {
                let nb_samples = nb_samples_ms(self.attack);
                let new_sample = self.peak * (step as f64).powf(self.attack_curve) / nb_samples.powf(self.attack_curve);
                self.state = if step as f64 >= nb_samples {
                    ADSRState::Decay(0)
                } else {
                    ADSRState::Attack(step + 1)
                };
                new_sample
            },
            ADSRState::Decay(step) => {
                let nb_samples = nb_samples_ms(self.decay);
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
                let nb_samples = nb_samples_ms(self.release);
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



//==============================================================================
// Complex mono effects
//==============================================================================
pub struct FxChain {
    effects: VecDeque<Rc<RefCell<dyn DSPFxMono>>>,
}
impl FxChain {
    pub fn new() -> Self {
        Self {
            effects: VecDeque::new(),
        }
    }
    pub fn insert(&mut self, effect: Rc<RefCell<dyn DSPFxMono>>) {
        self.effects.push_front(effect);
    }
    pub fn append(&mut self, effect: Rc<RefCell<dyn DSPFxMono>>) {
        self.effects.push_back(effect);
    }
}
impl DSPFxMono for FxChain {
    fn tick(&mut self, mut sample: Mono) -> Mono {
        for effect in &self.effects {
            sample = effect.borrow_mut().tick(sample);
        }
        sample
    }
}



//==============================================================================
// Simple mono effects
//==============================================================================
pub struct Absolute {}
impl Absolute {
    pub fn new() -> Self {
        Self {}
    }
}
impl DSPFxMono for Absolute {
    fn tick(&mut self, sample: f64) -> f64 {
        sample.abs()
    }
}

//==============================================================================
pub struct Operator<F> {
    operator: F,
}
impl<F> Operator<F> where
    F: Fn(f64) -> f64,
{
    pub fn new(operator: F) -> Self {
        Self { operator }
    }
}
impl<F> DSPFxMono for Operator<F> where
    F: Fn(f64) -> f64,
{
    fn tick(&mut self, sample: f64) -> f64 {
        (self.operator)(sample)
    }
}

//==============================================================================
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

//==============================================================================
pub enum FilterKind {
    AllPass,
    LowPass,
    HighPass,
}
pub enum FilterOrder {
    First,
}
pub struct Filter {
    kind: FilterKind,
    order: FilterOrder,
    pub cut_off: Parameter,
    old_cut_off: f64,
    coefficient: f64,
    buffer: f64,
    sample_rate: u64,
}
impl Filter {
    fn new(kind: FilterKind, order: FilterOrder, cut_off: Frequency, sample_rate: u64) -> Self {
        Self {
            kind,
            order,
            sample_rate,
            old_cut_off: cut_off + 1.,
            coefficient: 0.,
            buffer: 0.,
            cut_off: Parameter::new(cut_off),
        }
    }
}
impl DSPFxMono for Filter {
    fn tick(&mut self, sample: Mono) -> Mono {
        let cut_off = self.cut_off.real_value();
        if cut_off != self.old_cut_off {
            self.old_cut_off = cut_off;
            self.coefficient = match self.order {
                FilterOrder::First => {
                    let tan = (std::f64::consts::PI * cut_off / self.sample_rate as f64).tan();
                    (tan - 1.) / (tan + 1.)
                },
            };
        }

        let all_pass = match self.order {
            FilterOrder::First =>
                self.coefficient * sample + self.buffer,
        };
        self.buffer = sample - self.coefficient * all_pass;
        match self.kind {
            FilterKind::AllPass => all_pass,
            FilterKind::HighPass => (sample - all_pass) / 2.,
            FilterKind::LowPass => (sample + all_pass) / 2.,
        }
    }
}

//==============================================================================
pub struct MovingAverage {
    window_size: usize,
    processed: usize,
    index: usize,
    buffer: Vec<f64>,
}
impl MovingAverage {
    fn new(window_size: usize) -> Self{
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
impl DSPFxMono for MovingAverage {
    fn tick(&mut self, sample: Mono) -> Mono {
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

