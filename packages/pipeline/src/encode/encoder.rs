use super::*;
use ffmpeg_next::{
    codec::{context::Context, Compliance, Flags as CodecFlags, Id},
    encoder,
    ffi::av_opt_set_int,
    format::{context::Output, Flags as FormatFlags},
    Packet, Rational,
};

pub struct Encoder<'o> {
    index: usize,
    output: &'o mut Output,
    encoder: StreamEncoder,
    in_time_base: Rational,
    out_time_base: Rational,
}

impl<'o> Encoder<'o> {
    pub fn new(
        output: &'o mut Output,
        codec_id: Id,
        codec_params: EncodeParams,
    ) -> FFmpegResult<Self> {
        let codec = encoder::find(codec_id).ok_or(FFmpegError::CodecNotFound(codec_id))?;
        let output_requires_global_header =
            output.format().flags().contains(FormatFlags::GLOBAL_HEADER);
        let mut stream = output.add_stream(codec)?;
        let mut encoder = Context::from_parameters(stream.parameters())?.encoder();
        encoder.compliance(Compliance::Experimental);

        let encoder = match codec_params {
            EncodeParams::Audio {
                bitrate,
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                vbr,
            } if codec.is_audio() => {
                let codec = codec.audio()?;
                let mut encoder = encoder.audio()?;
                let channel_layout = codec
                    .channel_layouts()
                    .map(|cls| cls.best(channel_layout.channels()))
                    .unwrap_or(channel_layout);
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
                encoder.set_bit_rate(bitrate);
                encoder.set_channel_layout(channel_layout);
                encoder.set_compression(compression);
                encoder.set_format(
                    codec
                        .formats()
                        .and_then(|mut formats| formats.next())
                        .ok_or(FFmpegError::CodecNotFound(codec_id))?,
                );
                encoder.set_rate(rate);
                encoder.set_time_base(time_base);
                if global_header || output_requires_global_header {
                    encoder.set_flags(CodecFlags::GLOBAL_HEADER);
                }
                stream.set_time_base(time_base);

                let mut encoder = encoder.open_as(codec)?;
                if vbr && encoder.id() == Id::OPUS {
                    unsafe {
                        match av_opt_set_int(
                            (*encoder.as_mut_ptr()).priv_data,
                            c"vbr".as_ptr(),
                            2,
                            0,
                        ) {
                            0 => Ok(()),
                            e => Err(ffmpeg_next::Error::from(e)),
                        }
                    }?;
                }

                stream.set_parameters(&encoder);
                StreamEncoder::Audio(encoder)
            }
            EncodeParams::Video {
                time_base,
                global_header,
            } if codec.is_video() => {
                let mut encoder = encoder.video()?;
                encoder.set_time_base(time_base);
                if global_header || output_requires_global_header {
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

    pub fn set_metadata(&mut self, key: &str, value: &str) -> FFmpegResult {
        let mut stream = self
            .output
            .stream_mut(self.index)
            .ok_or(FFmpegError::StreamNotFound(self.index))?;
        let mut metadata = stream.metadata().to_owned();
        metadata.set(key, value);
        stream.set_metadata(metadata);
        Ok(())
    }

    pub fn write_header(&mut self) -> FFmpegResult {
        Ok(self.output.write_header()?)
    }

    pub fn set_size(&mut self, size: FrameSize) {
        if let StreamEncoder::Video(encoder) = &mut self.encoder {
            encoder.set_height(size.height as u32);
            encoder.set_width(size.width as u32);
        }
    }

    pub fn send_frame(&mut self, frame: &StreamFrame) -> FFmpegResult {
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

    pub fn encode_frame(&mut self) -> FFmpegResult {
        let mut encoded = Packet::empty();

        match self.encoder {
            StreamEncoder::Audio(ref mut encoder) => {
                while encoder.receive_packet(&mut encoded).is_ok() {
                    encoded.set_stream(self.index);
                    encoded.rescale_ts(self.in_time_base, self.out_time_base);
                    encoded.write_interleaved(self.output)?;
                }
            }
            StreamEncoder::Video(ref mut _encoder) => {
                todo!()
            }
        }

        Ok(())
    }
}

impl Drop for Encoder<'_> {
    fn drop(&mut self) {
        if let Err(e) = self.output.write_trailer() {
            error!("Error writing trailer: {}", e);
        }
    }
}
