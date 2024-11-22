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
            let mut buffer = audio_buffer(&decoder, &encoder).unwrap();

            encoder.set_metadata("encoder", "ffmpeg");
            encoder.write_header().unwrap();

            let mut buffer_out = buffer.get("out").unwrap();
            let mut receive_buffered_frame = || {
                let mut filtered = AudioFrame::empty();
                while buffer_out.sink().frame(&mut filtered).is_ok() {
                    encoder.send_frame(&StreamFrame::Audio(filtered.clone()))?;
                    encoder.encode_frame()?;
                }
                Ok::<(), FFmpegError>(())
            };

            let mut buffer_in = buffer.get("in").unwrap();
            let mut buffer_in_src = buffer_in.source();
            for (idx, frame) in decoder.enumerate() {
                let Frame::Frame(StreamFrame::Audio(frame)) = frame else {
                    panic!("Unexpected frame type");
                };
                buffer_in_src.add(&frame).unwrap();
                receive_buffered_frame().unwrap();
            }
            buffer_in_src.flush().unwrap();
            receive_buffered_frame().unwrap();

            encoder.send_frame(&StreamFrame::Eof).unwrap();
            encoder.encode_frame().unwrap();
        }

        let buffer = output.into_inner::<Cursor<Vec<_>>>().unwrap();
        write("./tmp/1.opus", buffer.into_inner()).unwrap();
    }
}
