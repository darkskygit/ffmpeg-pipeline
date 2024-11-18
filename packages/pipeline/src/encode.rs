use super::*;
use ffmpeg_next::{
    channel_layout::ChannelLayout,
    codec::{context::Context, Compliance, Flags as CodecFlags, Id},
    encoder,
    format::context::Output,
    Packet, Rational,
};

pub enum EncodeParams {
    Audio {
        rate: i32,
        channel_layout: ChannelLayout,
        time_base: Rational,
        global_header: bool,
    },
    Video {
        time_base: Rational,
        global_header: bool,
    },
}

impl EncodeParams {
    pub fn time_base(&self) -> Rational {
        match self {
            EncodeParams::Audio { time_base, .. } => *time_base,
            EncodeParams::Video { time_base, .. } => *time_base,
        }
    }
}

impl Default for EncodeParams {
    fn default() -> Self {
        EncodeParams::Audio {
            rate: 44100,
            channel_layout: ChannelLayout::STEREO,
            time_base: Rational::new(1, 44100),
            global_header: false,
        }
    }
}

impl From<&Decoder<'_>> for EncodeParams {
    fn from(decoder: &Decoder<'_>) -> Self {
        match decoder.get_decoder() {
            StreamDecoder::Audio(decoder) => EncodeParams::Audio {
                rate: decoder.rate() as i32,
                channel_layout: decoder.channel_layout(),
                time_base: decoder.time_base(),
                global_header: false,
            },
            StreamDecoder::Video(decoder) => EncodeParams::Video {
                time_base: decoder.time_base(),
                global_header: false,
            },
        }
    }
}

pub struct Encoder {
    index: usize,
    output: Output,
    encoder: StreamEncoder,
    in_time_base: Rational,
    out_time_base: Rational,
}

impl Encoder {
    pub fn new(mut output: Output, codec_id: Id, codec_params: EncodeParams) -> FFmpegResult<Self> {
        let codec = encoder::find(codec_id).ok_or(FFmpegError::CodecNotFound(codec_id))?;
        let mut stream = output.add_stream(codec.clone())?;
        let mut encoder = Context::from_parameters(stream.parameters())?.encoder();
        encoder.compliance(Compliance::Experimental);

        let encoder = match codec_params {
            EncodeParams::Audio {
                rate,
                channel_layout,
                time_base,
                global_header,
            } if codec.is_audio() => {
                let codec = codec.audio()?;
                let mut encoder = encoder.audio()?;
                let channel_layout = codec
                    .channel_layouts()
                    .map(|cls| cls.best(channel_layout.channels()))
                    .unwrap_or(ChannelLayout::STEREO);
                let rate = codec
                    .rates()
                    .and_then(|rates| {
                        let rates = rates.collect::<Vec<_>>();
                        rates
                            .iter()
                            .find(|&&r| r == rate)
                            .or_else(|| rates.iter().min_by_key(|&&r| (r - rate).abs()))
                            .copied()
                    })
                    .unwrap_or(rate);
                encoder.set_format(
                    codec
                        .formats()
                        .and_then(|mut f| f.next())
                        .ok_or(FFmpegError::CodecNotFound(codec_id))
                        .unwrap(),
                );
                encoder.set_rate(rate);
                encoder.set_channel_layout(channel_layout);
                encoder.set_time_base(time_base);
                if global_header {
                    encoder.set_flags(CodecFlags::GLOBAL_HEADER);
                }
                stream.set_time_base(time_base);

                let encoder = encoder.open_as(codec).unwrap();
                stream.set_parameters(&encoder);
                StreamEncoder::Audio(encoder)
            }
            EncodeParams::Video {
                time_base,
                global_header,
            } if codec.is_video() => {
                let mut encoder = encoder.video()?;
                encoder.set_time_base(time_base);
                if global_header {
                    encoder.set_flags(CodecFlags::GLOBAL_HEADER);
                }
                stream.set_time_base(time_base);

                let encoder = encoder.open_as(codec)?;
                stream.set_parameters(&encoder);
                StreamEncoder::Video(encoder)
            }
            _ => {
                return Err(FFmpegError::CodecNotFound(codec_id));
            }
        };

        let in_time_base = codec_params.time_base();
        let out_time_base = stream.time_base();

        Ok(Self {
            index: stream.index(),
            output,
            encoder,
            in_time_base,
            out_time_base,
        })
    }

