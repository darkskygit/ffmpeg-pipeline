use super::*;
use ffmpeg_next::{
    codec::context::Context,
    format::context::{input::PacketIter, Input},
    Error as FFmpegOrigError, Packet,
};

enum FrameStatus {
    Raw(Packet),
    Decoded(StreamFrame),
    Eof,
    Error(isize, FFmpegOrigError),
}

#[derive(PartialEq)]
pub enum FrameProcess {
    Passthrough,
    Decode,
}

pub enum Frame {
    Packet(Packet),
    Frame(StreamFrame),
}

pub struct Decoder<'i> {
    index: usize,
    decoder: StreamDecoder,
    packets: PacketIter<'i>,
    process: FrameProcess,
}

impl<'i> Decoder<'i> {
    pub fn new_with_video(
        handler: &'i mut Input,
        index: usize,
        process: FrameProcess,
    ) -> FFmpegResult<Self> {
        if let Some(stream) = handler.stream(index) {
            let codec = Context::from_parameters(stream.parameters())?;
            let decoder = StreamDecoder::Video(codec.decoder().video()?);
            let packets = handler.packets();

            Ok(Self {
                index,
                decoder,
                packets,
                process,
            })
        } else {
            Err(FFmpegError::StreamNotFound(index))
        }
    }

    pub fn new_with_audio(
        input: &'i mut Input,
        index: usize,
        process: FrameProcess,
    ) -> FFmpegResult<Self> {
        if let Some(stream) = input.stream(index) {
            let codec = Context::from_parameters(stream.parameters())?;
            let decoder = StreamDecoder::Audio(codec.decoder().audio()?);
            let packets = input.packets();

            Ok(Self {
                index,
                decoder,
                packets,
                process,
            })
        } else {
            Err(FFmpegError::StreamNotFound(index))
        }
    }

    pub fn get_decoder(&self) -> &StreamDecoder {
        &self.decoder
    }

    pub fn decode_frames(&mut self) -> Option<StreamFrame> {
        match self.decoder {
            StreamDecoder::Audio(ref mut decoder) => {
                let mut decoded = AudioFrame::empty();
                if decoder.receive_frame(&mut decoded).is_ok() {
                    let timestamp = decoded.timestamp();
                    decoded.set_pts(timestamp);
                    return Some(StreamFrame::Audio(decoded));
                }
            }
            StreamDecoder::Video(ref mut decoder) => {
                let mut decoded = VideoFrame::empty();
                if decoder.receive_frame(&mut decoded).is_ok() {
                    return Some(StreamFrame::Video(decoded));
                }
            }
        }
        None
    }
}

