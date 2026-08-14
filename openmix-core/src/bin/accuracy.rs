use openmix_core::analysis::{analyze_path, AnalysisConfig, MusicalKey};
use openmix_core::error::AppError;
use std::path::{Path, PathBuf};

struct Fixture {
    file: &'static str,
    bpm: Option<f64>,
    key: Option<MusicalKey>,
    offset_s: Option<f64>,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        file: "kick_70bpm.wav",
        bpm: Some(70.0),
        key: None,
        offset_s: None,
    },
    Fixture {
        file: "kick_87bpm.wav",
        bpm: Some(87.0),
        key: None,
        offset_s: None,
    },
    Fixture {
        file: "kick_100bpm.wav",
        bpm: Some(100.0),
        key: None,
        offset_s: None,
    },
    Fixture {
        file: "kick_120bpm.wav",
        bpm: Some(120.0),
        key: None,
        offset_s: Some(0.0),
    },
    Fixture {
        file: "kick_120bpm.flac",
        bpm: Some(120.0),
        key: None,
        offset_s: Some(0.0),
    },
    Fixture {
        file: "kick_120bpm.mp3",
        bpm: Some(120.0),
        key: None,
        offset_s: Some(0.0),
    },
    Fixture {
        file: "kick_120bpm_stereo.wav",
        bpm: Some(120.0),
        key: None,
        offset_s: Some(0.0),
    },
    Fixture {
        file: "kick_120bpm_intro.wav",
        bpm: Some(120.0),
        key: None,
        offset_s: Some(0.87),
    },
    Fixture {
        file: "kick_128bpm.wav",
        bpm: Some(128.0),
        key: None,
        offset_s: None,
    },
    Fixture {
        file: "kick_140bpm.wav",
        bpm: Some(140.0),
        key: None,
        offset_s: None,
    },
    Fixture {
        file: "kick_174bpm.wav",
        bpm: Some(174.0),
        key: None,
        offset_s: None,
    },
    Fixture {
        file: "kick_180bpm.wav",
        bpm: Some(180.0),
        key: None,
        offset_s: None,
    },
    Fixture {
        file: "pad_A_major.wav",
        bpm: None,
        key: Some(MusicalKey::AMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_A#_major.wav",
        bpm: None,
        key: Some(MusicalKey::ASharpMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_B_major.wav",
        bpm: None,
        key: Some(MusicalKey::BMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_C_major.wav",
        bpm: None,
        key: Some(MusicalKey::CMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_C#_major.wav",
        bpm: None,
        key: Some(MusicalKey::CSharpMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_D_major.wav",
        bpm: None,
        key: Some(MusicalKey::DMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_D#_major.wav",
        bpm: None,
        key: Some(MusicalKey::DSharpMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_E_major.wav",
        bpm: None,
        key: Some(MusicalKey::EMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_F_major.wav",
        bpm: None,
        key: Some(MusicalKey::FMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_F#_major.wav",
        bpm: None,
        key: Some(MusicalKey::FSharpMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_G_major.wav",
        bpm: None,
        key: Some(MusicalKey::GMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_G#_major.wav",
        bpm: None,
        key: Some(MusicalKey::GSharpMajor),
        offset_s: None,
    },
    Fixture {
        file: "pad_A_minor.wav",
        bpm: None,
        key: Some(MusicalKey::AMinor),
        offset_s: None,
    },
    Fixture {
        file: "pad_C_minor.wav",
        bpm: None,
        key: Some(MusicalKey::CMinor),
        offset_s: None,
    },
    Fixture {
        file: "pad_D_minor.wav",
        bpm: None,
        key: Some(MusicalKey::DMinor),
        offset_s: None,
    },
    Fixture {
        file: "pad_E_minor.wav",
        bpm: None,
        key: Some(MusicalKey::EMinor),
        offset_s: None,
    },
    Fixture {
        file: "pad_F_minor.wav",
        bpm: None,
        key: Some(MusicalKey::FMinor),
        offset_s: None,
    },
    Fixture {
        file: "pad_G_minor.wav",
        bpm: None,
        key: Some(MusicalKey::GMinor),
        offset_s: None,
    },
];

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn pct(passed: usize, total: usize) -> f64 {
    passed as f64 / total as f64 * 100.0
}

fn main() -> Result<(), AppError> {
    let mut bpm_pass = 0usize;
    let mut key_pass = 0usize;
    let mut offset_pass = 0usize;
    let mut bpm_total = 0usize;
    let mut key_total = 0usize;
    let mut offset_total = 0usize;

    println!(
        "{:<26} {:>9} {:>9} {:>13} {:>13} {:>9} {:>9}",
        "fixture", "bpm", "bpm_got", "key", "key_got", "offset", "offset_got"
    );
    for f in FIXTURES {
        let r = analyze_path(fixtures_dir().join(f.file), &AnalysisConfig::default());
        let (got_bpm, got_key, got_offset) = r
            .as_ref()
            .map(|a| {
                (
                    a.bpm.map(|b| format!("{b:.2}")),
                    a.key.as_ref().map(|k| format!("{:?}", k.key)),
                    a.grid
                        .as_ref()
                        .map(|g| format!("{:.3}", g.first_beat_offset)),
                )
            })
            .unwrap_or_else(|e| (Some(format!("ERR {e}")), None, None));

        let bpm_ok = match (r.as_ref().ok(), f.bpm) {
            (Some(a), Some(exp)) => a.bpm.is_some_and(|g| (g - exp).abs() <= exp * 0.015),
            (Some(_), None) => true,
            _ => false,
        };
        let key_ok = match (r.as_ref().ok(), f.key) {
            (Some(a), Some(exp)) => a
                .key
                .as_ref()
                .is_some_and(|k| k.key == exp || k.key == exp.relative()),
            (Some(_), None) => true,
            _ => false,
        };
        let offset_ok = match (r.as_ref().ok(), f.offset_s) {
            (Some(a), Some(exp)) => a
                .grid
                .as_ref()
                .is_some_and(|g| (g.first_beat_offset - exp).abs() <= 0.050),
            (Some(_), None) => true,
            _ => false,
        };

        if f.bpm.is_some() {
            bpm_total += 1;
            if bpm_ok {
                bpm_pass += 1;
            }
        }
        if f.key.is_some() {
            key_total += 1;
            if key_ok {
                key_pass += 1;
            }
        }
        if f.offset_s.is_some() {
            offset_total += 1;
            if offset_ok {
                offset_pass += 1;
            }
        }

        println!(
            "{:<26} {:>9} {:>9} {:>13} {:>13} {:>9} {:>9}",
            f.file,
            f.bpm.map(|b| format!("{b:.2}")).unwrap_or_default(),
            got_bpm.unwrap_or_default(),
            f.key.map(|k| format!("{k:?}")).unwrap_or_default(),
            got_key.unwrap_or_default(),
            f.offset_s.map(|o| format!("{o:.3}")).unwrap_or_default(),
            got_offset.unwrap_or_default(),
        );
    }

    println!();
    println!(
        "bpm accuracy:    {bpm_pass}/{bpm_total} ({:.1}%)",
        pct(bpm_pass, bpm_total)
    );
    println!(
        "key accuracy:    {key_pass}/{key_total} ({:.1}%)",
        pct(key_pass, key_total)
    );
    println!(
        "offset accuracy: {offset_pass}/{offset_total} ({:.1}%)",
        pct(offset_pass, offset_total)
    );

    let ok = pct(bpm_pass, bpm_total) >= 90.0
        && pct(key_pass, key_total) >= 90.0
        && pct(offset_pass, offset_total) >= 90.0;
    if !ok {
        eprintln!("accuracy gate: FAILED (any metric < 90%)");
        std::process::exit(1);
    }
    Ok(())
}
