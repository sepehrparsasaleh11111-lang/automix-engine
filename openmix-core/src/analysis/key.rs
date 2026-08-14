use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicalKey {
    AMajor,
    ASharpMajor,
    BMajor,
    CMajor,
    CSharpMajor,
    DMajor,
    DSharpMajor,
    EMajor,
    FMajor,
    FSharpMajor,
    GMajor,
    GSharpMajor,
    AMinor,
    ASharpMinor,
    BMinor,
    CMinor,
    CSharpMinor,
    DMinor,
    DSharpMinor,
    EMinor,
    FMinor,
    FSharpMinor,
    GMinor,
    GSharpMinor,
}

impl MusicalKey {
    pub fn relative(&self) -> MusicalKey {
        use MusicalKey::*;
        match self {
            AMajor => FSharpMinor,
            ASharpMajor => GMinor,
            BMajor => GSharpMinor,
            CMajor => AMinor,
            CSharpMajor => ASharpMinor,
            DMajor => BMinor,
            DSharpMajor => CMinor,
            EMajor => CSharpMinor,
            FMajor => DMinor,
            FSharpMajor => DSharpMinor,
            GMajor => EMinor,
            GSharpMajor => FMinor,
            AMinor => CMajor,
            ASharpMinor => CSharpMajor,
            BMinor => DMajor,
            CMinor => DSharpMajor,
            CSharpMinor => EMajor,
            DMinor => FMajor,
            DSharpMinor => FSharpMajor,
            EMinor => GMajor,
            FMinor => GSharpMajor,
            FSharpMinor => AMajor,
            GMinor => ASharpMajor,
            GSharpMinor => BMajor,
        }
    }
    pub fn camelot(&self) -> (u8, char) {
        use MusicalKey::*;
        let (n, l) = match self {
            AMajor => (11, 'B'),
            ASharpMajor => (1, 'B'),
            BMajor => (2, 'B'),
            CMajor => (8, 'B'),
            CSharpMajor => (3, 'B'),
            DMajor => (10, 'B'),
            DSharpMajor => (5, 'B'),
            EMajor => (12, 'B'),
            FMajor => (9, 'B'),
            FSharpMajor => (4, 'B'),
            GMajor => (7, 'B'),
            GSharpMajor => (6, 'B'),
            AMinor => (8, 'A'),
            ASharpMinor => (3, 'A'),
            BMinor => (10, 'A'),
            CMinor => (5, 'A'),
            CSharpMinor => (12, 'A'),
            DMinor => (7, 'A'),
            DSharpMinor => (2, 'A'),
            EMinor => (9, 'A'),
            FMinor => (4, 'A'),
            FSharpMinor => (11, 'A'),
            GMinor => (6, 'A'),
            GSharpMinor => (1, 'A'),
        };
        (n, l)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAlgorithm {
    KeyFinder,
    KrumhanslSchmuckler,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyResult {
    pub key: MusicalKey,
    pub confidence: f32,
    pub algorithm: KeyAlgorithm,
    pub alternate: Option<(MusicalKey, f32)>,
}
