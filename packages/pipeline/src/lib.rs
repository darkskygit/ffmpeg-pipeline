//! Composable decoding, encoding, remuxing, scaling, resampling, and media I/O
//! built on FFmpeg.

mod audio;
mod decode;
mod encode;
mod io;
mod parse;
mod remux;
mod result;
mod scaler;
mod types;

pub(crate) use audio::decoder_channel_layout;
pub use audio::{transcode_audio_buffer, AudioSpec, AutoAudioBuffer, Resampler};
pub use decode::{Decoder, Frame, FrameProcess};
pub use encode::{EncodeParams, Encoder};
pub use io::{
    input_buffer, input_buffer_with_format, input_buffer_with_format_options, input_file,
    input_reader, output_buffer, output_file, output_writer, read_attachment,
};
pub use parse::{parse_stream_info, parse_video_group};
pub use remux::{remux, RemuxRequest, RemuxStream};
pub use result::{FFmpegError, FFmpegResult};
pub use scaler::{Scaler, ScalingAlgorithm};
pub use types::{
    AudioFrame, ChannelLayout, CodecId, FrameCalculation, FrameSize, Input, MediaType, Output,
    Rational, Sample, SampleType, Stream, StreamDecoder, StreamEncoder, StreamFormat, StreamFrame,
    VideoFrame, VideoGroups, VideoInfo, VideoPixel,
};

use ffmpeg_next::{
    ffi::{AV_LOG_ERROR, AV_LOG_INFO, AV_LOG_TRACE, AV_LOG_WARNING},
    sys::{av_log_set_level, AV_LOG_DEBUG},
};
use log::{debug, error, warn};
use std::{path::Path, time::Instant};

pub fn initialize(level: log::Level) -> Result<(), ffmpeg_next::Error> {
    let level = match level {
        log::Level::Error => AV_LOG_ERROR,
        log::Level::Warn => AV_LOG_WARNING,
        log::Level::Info => AV_LOG_INFO,
        log::Level::Debug => AV_LOG_DEBUG,
        log::Level::Trace => AV_LOG_TRACE,
    };
    unsafe { av_log_set_level(level) }
    ffmpeg_next::init()
}
