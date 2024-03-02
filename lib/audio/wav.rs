use std::rc::Rc;
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Write};
use std::io;
use byteorder::{LittleEndian, WriteBytesExt};
use super::dsp::{DSPBuilder, DSPGenMono, DSPFxMono, DownSample, Oscillator, Frequency, Mono, WaveType, ADSRState};

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

    let dsp_builder = DSPBuilder::new(44100);
    let adsr = dsp_builder.build_adsr(
        200, 0.5,
        0.1,
        100, 0.5,
        0.08,
        100, 0.5);
    adsr.borrow_mut().state = ADSRState::Attack(0);
    let redux = dsp_builder.build_down_sample(8);
    let modulator = dsp_builder.build_oscillator(WaveType::Sine, 4., 20.);

    let d = dsp_builder.build_oscillator(WaveType::Sine, 261.6256, 0.);
    d.borrow_mut().frequency.add_modulator(modulator.clone());
    d.borrow_mut().amplitude.add_modulator(adsr.clone());
    let e = dsp_builder.build_oscillator(WaveType::Sine, 329.6276, 0.);
    e.borrow_mut().frequency.add_modulator(modulator.clone());
    e.borrow_mut().amplitude.add_modulator(adsr.clone());
    let g = dsp_builder.build_oscillator(WaveType::Sine, 391.9954, 0.);
    g.borrow_mut().frequency.add_modulator(modulator.clone());
    g.borrow_mut().amplitude.add_modulator(adsr.clone());
    let h = dsp_builder.build_oscillator(WaveType::Sine, 493.8833, 0.);
    h.borrow_mut().frequency.add_modulator(modulator.clone());
    h.borrow_mut().amplitude.add_modulator(adsr.clone());

    for i in 0..88200 {
        let sample = d.borrow_mut().tick(1) + e.borrow_mut().tick(1) + g.borrow_mut().tick(1) + h.borrow_mut().tick(1);
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
