mod encoder;
mod params;

use super::*;

pub use encoder::Encoder;
pub use params::EncodeParams;

#[cfg(test)]
mod tests {
    use super::*;
    use ffmpeg_next::codec::Id;
    use std::{
        fs::{read, write},
        io::Cursor,
    };

    #[test]
    fn test_encode_audio() {
        ffmpeg_init();

        let buffer = read(r#"../../tests/assets/1.m4a"#).unwrap();
        let index = 0;

        let mut input = input_buffer(buffer).unwrap();
        let mut output = output_buffer("opus").unwrap();

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

            encoder.set_metadata("encoder", "ffmpeg");
            encoder.write_header().unwrap();

            let mut buffer = AutoAudioBuffer::new(&decoder, &encoder).unwrap();

            for frame in decoder {
                let Frame::Frame(StreamFrame::Audio(frame)) = frame else {
                    panic!("Unexpected frame type");
                };
                buffer.add_frame(&frame).unwrap();
                buffer
                    .recv_frames(|frame| {
                        encoder.send_frame(&StreamFrame::Audio(frame))?;
                        encoder.encode_frame()?;
                        Ok(())
                    })
                    .unwrap();
            }
            buffer.flush().unwrap();
            buffer
                .recv_frames(|frame| {
                    encoder.send_frame(&StreamFrame::Audio(frame))?;
                    encoder.encode_frame()?;
                    Ok(())
                })
                .unwrap();

            encoder.send_frame(&StreamFrame::Eof).unwrap();
            encoder.encode_frame().unwrap();
        }

        let buffer = output.into_inner::<Cursor<Vec<_>>>().unwrap();
        write("./tmp/1.opus", buffer.into_inner()).unwrap();
    }
}