impl Iterator for Decoder<'_> {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let next_frame = loop {
            if let Some((stream, packet)) = self.packets.next() {
                if stream.index() != self.index {
                    continue;
                }
                match self.process {
                    FrameProcess::Passthrough => break FrameStatus::Raw(packet),
                    FrameProcess::Decode => {
                        match self.decoder {
                            StreamDecoder::Audio(ref mut decoder) => {
                                if let Err(e) = decoder.send_packet(&packet) {
                                    break FrameStatus::Error(packet.position(), e);
                                }
                            }
                            StreamDecoder::Video(ref mut decoder) => {
                                if let Err(e) = decoder.send_packet(&packet) {
                                    break FrameStatus::Error(packet.position(), e);
                                }
                            }
                        }
                        if let Some(frame) = self.decode_frames() {
                            break FrameStatus::Decoded(frame);
                        }
                    }
                }
            } else {
                break FrameStatus::Eof;
            }
        };

        match next_frame {
            FrameStatus::Raw(packet) => Some(Frame::Packet(packet)),
            FrameStatus::Decoded(frame) => Some(Frame::Frame(frame)),
            FrameStatus::Eof => {
                if let Err(e) = match self.decoder {
                    StreamDecoder::Audio(ref mut decoder) => decoder.send_eof(),
                    StreamDecoder::Video(ref mut decoder) => decoder.send_eof(),
                } {
                    warn!("Failed to send EOF to stream {}: {}", self.index, e);
                }
                if self.process == FrameProcess::Decode {
                    self.decode_frames().map(Frame::Frame)
                } else {
                    None
                }
            }
            FrameStatus::Error(pos, e) => {
                warn!(
                    "Failed to process packet {} in stream {}: {}",
                    pos, self.index, e
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use std::fs::read;
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::channel;
    use std::thread;

    fn save_file(frame: &VideoFrame, index: usize) -> std::result::Result<(), std::io::Error> {
        let buffer = frame.data(0).to_owned();

        debug!("{}, {}", buffer.len(), (frame.width() * frame.height() * 3));
        assert!(buffer.len() == (frame.width() * frame.height() * 3) as usize);

        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_raw(frame.width(), frame.height(), buffer).unwrap();
        debug!(
            "saving: {}",
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("out1")
                .join(format!("{}.png", index + 1))
                .display()
        );
        img.save(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/assets/out1")
                .join(format!("{}.png", index + 1)),
        )
        .unwrap();

        Ok(())
    }

    #[test]
    fn test_video_frame_iterator() {
        initialize(log::Level::Error).unwrap();

        let path = Path::new("../../tests/assets/test.mkv");
        let index = 5;

        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();

        let producer = thread::spawn(move || {
            let mut input = input_file(path).unwrap();
            let frames = Decoder::new_with_video(&mut input, index, FrameProcess::Decode).unwrap();

            for (idx, frame) in frames.enumerate() {
                debug!("decoded {}", idx,);
                tx1.send(frame).expect("Failed to send frame to stage 1");
            }
        });

        let handler = thread::spawn(move || {
            let mut scaler = Scaler::from_path(path, index, VideoPixel::RGB24).unwrap();

            for (idx, frame) in rx1
                .iter()
                .filter_map(|frame| {
                    if let Frame::Frame(frame) = frame {
                        Some(frame)
                    } else {
                        None
                    }
                })
                .enumerate()
            {
                if let StreamFrame::Video(frame) = frame {
                    debug!("scaling {}", idx);
                    let processed = scaler.scale_frame(&frame).unwrap();
                    tx2.send(processed)
                        .expect("Failed to send frame to stage 2");
                } else {
                    panic!("Unexpected frame type");
                }
            }
        });

        let collector = thread::spawn(move || {
            for (idx, frame) in rx2.iter().enumerate() {
                debug!("saving {}", idx);
                save_file(&frame, idx).unwrap();
            }
        });

        producer.join().expect("Producer thread panicked");
        debug!("producer finished");
        handler.join().expect("Handler thread panicked");
        debug!("handler finished");
        collector.join().expect("Collector thread panicked");
        debug!("collector finished");
    }

    #[test]
    fn test_audio_frame_iterator() {
        initialize(log::Level::Error).unwrap();

        let buffer = read("../../tests/assets/test.m4a").unwrap();
        let index = 0;
        println!("buffer: {}", buffer.len());

        let mut input = input_buffer(buffer).unwrap();
        let mut resampling = Resampler::new(
            &input.as_ref().stream(index).unwrap(),
            &AudioSpec::new(ChannelLayout::MONO, Sample::I16(SampleType::Planar), 48000),
        )
        .unwrap();
        let frames = Decoder::new_with_audio(input.as_mut(), index, FrameProcess::Decode).unwrap();

        for (idx, frame) in frames.enumerate() {
            let Frame::Frame(StreamFrame::Audio(frame)) = frame else {
                panic!("Unexpected frame type");
            };
            let mut decoded = Vec::new();
            let frame = resampling.resample(&frame).unwrap();
            decoded.extend_from_slice(frame.plane::<i16>(0));
            println!("decoded {}, frame: {}", idx, decoded.len());
        }
    }
}
