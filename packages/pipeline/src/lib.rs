mod decode;
mod io;
mod parse;
mod scaler;
mod types;

pub use decode::{Frame, FrameProcess, Frames};
pub use ffmpeg_next::format::Pixel as VideoPixel;
pub use io::{open_file, read_attachment};
pub use parse::parse_video_group;
pub use scaler::Scaler;

use ffmpeg_next::sys::{av_log_set_level, AV_LOG_FATAL};
use log::{debug, warn};
use std::{error::Error, io::Result as IoResult, path::Path, time::Instant};
use types::{FrameCalculation, FrameSize, StreamFormat, VideoGroups, VideoInfo};

pub fn ffmpeg_init() -> Result<(), Box<dyn Error>> {
    unsafe { av_log_set_level(AV_LOG_FATAL as i32) }
    ffmpeg_next::init().map_err(|e| e.into())
}
