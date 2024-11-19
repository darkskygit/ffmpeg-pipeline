use super::*;
use ffmpeg_next::{codec::context::Context, media::Type as MediaType, Codec, Rational, Stream};

#[derive(Clone)]
pub struct AudioSpec {
    pub(super) sample_rate: u32,
    pub(super) time_base: Rational,
    pub(super) format: Sample,
    pub(super) channel_layout: ChannelLayout,
    pub(super) codec: Option<Codec>,
    pub(super) frame_size: u32,
}

impl AudioSpec {
    pub fn new(channel_layout: ChannelLayout, format: Sample, sample_rate: u32) -> Self {
        Self {
            channel_layout,
            format,
            frame_size: 0,
            sample_rate,
            time_base: (1, sample_rate as i32).into(),
            codec: None,
        }
    }

    pub fn with_codec(mut self, codec: Option<Codec>) -> Self {
        self.codec = codec;
        self
    }

    pub fn with_frame_size(mut self, frame_size: u32) -> Self {
        self.frame_size = frame_size;
        self
    }

    pub fn with_time_base(mut self, time_base: Rational) -> Self {
        self.time_base = time_base;
        self
    }
}

impl TryFrom<&AudioSpec> for AudioSpec {
    type Error = FFmpegError;
    fn try_from(a: &AudioSpec) -> Result<Self, Self::Error> {
        Ok(a.clone())
    }
}

impl TryFrom<&Decoder<'_>> for AudioSpec {
    type Error = FFmpegError;
    fn try_from(decoder: &Decoder<'_>) -> Result<Self, Self::Error> {
        match decoder.get_decoder() {
            StreamDecoder::Audio(decoder) => {
                Ok(
                    Self::new(decoder.channel_layout(), decoder.format(), decoder.rate())
                        .with_codec(decoder.codec())
                        .with_frame_size(decoder.frame_size())
                        .with_time_base(decoder.time_base()),
                )
            }
            _ => Err(FFmpegError::InvalidStreamType("Video".into())),
        }
    }
}

impl TryFrom<&Encoder> for AudioSpec {
    type Error = FFmpegError;
    fn try_from(encoder: &Encoder) -> Result<Self, Self::Error> {
        match encoder.get_encoder() {
            StreamEncoder::Audio(encoder) => {
                Ok(
                    Self::new(encoder.channel_layout(), encoder.format(), encoder.rate())
                        .with_codec(encoder.codec())
                        .with_frame_size(encoder.frame_size())
                        .with_time_base(encoder.time_base()),
                )
            }
            _ => Err(FFmpegError::InvalidStreamType("Video".into())),
        }
    }
}

impl TryFrom<&Stream<'_>> for AudioSpec {
    type Error = FFmpegError;
    fn try_from(stream: &Stream) -> Result<Self, Self::Error> {
        let codec = Context::from_parameters(stream.parameters())?;
        if codec.medium() == MediaType::Audio {
            let decoder = codec.decoder().audio()?;
            Ok(
                AudioSpec::new(decoder.channel_layout(), decoder.format(), decoder.rate())
                    .with_codec(decoder.codec())
                    .with_frame_size(decoder.frame_size())
                    .with_time_base(decoder.time_base()),
            )
        } else {
            Err(FFmpegError::CodecNotFound(stream.parameters().id()))
        }
    }
}
