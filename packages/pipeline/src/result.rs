use ffmpeg_next::codec::Id;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FFmpegError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("FFmpeg error: {0}")]
    FFmpeg(#[from] ffmpeg_next::Error),
    #[error("Attachment not found: {0}")]
    AttachmentNotFound(usize),
    #[error("Stream not found: {0}")]
    StreamNotFound(usize),
    #[error("Codec not found: {0:?}")]
    CodecNotFound(Id),
    #[error("Invalid frame type: {0}")]
    InvalidFrameType(String),
}

pub type FFmpegResult<T> = Result<T, FFmpegError>;
