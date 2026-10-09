//! A small synthesized chime; no network, decoder or resident audio engine.
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};

fn wave() -> &'static [u8] {
    static WAVE: OnceLock<Vec<u8>> = OnceLock::new();
    WAVE.get_or_init(|| {
        let rate = 22050_u32;
        let count = (rate as f32 * 0.72) as usize;
        let size = (count * 2) as u32;
        let mut bytes = Vec::with_capacity(44 + size as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + size).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * 2).to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&size.to_le_bytes());
        for n in 0..count {
            let time = n as f32 / rate as f32;
            let mut sample = 0.;
            for (start, frequency) in [(0., 659.25), (0.13, 830.61), (0.26, 987.77)] {
                let t = time - start;
                if t >= 0. {
                    let envelope = (t / 0.015).min(1.)
                        * (-8. * t).exp()
                        * ((0.72 - time) / 0.05).clamp(0., 1.);
                    sample += (std::f32::consts::TAU * frequency * t).sin() * envelope * 0.13;
                }
            }
            bytes.extend_from_slice(
                &((sample.clamp(-1., 1.) * i16::MAX as f32) as i16).to_le_bytes(),
            );
        }
        bytes
    })
}

pub fn play() {
    static PLAYING: AtomicBool = AtomicBool::new(false);
    if PLAYING.swap(true, Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(|| {
        play_wave(wave());
        PLAYING.store(false, Ordering::Relaxed);
    });
}

#[cfg(windows)]
fn play_wave(bytes: &[u8]) {
    use windows_sys::Win32::Media::Audio::{PlaySoundA, SND_MEMORY, SND_NODEFAULT, SND_SYNC};
    // SAFETY: generated PCM WAV stays alive throughout this synchronous call,
    // which runs on a background thread and receives no external sound data.
    unsafe {
        PlaySoundA(
            bytes.as_ptr(),
            std::ptr::null_mut(),
            SND_MEMORY | SND_SYNC | SND_NODEFAULT,
        );
    }
}

#[cfg(target_os = "linux")]
fn play_wave(bytes: &[u8]) {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    // Desktop PulseAudio/PipeWire or ALSA helpers consume the embedded WAV via
    // stdin. Optional helpers never block the interface or write a temp file.
    for (program, args) in [
        ("paplay", vec!["--client-name=Whakoom Desktop"]),
        ("aplay", vec!["--quiet"]),
    ] {
        if let Ok(mut child) = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            let written = child
                .stdin
                .take()
                .is_some_and(|mut input| input.write_all(bytes).is_ok());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if status.success() && written {
                            return;
                        }
                        break;
                    }
                    Ok(None) if std::time::Instant::now() < deadline => {
                        std::thread::sleep(std::time::Duration::from_millis(20))
                    }
                    _ => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                }
            }
        }
    }
}
#[cfg(not(any(windows, target_os = "linux")))]
fn play_wave(_: &[u8]) {}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_chime_is_valid_bounded_pcm_and_fades_out() {
        let bytes = super::wave();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize,
            bytes.len() - 44
        );
        let samples = bytes[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|s| i16::from_le_bytes([s[0], s[1]]))
            .collect::<Vec<_>>();
        assert!(samples.iter().any(|s| s.abs() > 1000));
        assert!(samples.iter().all(|s| s.abs() < 15000));
        assert_eq!(samples[0], 0);
        assert!(samples.last().unwrap().abs() < 10);
    }
}
