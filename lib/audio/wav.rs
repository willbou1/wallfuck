use std::rc::Rc;
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Write};
use std::io;
use byteorder::{LittleEndian, WriteBytesExt};
use super::dsp::{DSPBuilder, DSPGenMono, DSPFxMono, DownSample, Oscillator, Frequency, Mono, WaveType, ADSRState, Parallel, FxChain, Chain, MovingAverage};

pub fn write_test_wav() -> io::Result<()> {
    let mut buffer = vec![];
    let mut cursor = Cursor::new(&mut buffer);

    // RIFF header (little endian)
    cursor.write(b"RIFF")?; // ChunkID
    cursor.write_u32::<LittleEndian>(36 + 88200 * 2 * 16 / 8)?; //ChunkSize
    cursor.write(b"WAVE")?;

    // fmt chunk (PCM)
    cursor.write(b"fmt ")?; // Subchunk1ID
    cursor.write_u32::<LittleEndian>(16)?; // Subchunk1Sizee
    cursor.write_u16::<LittleEndian>(1)?; // AudioFormat
    cursor.write_u16::<LittleEndian>(2)?; // NumChannels (stereo)
    cursor.write_u32::<LittleEndian>(44100)?; // SampleRate
    cursor.write_u32::<LittleEndian>(44100 * 2 * 16 / 8)?; // ByteRate
    cursor.write_u16::<LittleEndian>(2 * 16 / 8)?; //BlockAlign
    cursor.write_u16::<LittleEndian>(16)?; // BitsPerSample

    // data chunk
    cursor.write(b"data")?; // Subchunk2ID
    cursor.write_u32::<LittleEndian>(88200 * 2 * 16 / 8)?; // Subchunk2Size

    // use dsp lib to render sound
    let dsp_builder = DSPBuilder::new(44100);
    let adsr = dsp_builder.build_adsr(
        200, 0.5,
        0.4,
        100, 0.5,
        0.32,
        100, 0.5);
    adsr.borrow_mut().state = ADSRState::Attack(0);

    let sine = dsp_builder.build_oscillator(WaveType::Sine, 261.6256, 0.);
    sine.borrow_mut().amplitude.add_modulator(adsr.clone());
    let triangle = dsp_builder.build_oscillator(WaveType::Triangle, 261.6256, 0.);
    triangle.borrow_mut().amplitude.add_modulator(adsr.clone());

    let parallel = dsp_builder.build_parallel();
    parallel.borrow_mut().add(sine.clone());
    parallel.borrow_mut().add(triangle.clone());

    let chain = dsp_builder.build_chain(parallel.clone());
    let moving_average = dsp_builder.build_moving_average(100);
    chain.borrow_mut().fx_chain.insert(moving_average.clone());

    for i in 0..88200 {
        let sample = chain.borrow_mut().tick(1);
        if i == 83000 {
            adsr.borrow_mut().state = ADSRState::Release(0);
        }
        let processed_sample = if sample >= 0f64 {
            (sample * std::i16::MAX as f64 * 0.5) as i16
        } else {
            (sample * -(std::i16::MIN as f64) * 0.5) as i16
        };
        cursor.write_i16::<LittleEndian>(processed_sample)?; // sample
        cursor.write_i16::<LittleEndian>(processed_sample)?; // sample
    }

    let mut file = OpenOptions::new().write(true).create(true).open("test.wav")?;
    file.write(&buffer)?;
    
    Ok(())
}
