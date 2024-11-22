#![feature(seek_stream_len, trait_upcasting)]

mod audio;
mod decode;
mod encode;
mod io;
mod parse;
mod result;
mod scaler;
mod types;

pub use audio::{AudioSpec, AutoAudioBuffer, Resampler};
pub use decode::{Decoder, Frame, FrameProcess};
pub use encode::{EncodeParams, Encoder};
pub use ffmpeg_next::format::Pixel as VideoPixel;
pub use io::{
    input_buffer, input_file, input_reader, output_buffer, output_file, output_writer,
    read_attachment,
};
pub use parse::parse_video_group;
pub use result::{FFmpegError, FFmpegResult};
pub use scaler::Scaler;
pub use types::{
    AudioFrame, ChannelLayout, Input, Output, Sample, SampleType, Stream, StreamDecoder,
    StreamEncoder, StreamFrame, VideoFrame,
};

use ffmpeg_next::{
    ffi::{AV_LOG_ERROR, AV_LOG_INFO, AV_LOG_TRACE, AV_LOG_WARNING},
    sys::{av_log_set_level, AV_LOG_DEBUG, AV_LOG_FATAL},
};
use log::{debug, error, warn};
use std::{path::Path, sync::Once, time::Instant};
use types::{FrameCalculation, FrameSize, StreamFormat, VideoGroups, VideoInfo};

const FFMPEG_INIT: Once = Once::new();

pub fn ffmpeg_init() {
    FFMPEG_INIT.call_once(|| {
        if let Err(e) = ffmpeg_init_explicit(None) {
            warn!("Failed to initialize ffmpeg: {}", e);
        }
    });
}

pub fn ffmpeg_init_with_level(level: log::Level) {
    FFMPEG_INIT.call_once(|| {
        if let Err(e) = ffmpeg_init_explicit(Some(level)) {
            warn!("Failed to initialize ffmpeg: {}", e);
        }
    });
}

pub fn ffmpeg_init_explicit(level: Option<log::Level>) -> Result<(), ffmpeg_next::Error> {
    if let Some(level) = level {
        let level = match level {
            log::Level::Error => AV_LOG_ERROR as i32,
            log::Level::Warn => AV_LOG_WARNING as i32,
            log::Level::Info => AV_LOG_INFO as i32,
            log::Level::Debug => AV_LOG_DEBUG as i32,
            log::Level::Trace => AV_LOG_TRACE as i32,
        };
        unsafe { av_log_set_level(level) }
    } else if cfg!(debug_assertions) {
        unsafe { av_log_set_level(AV_LOG_DEBUG as i32) }
    } else {
        unsafe { av_log_set_level(AV_LOG_FATAL as i32) }
    }
    ffmpeg_next::init()
}
