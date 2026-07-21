use std::io::Cursor;

use super::*;

pub fn transcode_audio_buffer(
    input_bytes: Vec<u8>,
    output_extension: &str,
    codec_id: CodecId,
    bitrate: usize,
) -> FFmpegResult<Vec<u8>> {
    let mut input = input_buffer(input_bytes)?;
    let mut output = output_buffer(output_extension)?;
    {
        let stream_index = input
            .as_ref()
            .streams()
            .find(|stream| stream.parameters().medium() == MediaType::Audio)
            .map(|stream| stream.index())
            .ok_or(FFmpegError::StreamNotFound(0))?;
        let decoder = Decoder::new_with_audio(input.as_mut(), stream_index, FrameProcess::Decode)?;
        let params = EncodeParams::from(&decoder)
            .with_bitrate(bitrate)
            .with_compression((codec_id == CodecId::OPUS).then_some(10))
            .with_vbr(true);
        let mut encoder = Encoder::new(output.as_mut(), codec_id, params)?;
        encoder.set_metadata("encoder", env!("CARGO_PKG_NAME"))?;
        encoder.write_header()?;

        let mut buffer = AutoAudioBuffer::new(&decoder, &encoder)?;
        let mut encode = |frame: AudioFrame| -> FFmpegResult<()> {
            encoder.send_frame(&StreamFrame::Audio(frame))?;
            encoder.encode_frame()
        };
        for frame in decoder {
            let frame = frame?;
            if let Frame::Frame(StreamFrame::Audio(frame)) = frame {
                buffer.add_frame(&frame)?;
                buffer.recv_frames(&mut encode)?;
            }
        }
        buffer.flush()?;
        buffer.recv_frames(&mut encode)?;
        encoder.send_frame(&StreamFrame::Eof)?;
        encoder.encode_frame()?;
    }
    Ok(output.into_inner::<Cursor<Vec<u8>>>()?.into_inner())
}
