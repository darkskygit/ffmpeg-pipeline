use super::*;
use ffmpeg_next::{
    codec::{context::Context, decoder::Video as VideoDecoder},
    format::{
        context::{input::PacketIter, Input},
        Pixel,
    },
    software::scaling::{context::Context as ScalerContext, flag::Flags as ScalerFlags},
    util::frame::video::Video as VideoFrame,
    Error as FFmpegError, Stream,
};

enum FrameStatus {
    Decoded(VideoFrame),
    Eof,
    Error(isize, FFmpegError),
}

pub struct FrameIterator<'i> {
    index: usize,
    video: VideoDecoder,
    packets: PacketIter<'i>,
}

impl<'i> FrameIterator<'i> {
    pub fn new(handler: &'i mut Input, index: usize) -> IoResult<Option<Self>> {
        if let Some(stream) = handler.stream(index) {
            let codec = Context::from_parameters(stream.parameters())?;
            let video = codec.decoder().video()?;
            let packets = handler.packets();

            Ok(Some(Self {
                index,
                video,
                packets,
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
    type Item = VideoFrame;

    fn next(&mut self) -> Option<Self::Item> {
        let next_frame = loop {
            if let Some((stream, packet)) = self.packets.next() {
                if stream.index() != self.index as usize {
                    continue;
                }

                if let Err(e) = self.video.send_packet(&packet) {
                    break FrameStatus::Error(packet.position(), e);
                }

                if let Some(frame) = self.decode_frames() {
                    break FrameStatus::Decoded(frame);
                }
            } else {
                break FrameStatus::Eof;
            }
        };

        match next_frame {
            FrameStatus::Decoded(frame) => Some(frame),
            FrameStatus::Eof => {
                if let Err(e) = self.video.send_eof() {
                    warn!("Failed to send EOF to stream {}: {}", self.index, e);
                }
                self.decode_frames()
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

pub struct Scaler {
    scaler: ScalerContext,
}

impl Scaler {
    pub fn new(info: &VideoInfo, dst_format: Pixel) -> IoResult<Self> {
        debug!(
            "stream: {}, size: {} x {}, pixel: {:?}",
            info.stream, info.size.width, info.size.height, info.pixel
        );
        Ok(Self {
            scaler: ScalerContext::get(
                info.pixel,
                info.size.width as u32,
                info.size.height as u32,
                dst_format,
                info.size.width as u32,
                info.size.height as u32,
                ScalerFlags::SPLINE,
            )?,
        })
    }

    pub fn new_from_stream(stream: &Stream, dst_format: Pixel) -> IoResult<Self> {
        let info = parse::parse_stream_info(stream)?;
        Self::new(&info, dst_format)
    }

    pub fn new_from_path(path: &Path, index: usize, dst_format: Pixel) -> IoResult<Self> {
        let input = input(path)?;
        let stream = input
            .stream(index)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Stream not found"))?;
        Self::new_from_stream(&stream, dst_format)
    }

    pub fn scale_frame(&mut self, frame: &VideoFrame) -> IoResult<VideoFrame> {
        let mut rgb_frame = VideoFrame::empty();
        self.scaler.run(frame, &mut rgb_frame)?;
        Ok(rgb_frame)
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
            let frames = FrameIterator::new(&mut input, index).unwrap().unwrap();

            for (idx, frame) in frames.enumerate() {
                debug!("decoded {}", idx,);
                tx1.send(frame).expect("Failed to send frame to stage 1");
            }
        });

        let handler = thread::spawn(move || {
            let mut scaler = Scaler::new_from_path(&path, index, Pixel::RGB24).unwrap();

            for (idx, frame) in rx1.iter().enumerate() {
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
