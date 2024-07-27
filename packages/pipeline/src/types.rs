use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as fmtResult},
    time::Duration,
};

use ffmpeg_next::format::Pixel;
use log::warn;
use serde::{Deserialize, Serialize};

pub use ffmpeg_next::{
    format::{
        context::{Input, Output},
        sample::Type as SampleType,
        Sample,
    },
    util::frame::{audio::Audio as AudioFrame, video::Video as VideoFrame},
    ChannelLayout, Stream,
};

#[derive(Default, Clone)]
pub enum FrameCalculation {
    #[default]
    Skip,
    Fast,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamFormat {
    AV1,
    H264,
    HEVC,
    Other(String),
}

impl Default for StreamFormat {
    fn default() -> Self {
        Self::Other("Unknown".into())
    }
}

impl ToString for StreamFormat {
    fn to_string(&self) -> String {
        match self {
            Self::AV1 => "ivf",
            Self::H264 => "h264",
            Self::HEVC => "hevc",
            Self::Other(fmt) => {
                warn!("unknown stream format: {}, get rawvideo", fmt);
                "rawvideo"
            }
        }
        .into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSize {
    pub width: isize,
    pub height: isize,
}

impl FrameSize {
    pub fn new() -> Self {
        FrameSize {
            width: 0,
            height: 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
    pub fn width<I: Into<isize>>(mut self, width: I) -> Self {
        self.width = width.into();
        self
    }
    pub fn height<I: Into<isize>>(mut self, height: I) -> Self {
        self.height = height.into();
        self
    }
}

impl Default for FrameSize {
    fn default() -> Self {
        FrameSize::new()
    }
}

impl Display for FrameSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}x{}", self.width, self.height)
    }
}

fn default_pixel() -> Pixel {
    Pixel::None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub format: StreamFormat,
    #[serde(skip)]
    pub stream: u16,
    pub stream_type: String,
    #[serde(skip, default = "default_pixel")]
    pub pixel: Pixel,
    pub size: FrameSize,
    pub fps: Option<f64>,
    pub frames: Option<u64>,
    pub cost: Duration,
    metadata: HashMap<String, String>,
}

impl Default for VideoInfo {
    fn default() -> Self {
        Self {
            format: StreamFormat::default(),
            stream: 0,
            stream_type: "".into(),
            pixel: Pixel::None,
            size: FrameSize::default(),
            fps: None,
            frames: None,
            cost: Duration::from_secs(0),
            metadata: HashMap::new(),
        }
    }
}

impl VideoInfo {
    pub fn stream(mut self, stream: u16) -> Self {
        self.stream = stream;
        self
    }
    pub fn stream_type(mut self, stream_type: String) -> Self {
        self.stream_type = stream_type;
        self
    }
    pub fn insert(mut self, key: String, val: String) -> Self {
        self.metadata.insert(key, val);
        self
    }
    pub fn size<I: Into<isize>>(mut self, w: I, h: I) -> Self {
        self.size = self.size.width(w).height(h);
        self
    }
    fn get_unnamed_title(&self) -> String {
        format!("!unnamed_stream_{}", &self.stream)
    }
    fn get_filename(&self) -> String {
        self.metadata
            .get("filename")
            .unwrap_or(&self.get_unnamed_title())
            .to_string()
    }
    pub fn get_handler_name(&self) -> String {
        self.metadata
            .get("handler_name")
            .unwrap_or(&self.get_filename())
            .to_string()
    }
    pub fn get_title(&self) -> String {
        self.metadata
            .get("title")
            .unwrap_or(&self.get_handler_name())
            .to_string()
    }
    pub fn get_mimetype(&self) -> String {
        self.metadata
            .get("mimetype")
            .unwrap_or(&"binary".into())
            .into()
    }
}

pub type VideoGroups = HashMap<String, VideoInfo>;
