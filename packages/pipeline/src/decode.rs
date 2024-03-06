use super::*;
use ffmpeg_next::{
    codec::{context::Context, decoder::Video as VideoDecoder},
    format::{
        context::{input::PacketIter, Input},
        Pixel,
    },
    util::frame::video::Video as VideoFrame,
    Error as FFmpegError, Packet,
};

enum FrameStatus {
    Raw(Packet),
    Decoded(VideoFrame),
    Eof,
    Error(isize, FFmpegError),
}

#[derive(PartialEq)]
pub enum FrameProcess {
    Passthrough,
    Decode,
}

pub enum Frame {
    Packet(Packet),
    Frame(VideoFrame),
}

pub struct FrameIterator<'i> {
    index: usize,
    video: VideoDecoder,
    packets: PacketIter<'i>,
    process: FrameProcess,
}

impl<'i> FrameIterator<'i> {
    pub fn new(
        handler: &'i mut Input,
        index: usize,
        process: FrameProcess,
    ) -> IoResult<Option<Self>> {
        if let Some(stream) = handler.stream(index) {
            let codec = Context::from_parameters(stream.parameters())?;
            let video = codec.decoder().video()?;
            let packets = handler.packets();

            Ok(Some(Self {
                index,
                video,
                packets,
                process,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn decode_frames(&mut self) -> Option<VideoFrame> {
        let mut decoded = VideoFrame::empty();
        while self.video.receive_frame(&mut decoded).is_ok() {
            return Some(decoded);
        }
        None
    }
}

impl Iterator for FrameIterator<'_> {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let next_frame = loop {
            if let Some((stream, packet)) = self.packets.next() {
                if stream.index() != self.index as usize {
                    continue;
                }
                match self.process {
                    FrameProcess::Passthrough => break FrameStatus::Raw(packet),
                    FrameProcess::Decode => {
                        if let Err(e) = self.video.send_packet(&packet) {
                            break FrameStatus::Error(packet.position(), e);
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
                if let Err(e) = self.video.send_eof() {
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
            std::env::current_dir()
                .unwrap()
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
    fn test_frame_iterator() {
        ffmpeg_init().unwrap();

        let path = Path::new("../../tests/assets/test.mkv");
        let index = 5;

        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();

        let producer = thread::spawn(move || {
            let mut input = input(path).unwrap();
            let frames = FrameIterator::new(&mut input, index, FrameProcess::Decode)
                .unwrap()
                .unwrap();

            for (idx, frame) in frames.enumerate() {
                debug!("decoded {}", idx,);
                tx1.send(frame).expect("Failed to send frame to stage 1");
            }
        });

        let handler = thread::spawn(move || {
            let mut scaler = Scaler::new_from_path(&path, index, Pixel::RGB24).unwrap();

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
                debug!("scaling {}", idx);
                let processed = scaler.scale_frame(&frame).unwrap();
                tx2.send(processed)
                    .expect("Failed to send frame to stage 2");
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
}
