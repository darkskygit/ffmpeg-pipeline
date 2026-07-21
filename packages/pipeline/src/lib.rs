//! Composable decoding, encoding, remuxing, scaling, resampling, and media I/O
//! built on FFmpeg.

mod audio;
mod decode;
mod encode;
mod io;
mod parse;
mod remux;
mod result;
mod scaler;
mod types;

pub(crate) use audio::decoder_channel_layout;
pub use audio::{transcode_audio_buffer, AudioSpec, AutoAudioBuffer, Resampler};
pub use decode::{Decoder, Frame, FrameProcess};
pub use encode::{EncodeParams, Encoder};
pub use io::{
    input_buffer, input_buffer_with_format, input_buffer_with_format_options, input_file,
    input_reader, output_buffer, output_file, output_writer, read_attachment,
};
pub use parse::{parse_stream_info, parse_video_group};
pub use remux::{remux, RemuxRequest, RemuxStream};
pub use result::{FFmpegError, FFmpegResult};
pub use scaler::{Scaler, ScalingAlgorithm};
pub use types::{
    AudioFrame, ChannelLayout, CodecId, FrameCalculation, FrameSize, Input, MediaType, Output,
    Rational, Sample, SampleType, Stream, StreamDecoder, StreamEncoder, StreamFormat, StreamFrame,
    VideoFrame, VideoGroups, VideoInfo, VideoPixel,
};

use ffmpeg_next::{
    ffi::{AV_LOG_ERROR, AV_LOG_INFO, AV_LOG_TRACE, AV_LOG_WARNING},
    sys::{av_log_set_level, AV_LOG_DEBUG},
};
use log::{debug, error};
use std::{path::Path, time::Instant};

pub fn initialize(level: log::Level) -> Result<(), ffmpeg_next::Error> {
    let level = match level {
        log::Level::Error => AV_LOG_ERROR,
        log::Level::Warn => AV_LOG_WARNING,
        log::Level::Info => AV_LOG_INFO,
        log::Level::Debug => AV_LOG_DEBUG,
        log::Level::Trace => AV_LOG_TRACE,
    };
    unsafe { av_log_set_level(level) }
    ffmpeg_next::init()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffmpeg_next::codec::Id;
    use rav1e::prelude::{ChromaSampling, Config, Context, EncoderConfig, EncoderStatus};
    use std::io::Cursor;

    pub(crate) fn encoded_ogg() -> Vec<u8> {
        let mut output = output_buffer("ogg").unwrap();
        {
            let mut encoder = Encoder::new(
                output.as_mut(),
                Id::OPUS,
                EncodeParams::default()
                    .with_bitrate(64 * 1024)
                    .with_vbr(true),
            )
            .unwrap();
            encoder.write_header().unwrap();
            let (format, samples, layout, rate) = match encoder.get_encoder() {
                StreamEncoder::Audio(encoder) => (
                    encoder.format(),
                    encoder.frame_size() as usize,
                    encoder.channel_layout(),
                    encoder.rate(),
                ),
                StreamEncoder::Video(_) => panic!("unexpected video encoder"),
            };
            for index in 0..4 {
                let mut frame = AudioFrame::new(format, samples, layout);
                frame.set_rate(rate);
                frame.set_pts(Some((index * samples) as i64));
                for plane in 0..frame.planes() {
                    frame.data_mut(plane).fill(0);
                }
                encoder.send_frame(&StreamFrame::Audio(frame)).unwrap();
                encoder.encode_frame().unwrap();
            }
            encoder.send_frame(&StreamFrame::Eof).unwrap();
            encoder.encode_frame().unwrap();
        }
        output.into_inner::<Cursor<Vec<u8>>>().unwrap().into_inner()
    }

    pub(crate) fn encoded_ivf(frame_count: usize) -> Vec<u8> {
        let mut encoder = EncoderConfig::with_speed_preset(10);
        encoder.width = 16;
        encoder.height = 16;
        encoder.chroma_sampling = ChromaSampling::Cs420;
        encoder.low_latency = false;
        encoder.min_key_frame_interval = 4;
        encoder.max_key_frame_interval = 4;
        let config = Config::new().with_encoder_config(encoder).with_threads(1);
        let mut context: Context<u8> = config.new_context().unwrap();
        for index in 0..frame_count {
            let mut frame = context.new_frame();
            frame.planes[0].data_origin_mut().fill((index * 16) as u8);
            frame.planes[1].data_origin_mut().fill(128);
            frame.planes[2].data_origin_mut().fill(128);
            context.send_frame(frame).unwrap();
        }
        context.flush();
        let mut packets = Vec::new();
        loop {
            match context.receive_packet() {
                Ok(packet) => packets.push((packet.input_frameno, packet.data)),
                Err(EncoderStatus::Encoded | EncoderStatus::NeedMoreData) => continue,
                Err(EncoderStatus::LimitReached) => break,
                Err(error) => panic!("rav1e fixture failed: {error}"),
            }
        }

        let mut ivf = Vec::new();
        ivf.extend_from_slice(b"DKIF");
        ivf.extend_from_slice(&0_u16.to_le_bytes());
        ivf.extend_from_slice(&32_u16.to_le_bytes());
        ivf.extend_from_slice(b"AV01");
        ivf.extend_from_slice(&16_u16.to_le_bytes());
        ivf.extend_from_slice(&16_u16.to_le_bytes());
        ivf.extend_from_slice(&30_u32.to_le_bytes());
        ivf.extend_from_slice(&1_u32.to_le_bytes());
        ivf.extend_from_slice(&(packets.len() as u32).to_le_bytes());
        ivf.extend_from_slice(&0_u32.to_le_bytes());
        for (timestamp, packet) in packets {
            ivf.extend_from_slice(&(packet.len() as u32).to_le_bytes());
            ivf.extend_from_slice(&timestamp.to_le_bytes());
            ivf.extend_from_slice(&packet);
        }
        ivf
    }
}
