use std::path::{Path, PathBuf};

use openmix_core::analysis::{analyze_path, AnalysisConfig, AnalysisResult, MusicalKey};
use openmix_core::AppError;

struct Fixture {
    file: &'static str,
    bpm: Option<f64>,
    key: Option<MusicalKey>,
    offset_s: Option<f64>,
}

const FIXTURES: &[Fixture] = &[
    // BPM set (all WAV unless noted)
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
    // Key set (pad fixtures): 12 major + 6 minor
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

fn analyze(fixture: &Fixture) -> Result<AnalysisResult, AppError> {
    let path = fixtures_dir().join(fixture.file);
    analyze_path(path, &AnalysisConfig::default())
}

#[test]
fn bpm_accuracy_ge_90_percent() {
    let bpm_fixtures: Vec<&Fixture> = FIXTURES.iter().filter(|f| f.bpm.is_some()).collect();
    let passed = bpm_fixtures
        .iter()
        .filter(|f| match analyze(f) {
            Ok(r) => r
                .bpm
                .is_some_and(|b| (b - f.bpm.unwrap()).abs() <= f.bpm.unwrap() * 0.015),
            Err(_) => false,
        })
        .count();
    let pct = passed as f64 / bpm_fixtures.len() as f64;
    assert!(
        pct >= 0.90,
        "BPM accuracy {:.0}% ({passed}/{}); inspect per-fixture report",
        pct * 100.0,
        bpm_fixtures.len()
    );
}

#[test]
fn key_accuracy_ge_90_percent() {
    let key_fixtures: Vec<&Fixture> = FIXTURES.iter().filter(|f| f.key.is_some()).collect();
    let passed = key_fixtures
        .iter()
        .filter(|f| match analyze(f) {
            Ok(r) => r
                .key
                .is_some_and(|k| k.key == f.key.unwrap() || k.key == f.key.unwrap().relative()),
            Err(_) => false,
        })
        .count();
    let pct = passed as f64 / key_fixtures.len() as f64;
    assert!(
        pct >= 0.90,
        "Key accuracy {:.0}% ({passed}/{}); inspect per-fixture report",
        pct * 100.0,
        key_fixtures.len()
    );
}

#[test]
fn grid_offset_accuracy_ge_90_percent() {
    let grid_fixtures: Vec<&Fixture> = FIXTURES.iter().filter(|f| f.offset_s.is_some()).collect();
    let passed = grid_fixtures
        .iter()
        .filter(|f| match analyze(f) {
            Ok(r) => r
                .grid
                .is_some_and(|g| (g.first_beat_offset - f.offset_s.unwrap()).abs() <= 0.050),
            Err(_) => false,
        })
        .count();
    let pct = passed as f64 / grid_fixtures.len() as f64;
    assert!(
        pct >= 0.90,
        "Grid offset accuracy {:.0}% ({passed}/{}); inspect per-fixture report",
        pct * 100.0,
        grid_fixtures.len()
    );
}

#[test]
fn accuracy_report_prints_per_fixture() {
    // informational: prints a table the Gate 2 report copies verbatim
    for f in FIXTURES {
        let r = analyze(f);
        println!(
            "{:<24} bpm={:?} key={:?} offset={:?} => {:?}",
            f.file,
            f.bpm,
            f.key,
            f.offset_s,
            r.as_ref().map(|a| (
                a.bpm,
                a.key.as_ref().map(|k| k.key),
                a.grid.as_ref().map(|g| g.first_beat_offset)
            ))
        );
    }
}
