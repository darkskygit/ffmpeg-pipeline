#![feature(seek_stream_len)]

mod audio;
mod decode;
mod encode;
mod parse;
mod reader;
mod result;
mod scaler;
mod types;

pub use audio::{audio_buffer, AudioSpec, Resampler};
pub use decode::{Decoder, Frame, FrameProcess};
pub use encode::{EncodeParams, Encoder};
pub use ffmpeg_next::format::Pixel as VideoPixel;
pub use parse::parse_video_group;
pub use reader::{input_buffer, input_file, input_reader, output_file, read_attachment};
pub use result::{FFmpegError, FFmpegResult};
pub use scaler::Scaler;
pub use types::{
    AudioFrame, ChannelLayout, Input, Output, Sample, SampleType, Stream, StreamDecoder,
    StreamEncoder, StreamFrame, VideoFrame,
};

use ffmpeg_next::sys::{av_log_set_level, AV_LOG_DEBUG, AV_LOG_FATAL};
use log::{debug, error, warn};
use std::{path::Path, sync::Once, time::Instant};
use types::{FrameCalculation, FrameSize, StreamFormat, VideoGroups, VideoInfo};

const FFMPEG_INIT: Once = Once::new();

pub fn ffmpeg_init() {
    FFMPEG_INIT.call_once(|| {
        if let Err(e) = ffmpeg_init_explicit() {
            warn!("Failed to initialize ffmpeg: {}", e);
        }
    });
}

pub fn ffmpeg_init_explicit() -> Result<(), ffmpeg_next::Error> {
    if cfg!(debug_assertions) && false {
        unsafe { av_log_set_level(AV_LOG_DEBUG as i32) }
    } else {
        unsafe { av_log_set_level(AV_LOG_FATAL as i32) }
    }
    ffmpeg_next::init()
}
