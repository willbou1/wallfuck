use std::fs::{File, OpenOptions};
use std::io::{Cursor, Write};
use std::io;
use byteorder::{LittleEndian, WriteBytesExt};
use super::dsp::{DSP, DownSample};

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

    let mut redux = DownSample::new(10);

    for i in 0..88200 {
        let tremolo = (i as f64 * 2f64 * std::f64::consts::PI / 44100f64 * 3f64).sin() * 0.2f64;
        let c = (i as f64 * 2f64 * std::f64::consts::PI / 44100f64 * (261.6256 + tremolo)).sin();
        let e = (i as f64 * 2f64 * std::f64::consts::PI / 44100f64 * (329.6276 + tremolo)).sin();
        let g = (i as f64 * 2f64 * std::f64::consts::PI / 44100f64 * (391.9954 + tremolo)).sin();
        let h = (i as f64 * 2f64 * std::f64::consts::PI / 44100f64 * (493.8833 + tremolo)).sin();
        let sample = redux.tick((c + e + g + h) / 4f64);
        let adsr = if i < 8820 {
            (1f64 / 8820f64) * i as f64 * sample
        } else if i < 44100 {
            sample
        } else {
            (-(1f64 / 8820f64) * (i as f64 - 44100f64) + 1f64).max(0f64) * sample
        };
        let processed_sample = if adsr >= 0f64 {
            (adsr * std::i16::MAX as f64 * 0.5) as i16
        } else {
            (adsr * -(std::i16::MIN as f64) * 0.5) as i16
        };
        cursor.write_i16::<LittleEndian>(processed_sample)?; // sample
        cursor.write_i16::<LittleEndian>(processed_sample)?; // sample
    }

    let mut file = OpenOptions::new().write(true).create(true).open("test.wav")?;
    file.write(&buffer)?;
    
    Ok(())
}
