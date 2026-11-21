//! Procedural audio of the Project 3 ship, synthesized at start-up (no
//! sound files, no new dependency): a low thruster rumble and a light-speed
//! whoosh that loop with their levels following the ship's thrust and
//! boost, and a rising chirp for the tangential escape. The samples are
//! plain 16-bit PCM wrapped as WAV in memory, played by raylib's own audio
//! device in the live viewer (`M` mutes it).

use std::f32::consts::TAU;

/// Samples per second of every sound.
pub const SAMPLE_RATE: u32 = 22_050;

fn hash01(i: u32, seed: u32) -> f32 {
    let mut h = i.wrapping_mul(0x9E37_79B9) ^ seed.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// One second of `partials` sines between `lo` and `hi` Hz, each a whole
/// number of cycles with a seeded phase and a 1/f share, so the second
/// loops without a click.
fn partial_loop(lo: u32, hi: u32, partials: u32, seed: u32) -> Vec<f32> {
    let n = SAMPLE_RATE as usize;
    let mut out = vec![0.0f32; n];
    for k in 0..partials {
        let f = lo + ((hi - lo) as f32 * hash01(k, seed)) as u32;
        let phase = TAU * hash01(k, seed ^ 0xABCD);
        let amp = 1.0 / (f as f32).sqrt();
        for (i, s) in out.iter_mut().enumerate() {
            *s += amp * (TAU * f as f32 * i as f32 / n as f32 + phase).sin();
        }
    }
    out
}

/// Scales `x` so its peak is `peak` and converts it to 16-bit samples.
fn to_pcm(x: &[f32], peak: f32) -> Vec<i16> {
    let max = x.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-9);
    x.iter()
        .map(|v| (v / max * peak * i16::MAX as f32) as i16)
        .collect()
}

/// The thruster rumble: a 55 Hz hum and its harmonics over a band of
/// low partials, swelling four times a second. One looping second.
pub fn thruster_loop() -> Vec<i16> {
    let noise = partial_loop(60, 420, 48, 7);
    let n = noise.len() as f32;
    let x: Vec<f32> = noise
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let t = i as f32 / n;
            let hum = (TAU * 55.0 * t).sin() + 0.5 * (TAU * 110.0 * t).sin();
            let swell = 0.85 + 0.15 * (TAU * 4.0 * t).sin();
            (0.6 * v + 0.25 * hum) * swell
        })
        .collect();
    to_pcm(&x, 0.7)
}

/// The light-speed whoosh: a bright band of partials with a slow shimmer.
/// One looping second.
pub fn boost_loop() -> Vec<i16> {
    let band = partial_loop(300, 2200, 64, 11);
    let n = band.len() as f32;
    let x: Vec<f32> = band
        .iter()
        .enumerate()
        .map(|(i, v)| v * (0.75 + 0.25 * (TAU * 3.0 * i as f32 / n).sin()))
        .collect();
    to_pcm(&x, 0.55)
}

/// Length of the escape chirp, seconds.
pub const ESCAPE_SECONDS: f32 = 0.9;

/// The tangential escape: a sweep from 180 to 900 Hz with a quick attack
/// and a long fade, over a soft fifth.
pub fn escape_chirp() -> Vec<i16> {
    let n = (ESCAPE_SECONDS * SAMPLE_RATE as f32) as usize;
    let mut phase = 0.0f32;
    let x: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let f = 180.0 * 5f32.powf(t);
            phase += TAU * f / SAMPLE_RATE as f32;
            let env = (t / 0.03).min(1.0) * (1.0 - t).powi(2);
            env * (phase.sin() + 0.35 * (1.5 * phase).sin())
        })
        .collect();
    to_pcm(&x, 0.8)
}

/// `samples` as a mono 16-bit PCM WAV file at `SAMPLE_RATE`.
pub fn wav_bytes(samples: &[i16]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Volumes of the two loops for a ship at `thrust` and `boost` (both in
/// [0, 1]): a faint idle hum that grows with thrust, the whoosh only with
/// the boost. Muted: both 0.
pub fn loop_volumes(thrust: f32, boost: f32, enabled: bool) -> (f32, f32) {
    if !enabled {
        return (0.0, 0.0);
    }
    let unit = |v: f32| {
        if v.is_finite() {
            v.clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    (0.12 + 0.5 * unit(thrust), 0.45 * unit(boost))
}
