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
            ASharpMajor => (6, 'B'),
            BMajor => (1, 'B'),
            CMajor => (8, 'B'),
            CSharpMajor => (3, 'B'),
            DMajor => (10, 'B'),
            DSharpMajor => (5, 'B'),
            EMajor => (12, 'B'),
            FMajor => (7, 'B'),
            FSharpMajor => (2, 'B'),
            GMajor => (9, 'B'),
            GSharpMajor => (4, 'B'),
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
    /// Map a libkeyfinder `key_t` enum value (vendored at
    /// `src/keyfinder/vendor/libkeyfinder/src/constants.h`) to `MusicalKey`.
    /// Index order follows the vendored header: per pitch class (A, Bb, B,
    /// C, Db, D, Eb, E, F, Gb, G, Ab) major then minor, i.e.
    /// `A_MAJOR=0 .. A_FLAT_MINOR=23`. Returns `None` for out-of-range or
    /// `SILENCE` (24).
    pub fn from_keyfinder_index(i: i32) -> Option<MusicalKey> {
        use MusicalKey::*;
        const MAJOR: [MusicalKey; 12] = [
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
        ];
        const MINOR: [MusicalKey; 12] = [
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
        ];
        if !(0..24).contains(&i) {
            return None;
        }
        let pc = (i / 2) as usize;
        Some(if i % 2 == 0 { MAJOR[pc] } else { MINOR[pc] })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn keyfinder_index_maps_all_24_to_distinct_keys() {
        let mut seen = HashSet::new();
        for i in 0..24 {
            let k = MusicalKey::from_keyfinder_index(i).expect("in range");
            assert!(
                seen.insert(k.camelot()),
                "duplicate camelot for index {i}: {k:?}"
            );
        }
        assert_eq!(seen.len(), 24);
    }

    #[test]
    fn keyfinder_index_out_of_range_is_none() {
        assert_eq!(MusicalKey::from_keyfinder_index(-1), None);
        assert_eq!(MusicalKey::from_keyfinder_index(24), None);
        assert_eq!(MusicalKey::from_keyfinder_index(100), None);
    }

    #[test]
    fn keyfinder_index_spot_checks() {
        assert_eq!(
            MusicalKey::from_keyfinder_index(0),
            Some(MusicalKey::AMajor)
        );
        assert_eq!(
            MusicalKey::from_keyfinder_index(1),
            Some(MusicalKey::AMinor)
        );
        assert_eq!(
            MusicalKey::from_keyfinder_index(6),
            Some(MusicalKey::CMajor)
        );
        assert_eq!(
            MusicalKey::from_keyfinder_index(18),
            Some(MusicalKey::FSharpMajor)
        );
        assert_eq!(
            MusicalKey::from_keyfinder_index(23),
            Some(MusicalKey::GSharpMinor)
        );
    }
}
