use super::*;
use ffmpeg_next::software::resampling::context::Context;

pub struct Resampler {
    resampler: Context,
}

impl Resampler {
    pub fn new<S, D>(src: S, dst: D) -> FFmpegResult<Self>
    where
        S: TryInto<AudioSpec, Error = FFmpegError>,
        D: TryInto<AudioSpec, Error = FFmpegError>,
    {
        let src: AudioSpec = src.try_into()?;
        let dst: AudioSpec = dst.try_into()?;
        Ok(Self {
            resampler: Context::get(
                src.format,
                src.channel_layout,
                src.sample_rate,
                dst.format,
                dst.channel_layout,
                dst.sample_rate,
            )?,
        })
    }

    pub fn resample(&mut self, frame: &AudioFrame) -> FFmpegResult<AudioFrame> {
        let mut resampled_frame = AudioFrame::empty();
        self.resampler.run(frame, &mut resampled_frame)?;
        Ok(resampled_frame)
    }
}
