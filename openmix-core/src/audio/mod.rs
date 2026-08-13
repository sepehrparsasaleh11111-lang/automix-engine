pub mod decode;
pub mod peaks;

pub use decode::{AudioChunk, DecodedStream, TrackMeta};
pub use peaks::{compute_peaks, Peak};
