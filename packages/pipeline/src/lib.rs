mod decode;
mod io;
mod parse;
mod result;
mod scaler;
mod types;

pub use decode::{Decoder, Frame, FrameProcess};
pub use ffmpeg_next::format::Pixel as VideoPixel;
pub use io::{input_file, output_file, read_attachment};
pub use parse::parse_video_group;
pub use result::{FFmpegError, FFmpegResult};
pub use scaler::Scaler;

use ffmpeg_next::{
    ffi::AV_LOG_DEBUG,
    sys::{av_log_set_level, AV_LOG_FATAL},
};
use log::{debug, warn};
use std::{error::Error, path::Path, time::Instant};
use types::{FrameCalculation, FrameSize, StreamFormat, VideoFrame, VideoGroups, VideoInfo};

pub fn ffmpeg_init() -> Result<(), Box<dyn Error>> {
    if cfg!(debug_assertions) && false {
        unsafe { av_log_set_level(AV_LOG_DEBUG as i32) }
    } else {
        unsafe { av_log_set_level(AV_LOG_FATAL as i32) }
    }
    ffmpeg_next::init().map_err(|e| e.into())
}
