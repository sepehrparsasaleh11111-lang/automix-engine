use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("failed to open audio file {0}: {1}")]
    OpenFile(PathBuf, String),
    #[error("unsupported or corrupt audio format in {0}")]
    UnsupportedFormat(PathBuf),
    #[error("audio decode error: {0}")]
    Decode(String),
    #[error("analysis error: {0}")]
    Analysis(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}
