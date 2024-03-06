use super::*;
use ffmpeg_next::{
    format::Pixel,
    software::scaling::{context::Context as ScalerContext, flag::Flags as ScalerFlags},
    util::frame::video::Video as VideoFrame,
    Stream,
};

pub struct Scaler {
    scaler: ScalerContext,
}

impl Scaler {
    pub fn new(info: &VideoInfo, dst_format: Pixel) -> IoResult<Self> {
        debug!(
            "stream: {}, size: {} x {}, pixel: {:?}",
            info.stream, info.size.width, info.size.height, info.pixel
        );
        Ok(Self {
            scaler: ScalerContext::get(
                info.pixel,
                info.size.width as u32,
                info.size.height as u32,
                dst_format,
                info.size.width as u32,
                info.size.height as u32,
                ScalerFlags::SPLINE,
            )?,
        })
    }

    pub fn new_from_stream(stream: &Stream, dst_format: Pixel) -> IoResult<Self> {
        let info = parse::parse_stream_info(stream)?;
        Self::new(&info, dst_format)
    }

    pub fn new_from_path(path: &Path, index: usize, dst_format: Pixel) -> IoResult<Self> {
        let input = input(path)?;
        let stream = input
            .stream(index)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Stream not found"))?;
        Self::new_from_stream(&stream, dst_format)
    }

    pub fn scale_frame(&mut self, frame: &VideoFrame) -> IoResult<VideoFrame> {
        let mut rgb_frame = VideoFrame::empty();
        self.scaler.run(frame, &mut rgb_frame)?;
        Ok(rgb_frame)
    }
}
