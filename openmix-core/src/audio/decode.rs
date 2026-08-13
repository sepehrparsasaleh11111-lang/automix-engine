use crate::error::AppError;
use std::path::{Path, PathBuf};
use std::time::Duration;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioChunk {
    pub samples: Vec<f32>,
    pub frames: usize,
}

#[derive(Debug, Default, Clone)]
pub struct TrackMeta {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

pub struct DecodedStream {
    path: PathBuf,
    sample_rate: u32,
    channels: u16,
    duration: Duration,
    meta: TrackMeta,
    decoder: Option<Box<dyn symphonia::core::codecs::Decoder>>,
    track_id: u32,
    sample_buf: SampleBuffer<f32>,
    format: Option<Box<dyn symphonia::core::formats::FormatReader>>,
}

impl DecodedStream {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref().to_path_buf();
        let file = std::fs::File::open(&path)
            .map_err(|e| AppError::OpenFile(path.clone(), e.to_string()))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let mut probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|_| AppError::UnsupportedFormat(path.clone()))?;

        let track = probed
            .format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| AppError::UnsupportedFormat(path.clone()))?;
        let track_id = track.id;
        let params = track.codec_params.clone();
        let sample_rate = params.sample_rate.unwrap_or(44100);
        let channels = params.channels.map(|c| c.count() as u16).unwrap_or(2);
        let n_frames = params.n_frames.unwrap_or(0) as u64;
        let duration = if n_frames > 0 {
            Duration::from_secs_f64(n_frames as f64 / sample_rate as f64)
        } else {
            Duration::ZERO
        };

        let mut meta = TrackMeta::default();
        if let Some(metadata) = probed.metadata.get() {
            if let Some(tags) = metadata.current().map(|revision| revision.tags()) {
                for tag in tags {
                    let key = tag.key.to_ascii_lowercase();
                    let val = tag.value.to_string();
                    match key.as_str() {
                        "title" => meta.title = Some(val),
                        "artist" => meta.artist = Some(val),
                        "album" => meta.album = Some(val),
                        _ => {}
                    }
                }
            }
        }

        let format = Some(probed.format);
        let decoder = {
            let codec_params = format.as_ref().unwrap().tracks()[track_id as usize]
                .codec_params
                .clone();
            symphonia::default::get_codecs()
                .make(&codec_params, &DecoderOptions::default())
                .ok()
        };
        let sample_buf = SampleBuffer::<f32>::new(
            0,
            symphonia::core::audio::SignalSpec::new(
                params.sample_rate.unwrap_or(44100),
                params
                    .channels
                    .unwrap_or(symphonia::core::audio::Channels::FRONT_LEFT),
            ),
        );

        Ok(Self {
            path,
            sample_rate,
            channels,
            duration,
            meta,
            decoder,
            track_id,
            sample_buf,
            format,
        })
    }

    pub fn next_chunk(&mut self, frames: usize) -> Result<Option<AudioChunk>, AppError> {
        let format = self
            .format
            .as_mut()
            .ok_or_else(|| AppError::Decode("stream exhausted".into()))?;
        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                Err(e) => return Err(AppError::Decode(e.to_string())),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let decoder = self
                .decoder
                .as_mut()
                .ok_or_else(|| AppError::Decode("no decoder".into()))?;
            let decoded = decoder
                .decode(&packet)
                .map_err(|e| AppError::Decode(e.to_string()))?;
            let spec = *decoded.spec();
            self.sample_buf = SampleBuffer::new(frames as u64, spec);
            self.sample_buf.copy_interleaved_ref(decoded);
            let samples = self.sample_buf.samples().to_vec();
            let frame_count = samples.len() / spec.channels.count();
            return Ok(Some(AudioChunk {
                samples,
                frames: frame_count,
            }));
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn metadata(&self) -> &TrackMeta {
        &self.meta
    }
}
