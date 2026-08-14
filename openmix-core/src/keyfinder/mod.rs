//! Unsafe FFI to the vendored libkeyfinder; wrapped in a tiny safe API.
//! NOTE: libkeyfinder is GPL-3.0-or-later — see vendor/NOTICE.

use crate::analysis::key::{KeyAlgorithm, KeyResult, MusicalKey};

// Safety contract of `openmix_kf_detect` (implemented in shim.cpp):
// - `samples` must be a valid pointer to `n` readable f32 values.
// - `key_out`/`conf_out` must be valid writable pointers.
// - The shim performs no memory allocation across the FFI boundary and
//   catches all C++ exceptions internally, returning -1 on failure.
#[link(name = "openmix_kf", kind = "static")]
unsafe extern "C" {
    fn openmix_kf_detect(
        samples: *const f32,
        n: usize,
        rate: u32,
        key_out: *mut i32,
        conf_out: *mut f32,
    ) -> i32;
}

/// Detect the musical key of a mono sample buffer using libkeyfinder.
/// Returns `None` if analysis fails (empty input, silence, or the shim
/// reports an error).
pub fn detect_key(samples: &[f32], rate: u32) -> Option<KeyResult> {
    if samples.is_empty() {
        return None;
    }
    let mut key_out: i32 = -1;
    let mut conf_out: f32 = 0.0;
    // SAFETY: samples.as_ptr() is valid for samples.len() reads; both
    // out-pointers point to valid stack slots; shim never writes elsewhere.
    let rc = unsafe {
        openmix_kf_detect(
            samples.as_ptr(),
            samples.len(),
            rate,
            &mut key_out,
            &mut conf_out,
        )
    };
    if rc != 0 || key_out < 0 {
        return None;
    }
    let key = MusicalKey::from_keyfinder_index(key_out)?;
    Some(KeyResult {
        key,
        confidence: conf_out.clamp(0.0, 1.0),
        algorithm: KeyAlgorithm::KeyFinder,
        alternate: None,
    })
}

#[cfg(test)]
mod tests {
    use super::detect_key;
    use crate::analysis::key::MusicalKey;

    fn c_major_pad(rate: u32, seconds: f64) -> Vec<f32> {
        let n = (rate as f64 * seconds) as usize;
        let attack = (rate as f64 * 0.010) as usize;
        let freqs = [261.63f64, 329.63, 392.00];
        (0..n)
            .map(|i| {
                let t = i as f64 / rate as f64;
                let mut v = 0.0f64;
                for f in freqs {
                    v += 0.2 * (2.0 * std::f64::consts::PI * f * t).sin();
                }
                let ramp = if i < attack {
                    i as f64 / attack as f64
                } else {
                    1.0
                };
                (v * ramp) as f32
            })
            .collect()
    }

    #[test]
    fn keyfinder_detects_c_major_pad() {
        let mono = c_major_pad(44100, 6.0);
        let k = detect_key(&mono, 44100).expect("detect");
        assert!(
            k.key == MusicalKey::CMajor || k.key == MusicalKey::AMinor,
            "key {:?}",
            k.key
        );
        assert!(k.confidence > 0.0);
    }

    #[test]
    fn keyfinder_detect_requires_signal() {
        assert!(detect_key(&[], 44100).is_none());
    }
}
