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
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    #[error("Invalid frame type: {0}")]
    InvalidFrameType(String),
    #[error("Invalid stream type: {0}")]
    InvalidStreamType(String),
    #[error(
        "Decoder {operation} failed on stream {stream} at packet {packet_position:?}: {source}"
    )]
    Decoder {
        operation: &'static str,
        stream: usize,
        packet_position: Option<isize>,
        #[source]
        source: ffmpeg_next::Error,
    },
}

impl FFmpegError {
    pub(crate) fn decoder(
        operation: &'static str,
        stream: usize,
        packet_position: Option<isize>,
        source: ffmpeg_next::Error,
    ) -> Self {
        Self::Decoder {
            operation,
            stream,
            packet_position,
            source,
        }
    }
}

impl PartialEq<FFmpegError> for FFmpegError {
    fn eq(&self, other: &FFmpegError) -> bool {
        match (self, other) {
            (FFmpegError::FFmpeg(e1), FFmpegError::FFmpeg(e2)) => e1 == e2,
            (FFmpegError::AttachmentNotFound(e1), FFmpegError::AttachmentNotFound(e2)) => e1 == e2,
            (FFmpegError::StreamNotFound(e1), FFmpegError::StreamNotFound(e2)) => e1 == e2,
            (FFmpegError::CodecNotFound(e1), FFmpegError::CodecNotFound(e2)) => e1 == e2,
            (FFmpegError::InvalidFrameType(e1), FFmpegError::InvalidFrameType(e2)) => e1 == e2,
            (
                FFmpegError::Decoder {
                    operation: o1,
                    stream: s1,
                    packet_position: p1,
                    source: e1,
                },
                FFmpegError::Decoder {
                    operation: o2,
                    stream: s2,
                    packet_position: p2,
                    source: e2,
                },
            ) => o1 == o2 && s1 == s2 && p1 == p2 && e1 == e2,
            _ => false,
        }
    }
}

pub type FFmpegResult<T = ()> = Result<T, FFmpegError>;