    pub fn get_encoder(&self) -> &StreamEncoder {
        &self.encoder
    }

    pub fn write_header(&mut self) -> FFmpegResult<()> {
        Ok(self.output.write_header()?)
    }

    pub fn set_size(&mut self, size: FrameSize) {
        if let StreamEncoder::Video(encoder) = &mut self.encoder {
            encoder.set_height(size.height as u32);
            encoder.set_width(size.width as u32);
        }
    }

    pub fn send_frame(&mut self, frame: &StreamFrame) -> FFmpegResult<()> {
        match (&mut self.encoder, frame) {
            (StreamEncoder::Audio(ref mut encoder), StreamFrame::Audio(frame)) => {
                encoder.send_frame(frame)?;
            }
            (StreamEncoder::Audio(ref mut encoder), StreamFrame::Eof) => {
                encoder.send_eof()?;
            }
            (StreamEncoder::Video(ref mut encoder), StreamFrame::Video(frame)) => {
                encoder.send_frame(frame)?;
            }
            (StreamEncoder::Video(ref mut encoder), StreamFrame::Eof) => {
                encoder.send_eof()?;
            }
            _ => {
                return Err(FFmpegError::InvalidFrameType(frame.to_string()));
            }
        }
        Ok(())
    }

    pub fn encode_frame(&mut self) -> FFmpegResult<()> {
        let mut encoded = Packet::empty();

        match self.encoder {
            StreamEncoder::Audio(ref mut encoder) => {
                while encoder.receive_packet(&mut encoded).is_ok() {
                    encoded.set_stream(self.index);
                    encoded.rescale_ts(self.in_time_base, self.out_time_base);
                    encoded.write_interleaved(&mut self.output)?;
                }
            }
            StreamEncoder::Video(ref mut _encoder) => {
                todo!()
            }
        }

        Ok(())
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        if let Err(e) = self.output.write_trailer() {
            error!("Error writing trailer: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read;

    #[test]
    fn test_encode_audio() {
        ffmpeg_init();

        let buffer = read(
            r#"../../tests/assets/封緘のグラセスタ SOUND COLLECTION/2-12 Brightly horizon.m4a"#,
        )
        .unwrap();
        let index = 0;

        let mut input = input_buffer(buffer).unwrap();

        let decoder = Decoder::new_with_audio(input.as_mut(), index, FrameProcess::Decode).unwrap();
        let mut encoder = Encoder::new(
            output_file("../../tests/tmp/test.opus").unwrap(),
            Id::OPUS,
            (&decoder).into(),
        )
        .unwrap();

        let mut buffer = audio_buffer(&decoder, &encoder).unwrap();

        // output.set_metadata(input.metadata().to_owned());
        encoder.write_header().unwrap();

        for (idx, frame) in decoder.enumerate() {
            let Frame::Frame(StreamFrame::Audio(frame)) = frame else {
                panic!("Unexpected frame type");
            };

            buffer.get("in").unwrap().source().add(&frame).unwrap();

            let mut filtered = AudioFrame::empty();
            let mut ctx = buffer.get("out").unwrap();
            while ctx.sink().frame(&mut filtered).is_ok() {
                encoder
                    .send_frame(&StreamFrame::Audio(filtered.clone()))
                    .unwrap();
                encoder.encode_frame().unwrap();
            }
        }
        encoder.send_frame(&StreamFrame::Eof).unwrap();
        encoder.encode_frame().unwrap();
    }
}
