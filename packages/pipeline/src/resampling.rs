use super::*;
use ffmpeg_next::software::resampling::context::Context as ResamplerContext;

pub struct AudioSpec {
    sample_rate: u32,
    format: Sample,
    channel_layout: ChannelLayout,
}

impl AudioSpec {
    pub fn new(sample_rate: u32, format: Sample, channel_layout: ChannelLayout) -> Self {
        Self {
            sample_rate,
            format,
            channel_layout,
        }
    }
}

pub struct Resampler {
    resampler: ResamplerContext,
}

impl Resampler {
    pub fn new(src: AudioSpec, dst: AudioSpec) -> FFmpegResult<Self> {
        Ok(Self {
            resampler: ResamplerContext::get(
                src.format,
                src.channel_layout,
                src.sample_rate,
                dst.format,
                dst.channel_layout,
                dst.sample_rate,
            )?,
        })
    }

    pub fn new_from_stream(stream: &Stream, dst: AudioSpec) -> FFmpegResult<Self> {
        Self::new(parse::parse_audio_spec(stream)?, dst)
    }

    pub fn resample(&mut self, frame: &AudioFrame) -> FFmpegResult<AudioFrame> {
        let mut resampled_frame = AudioFrame::empty();
        self.resampler.run(frame, &mut resampled_frame)?;
        Ok(resampled_frame)
    }
}
