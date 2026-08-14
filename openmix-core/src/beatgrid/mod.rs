use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeatLabel {
    Downbeat,
    Beat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Beat {
    pub position_sec: f64,
    pub label: BeatLabel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BpmCurvePoint {
    pub position_sec: f64,
    pub bpm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatGrid {
    pub first_beat_offset: f64,
    pub bpm: f64,
    pub beat_interval: f64,
    pub confidence: f32,
    pub curve: Vec<BpmCurvePoint>,
}
