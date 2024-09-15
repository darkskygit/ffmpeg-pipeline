use super::*;
use ffmpeg_next::{
    codec::{context::Context, Flags as CodecFlags, Id},
    encoder::{self, video::Video as VideoEncoder},
    format::{context::Output, stream::Stream, Flags as FormatFlags},
};
use image_tools::ImageSize;

pub struct Encoder {
    index: usize,
    context: Output,
    encoder: VideoEncoder,
}

impl Encoder {
    pub fn new(mut context: Output, codec: Id) -> FFmpegResult<Self> {
        let codec = encoder::find(codec).ok_or(FFmpegError::CodecNotFound(codec))?;
        let stream = context.add_stream(codec)?;
        let encoder = Context::from_parameters(stream.parameters())?
            .encoder()
            .video()?;

        Ok(Self {
            index: stream.index(),
            context,
            encoder,
        })
    }

    /// create a new encoder and copy params from a source stream
    pub fn new_from_stream(source: Stream, context: Output, codec: Id) -> FFmpegResult<Self> {
        let decoder = Context::from_parameters(source.parameters())?
            .decoder()
            .video()?;
        let mut encoder = Self::new(context, codec)?;

        {
            let global_header = encoder
                .context
                .format()
                .flags()
                .contains(FormatFlags::GLOBAL_HEADER);
            // copy params from source stream
            let encoder = &mut encoder.encoder;
            encoder.set_height(decoder.height());
            encoder.set_width(decoder.width());
            encoder.set_aspect_ratio(decoder.aspect_ratio());
            encoder.set_format(decoder.format());
            encoder.set_frame_rate(decoder.frame_rate());
            encoder.set_time_base(decoder.frame_rate().unwrap().invert());
            if global_header {
                encoder.set_flags(CodecFlags::GLOBAL_HEADER);
            }
        }

        Ok(encoder)
    }

    pub fn set_size(&mut self, size: ImageSize) {
        self.encoder.set_height(size.height as u32);
        self.encoder.set_width(size.width as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_file() {
        let mut output = output_file("./test.mp4").unwrap();
        let mut encoder = Encoder::new(output, Id::H264).unwrap();
    }
}
