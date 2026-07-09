use super::*;
use ffmpeg_next::{
    format::Pixel,
    software::scaling::{context::Context as ScalerContext, flag::Flags as ScalerFlags},
    Stream,
};

pub struct Scaler {
    scaler: ScalerContext,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScalingAlgorithm {
    Bilinear,
    Bicubic,
    #[default]
    Spline,
    Lanczos,
}

impl Scaler {
    pub fn new(size: &FrameSize, src_format: Pixel, dst_format: Pixel) -> FFmpegResult<Self> {
        Self::with_algorithm(size, src_format, dst_format, ScalingAlgorithm::default())
    }

    pub fn with_algorithm(
        size: &FrameSize,
        src_format: Pixel,
        dst_format: Pixel,
        algorithm: ScalingAlgorithm,
    ) -> FFmpegResult<Self> {
        let flags = match algorithm {
            ScalingAlgorithm::Bilinear => ScalerFlags::BILINEAR,
            ScalingAlgorithm::Bicubic => ScalerFlags::BICUBIC,
            ScalingAlgorithm::Spline => ScalerFlags::SPLINE,
            ScalingAlgorithm::Lanczos => ScalerFlags::LANCZOS,
        };
        Ok(Self {
            scaler: ScalerContext::get(
                src_format,
                size.width as u32,
                size.height as u32,
                dst_format,
                size.width as u32,
                size.height as u32,
                flags,
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
