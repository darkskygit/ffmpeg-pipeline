use super::*;
use ffmpeg_next::{
    format::Pixel,
    software::scaling::{context::Context as ScalerContext, flag::Flags as ScalerFlags},
    Stream,
};

pub struct Scaler {
    scaler: ScalerContext,
}

impl Scaler {
    pub fn new(size: &FrameSize, src_format: Pixel, dst_format: Pixel) -> FFmpegResult<Self> {
        Ok(Self {
            scaler: ScalerContext::get(
                src_format,
                size.width as u32,
                size.height as u32,
                dst_format,
                size.width as u32,
                size.height as u32,
                ScalerFlags::SPLINE,
            )?,
        })
    }

    pub fn from_info(info: &VideoInfo, dst_format: Pixel) -> FFmpegResult<Self> {
        Self::new(&info.size, info.pixel, dst_format)
    }

    pub fn from_stream(stream: &Stream, dst_format: Pixel) -> FFmpegResult<Self> {
        let info = parse::parse_stream_info(stream)?;
        debug!(
            "stream: {}, size: {} x {}, pixel: {:?}",
            info.stream, info.size.width, info.size.height, info.pixel
        );
        Self::from_info(&info, dst_format)
    }

    pub fn from_path(path: &Path, index: usize, dst_format: Pixel) -> FFmpegResult<Self> {
        let input = input_file(path)?;
        let stream = input
            .stream(index)
            .ok_or(FFmpegError::StreamNotFound(index))?;
        Self::from_stream(&stream, dst_format)
    }

    pub fn scale_frame(&mut self, frame: &VideoFrame) -> FFmpegResult<VideoFrame> {
        let mut rgb_frame = VideoFrame::empty();
        self.scaler.run(frame, &mut rgb_frame)?;
        Ok(rgb_frame)
    }
}
