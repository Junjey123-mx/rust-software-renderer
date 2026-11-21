//! Procedural audio of the Project 3 ship (expansion): sounds synthesized
//! in code (no sound files, no new dependency), wrapped as WAV in memory
//! for raylib's audio device in the live viewer, and muted with `M`. The
//! device itself is not opened here: these tests cover what is played.

use raylib::consts::KeyboardKey;
use rust_software_renderer::solar::audio::{
    ESCAPE_SECONDS, SAMPLE_RATE, boost_loop, escape_chirp, loop_volumes, thruster_loop, wav_bytes,
};
use rust_software_renderer::solar::input::{PRESSED, SolarAction};

fn peak(x: &[i16]) -> i32 {
    x.iter().map(|&s| i32::from(s).abs()).max().unwrap()
}

fn largest_step(x: &[i16]) -> i32 {
    x.windows(2)
        .map(|w| (i32::from(w[1]) - i32::from(w[0])).abs())
        .max()
        .unwrap()
}

#[test]
fn loops_are_one_seamless_second() {
    for x in [thruster_loop(), boost_loop()] {
        assert_eq!(x.len(), SAMPLE_RATE as usize);
        assert!(peak(&x) > 10_000 && peak(&x) < i32::from(i16::MAX));
        // Wrapping around is no bigger a jump than any step inside.
        let wrap = (i32::from(x[0]) - i32::from(*x.last().unwrap())).abs();
        assert!(wrap <= largest_step(&x), "{wrap}");
    }
    // Deterministic, and two different sounds.
    assert_eq!(thruster_loop(), thruster_loop());
    assert_ne!(thruster_loop(), boost_loop());
}

/// Sign changes in `x`.
fn crossings(x: &[i16]) -> usize {
    x.windows(2).filter(|w| (w[0] < 0) != (w[1] < 0)).count()
}

#[test]
fn escape_chirp_rises_and_fades() {
    let x = escape_chirp();
    assert_eq!(x.len(), (ESCAPE_SECONDS * SAMPLE_RATE as f32) as usize);
    let q = x.len() / 4;
    // Higher pitch at the end than at the start.
    assert!(crossings(&x[3 * q..]) > 2 * crossings(&x[..q]));
    // Soft start and end, loud in between.
    assert!(i32::from(x[0]).abs() < 500);
    assert!(peak(&x[x.len() - 200..]) < 500);
    assert!(peak(&x) > 20_000);
}

#[test]
fn wav_header_describes_the_samples() {
    let pcm = escape_chirp();
    let wav = wav_bytes(&pcm);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..16], b"WAVEfmt ");
    assert_eq!(&wav[36..40], b"data");
    let u32_at = |i: usize| u32::from_le_bytes(wav[i..i + 4].try_into().unwrap());
    let u16_at = |i: usize| u16::from_le_bytes(wav[i..i + 2].try_into().unwrap());
    assert_eq!(u32_at(4) as usize, wav.len() - 8);
    assert_eq!((u16_at(20), u16_at(22)), (1, 1), "PCM, mono");
    assert_eq!(u32_at(24), SAMPLE_RATE);
    assert_eq!(u16_at(34), 16);
    assert_eq!(u32_at(40) as usize, 2 * pcm.len());
    assert_eq!(i16::from_le_bytes([wav[44], wav[45]]), pcm[0]);
}

#[test]
fn volumes_follow_thrust_boost_and_mute() {
    assert_eq!(loop_volumes(1.0, 1.0, false), (0.0, 0.0));
    let (idle, quiet) = loop_volumes(0.0, 0.0, true);
    assert!(idle > 0.0 && quiet == 0.0, "idle hum, no whoosh");
    let (full, whoosh) = loop_volumes(1.0, 1.0, true);
    assert!(full > idle && whoosh > 0.0 && full <= 1.0);
    assert_eq!(
        loop_volumes(f32::NAN, 5.0, true),
        (idle, loop_volumes(0.0, 1.0, true).1)
    );
}

#[test]
fn m_toggles_the_sound() {
    assert!(PRESSED.contains(&(KeyboardKey::KEY_M, SolarAction::ToggleAudio)));
}
