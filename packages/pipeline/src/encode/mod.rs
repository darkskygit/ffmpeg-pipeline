mod encoder;
mod params;

use super::*;

pub use encoder::Encoder;
pub use params::EncodeParams;

#[cfg(test)]
mod tests {
    use super::*;
    use ffmpeg_next::codec::Id;
    use std::io::Cursor;

    fn silent_wav(sample_count: u32) -> Vec<u8> {
        let data_len = sample_count * 2;
        let mut wav = Vec::with_capacity((44 + data_len) as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&48_000_u32.to_le_bytes());
        wav.extend_from_slice(&96_000_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.resize((44 + data_len) as usize, 0);
        wav
    }

    #[test]
    fn test_encode_audio() {
        initialize(log::Level::Error).unwrap();

        let buffer = silent_wav(4_800);
        let index = 0;

        let mut input = input_buffer(buffer).unwrap();
        let mut output = output_buffer("ogg").unwrap();

        let decoder = Decoder::new_with_audio(input.as_mut(), index, FrameProcess::Decode).unwrap();
        {
            let mut encoder = Encoder::new(
                output.as_mut(),
                Id::OPUS,
                EncodeParams::from(&decoder)
                    .with_bitrate(64 * 1024)
                    .with_vbr(true),
            )
            .unwrap();

            encoder.set_metadata("encoder", "ffmpeg").unwrap();
            encoder.write_header().unwrap();

            let mut buffer = AutoAudioBuffer::new(&decoder, &encoder).unwrap();
            let mut encode_cb = |frame: AudioFrame| -> FFmpegResult<()> {
                encoder.send_frame(&StreamFrame::Audio(frame))?;
                encoder.encode_frame()?;
                Ok(())
            };

            for frame in decoder {
                let Frame::Frame(StreamFrame::Audio(frame)) = frame else {
                    panic!("Unexpected frame type");
                };
                buffer.add_frame(&frame).unwrap();
                buffer.recv_frames(&mut encode_cb).unwrap();
            }
            buffer.flush().unwrap();
            buffer.recv_frames(&mut encode_cb).unwrap();

            encoder.send_frame(&StreamFrame::Eof).unwrap();
            encoder.encode_frame().unwrap();
        }

        let buffer = output.into_inner::<Cursor<Vec<_>>>().unwrap().into_inner();
        assert!(buffer.starts_with(b"OggS"));
    }
}
