mod io;
mod parse;
mod pipeline;
mod types;

pub use io::read_attachment;
pub use parse::parse_video_group;
pub use pipeline::{FrameIterator, Scaler};

use ffmpeg_next::sys::{av_log_set_level, AV_LOG_FATAL};
use io::input;
use log::{debug, warn};
use std::{error::Error, io::Result as IoResult, path::Path, time::Instant};
use types::{FrameCalculation, StreamFormat, VideoGroups, VideoInfo};

pub fn ffmpeg_init() -> Result<(), Box<dyn Error>> {
    unsafe { av_log_set_level(AV_LOG_FATAL as i32) }
    ffmpeg_next::init().map_err(|e| e.into())
}
