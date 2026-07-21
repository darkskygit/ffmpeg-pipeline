use super::*;
use ffmpeg_next::{codec::context::Context, error::EAGAIN, Error as FFmpegOrigError, Packet};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameProcess {
    Passthrough,
    Decode,
}

pub enum Frame {
    Packet(Packet),
    Frame(StreamFrame),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DecoderState {
    Reading,
    SendingEof,
    Draining,
    Done,
}

enum ReceiveStatus<T> {
    Frame(T),
    Again,
    Eof,
}

#[cfg(test)]
#[derive(Default)]
struct DecoderStats {
    receive_again: usize,
    eof_sends: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReceiveEnd {
    Again,
    Eof,
}

fn drain_available<T, E>(
    mut receive: impl FnMut() -> Result<ReceiveStatus<T>, E>,
) -> Result<(VecDeque<T>, ReceiveEnd), E> {
    let mut frames = VecDeque::new();
    loop {
        match receive()? {
            ReceiveStatus::Frame(frame) => frames.push_back(frame),
            ReceiveStatus::Again => return Ok((frames, ReceiveEnd::Again)),
            ReceiveStatus::Eof => return Ok((frames, ReceiveEnd::Eof)),
        }
    }
}

/// Iterates packets or decoded frames for one stream.
///
/// Passthrough yields every packet from the selected stream and then `None` at
/// demux EOF. Decode mode drains all frames made available by a packet before
/// reading another packet, sends EOF once, drains delayed frames, and returns
/// `None` only after decoder EOF. Every other demux or codec failure is yielded.
pub struct Decoder<'i> {
    index: usize,
    decoder: StreamDecoder,
    input: &'i mut Input,
    process: FrameProcess,
    time_base: Rational,
    state: DecoderState,
    pending_packet: Option<Packet>,
    drain_again: bool,
    ready_frames: VecDeque<StreamFrame>,
    receive_end: Option<ReceiveEnd>,
    #[cfg(test)]
    stats: DecoderStats,
}

impl<'i> Decoder<'i> {
    pub fn new_with_video(
        input: &'i mut Input,
        index: usize,
        process: FrameProcess,
    ) -> FFmpegResult<Self> {
        let (decoder, time_base) = {
            let stream = input
                .stream(index)
                .ok_or(FFmpegError::StreamNotFound(index))?;
            let codec = Context::from_parameters(stream.parameters())?;
            (
                StreamDecoder::Video(codec.decoder().video()?),
                stream.time_base(),
            )
        };
        Ok(Self::new(input, index, process, decoder, time_base))
    }

    pub fn new_with_audio(
        input: &'i mut Input,
        index: usize,
        process: FrameProcess,
    ) -> FFmpegResult<Self> {
        let (decoder, time_base) = {
            let stream = input
                .stream(index)
                .ok_or(FFmpegError::StreamNotFound(index))?;
            let codec = Context::from_parameters(stream.parameters())?;
            (
                StreamDecoder::Audio(codec.decoder().audio()?),
                stream.time_base(),
            )
        };
        Ok(Self::new(input, index, process, decoder, time_base))
    }

    fn new(
        input: &'i mut Input,
        index: usize,
        process: FrameProcess,
        decoder: StreamDecoder,
        time_base: Rational,
    ) -> Self {
        Self {
            index,
            decoder,
            input,
            process,
            time_base,
            state: DecoderState::Reading,
            pending_packet: None,
            drain_again: false,
            ready_frames: VecDeque::new(),
            receive_end: None,
            #[cfg(test)]
            stats: DecoderStats::default(),
        }
    }

    pub fn get_decoder(&self) -> &StreamDecoder {
        &self.decoder
    }

    pub fn time_base(&self) -> Rational {
        self.time_base
    }

    fn receive_frame(&mut self) -> FFmpegResult<ReceiveStatus<StreamFrame>> {
        let result = match self.decoder {
            StreamDecoder::Audio(ref mut decoder) => {
                let mut frame = AudioFrame::empty();
                decoder
                    .receive_frame(&mut frame)
                    .map(|_| ReceiveStatus::Frame(StreamFrame::Audio(frame)))
            }
            StreamDecoder::Video(ref mut decoder) => {
                let mut frame = VideoFrame::empty();
                decoder
                    .receive_frame(&mut frame)
                    .map(|_| ReceiveStatus::Frame(StreamFrame::Video(frame)))
            }
        };
        match result {
            Ok(frame) => Ok(frame),
            Err(error) if is_again(error) => Ok(ReceiveStatus::Again),
            Err(FFmpegOrigError::Eof) => Ok(ReceiveStatus::Eof),
            Err(source) => Err(FFmpegError::decoder(
                "receive frame",
                self.index,
                None,
                source,
            )),
        }
    }

    fn send_packet(&mut self, packet: &Packet) -> Result<(), FFmpegOrigError> {
        match self.decoder {
            StreamDecoder::Audio(ref mut decoder) => decoder.send_packet(packet),
            StreamDecoder::Video(ref mut decoder) => decoder.send_packet(packet),
        }
    }

    fn send_eof(&mut self) -> Result<(), FFmpegOrigError> {
        match self.decoder {
            StreamDecoder::Audio(ref mut decoder) => decoder.send_eof(),
            StreamDecoder::Video(ref mut decoder) => decoder.send_eof(),
        }
    }

    fn read_selected_packet(&mut self) -> FFmpegResult<Option<Packet>> {
        loop {
            let mut packet = Packet::empty();
            match packet.read(self.input) {
                Ok(()) if packet.stream() == self.index => return Ok(Some(packet)),
                Ok(()) => continue,
                Err(FFmpegOrigError::Eof) => return Ok(None),
                Err(source) => return Err(FFmpegError::decoder("demux", self.index, None, source)),
            }
        }
    }

    fn next_packet(&mut self) -> Option<FFmpegResult<Frame>> {
        match self.read_selected_packet() {
            Ok(Some(packet)) => Some(Ok(Frame::Packet(packet))),
            Ok(None) => {
                self.state = DecoderState::Done;
                None
            }
            Err(error) => {
                self.state = DecoderState::Done;
                Some(Err(error))
            }
        }
    }

    fn fail(&mut self, error: FFmpegError) -> Option<FFmpegResult<Frame>> {
        self.state = DecoderState::Done;
        Some(Err(error))
    }
}

impl Iterator for Decoder<'_> {
    type Item = FFmpegResult<Frame>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state == DecoderState::Done {
            return None;
        }
        if self.process == FrameProcess::Passthrough {
            return self.next_packet();
        }

        loop {
            if let Some(frame) = self.ready_frames.pop_front() {
                return Some(Ok(Frame::Frame(frame)));
            }
            if let Some(end) = self.receive_end.take() {
                if end == ReceiveEnd::Eof {
                    self.state = DecoderState::Done;
                    return None;
                }
            } else {
                match drain_available(|| self.receive_frame()) {
                    Ok((frames, end)) => {
                        self.ready_frames = frames;
                        self.receive_end = Some(end);
                        self.drain_again = false;
                        #[cfg(test)]
                        if end == ReceiveEnd::Again {
                            self.stats.receive_again += 1;
                        }
                        continue;
                    }
                    Err(error) => return self.fail(error),
                }
            }

            match self.state {
                DecoderState::Reading => {
                    if self.pending_packet.is_none() {
                        match self.read_selected_packet() {
                            Ok(Some(packet)) => self.pending_packet = Some(packet),
                            Ok(None) => {
                                self.state = DecoderState::SendingEof;
                                continue;
                            }
                            Err(error) => return self.fail(error),
                        }
                    }
                    let Some(packet) = self.pending_packet.take() else {
                        continue;
                    };
                    let position = packet.position();
                    match self.send_packet(&packet) {
                        Ok(()) => {}
                        Err(error) if is_again(error) => self.pending_packet = Some(packet),
                        Err(source) => {
                            return self.fail(FFmpegError::decoder(
                                "send packet",
                                self.index,
                                Some(position),
                                source,
                            ));
                        }
                    }
                }
                DecoderState::SendingEof => {
                    #[cfg(test)]
                    {
                        self.stats.eof_sends += 1;
                    }
                    match self.send_eof() {
                        Ok(()) => self.state = DecoderState::Draining,
                        Err(error) if is_again(error) => continue,
                        Err(FFmpegOrigError::Eof) => {
                            self.state = DecoderState::Done;
                            return None;
                        }
                        Err(source) => {
                            return self
                                .fail(FFmpegError::decoder("flush", self.index, None, source));
                        }
                    }
                }
                DecoderState::Draining => {
                    if self.drain_again {
                        return self.fail(FFmpegError::InvalidFormat(format!(
                            "decoder stream {} returned EAGAIN while draining",
                            self.index
                        )));
                    }
                    self.drain_again = true;
                }
                DecoderState::Done => return None,
            }
        }
    }
}

fn is_again(error: FFmpegOrigError) -> bool {
    matches!(error, FFmpegOrigError::Other { errno } if errno == EAGAIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_drains_to_stable_eof_and_preserves_time_base() {
        initialize(log::Level::Error).unwrap();
        let mut input = input_buffer_with_format(crate::tests::encoded_ivf(4), "ivf").unwrap();
        let expected_time_base = input.as_ref().stream(0).unwrap().time_base();
        let mut decoder = Decoder::new_with_video(input.as_mut(), 0, FrameProcess::Decode).unwrap();
        assert_eq!(decoder.time_base(), expected_time_base);

        let mut frames = 0;
        let mut timestamps = Vec::new();
        for frame in decoder.by_ref() {
            let Frame::Frame(StreamFrame::Video(frame)) = frame.unwrap() else {
                panic!("unexpected frame type");
            };
            frames += 1;
            timestamps.push(frame.timestamp());
        }
        assert_eq!(frames, 4);
        assert!(timestamps.iter().all(Option::is_some));
        assert!(timestamps.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(decoder.stats.receive_again > 0);
        assert_eq!(decoder.stats.eof_sends, 1);
        assert!(decoder.next().is_none());
    }

    #[test]
    fn receive_drain_handles_zero_multiple_and_delayed_frames() {
        let mut zero = VecDeque::from([ReceiveStatus::<u8>::Again]);
        let (frames, end) = drain_available(|| Ok::<_, ()>(zero.pop_front().unwrap())).unwrap();
        assert!(frames.is_empty());
        assert_eq!(end, ReceiveEnd::Again);

        let mut multiple = VecDeque::from([
            ReceiveStatus::Frame(1),
            ReceiveStatus::Frame(2),
            ReceiveStatus::Again,
        ]);
        let (frames, end) = drain_available(|| Ok::<_, ()>(multiple.pop_front().unwrap())).unwrap();
        assert_eq!(frames, VecDeque::from([1, 2]));
        assert_eq!(end, ReceiveEnd::Again);

        let mut delayed = VecDeque::from([ReceiveStatus::Frame(3), ReceiveStatus::Eof]);
        let (frames, end) = drain_available(|| Ok::<_, ()>(delayed.pop_front().unwrap())).unwrap();
        assert_eq!(frames, VecDeque::from([3]));
        assert_eq!(end, ReceiveEnd::Eof);
    }

    #[test]
    fn passthrough_yields_packets_and_invalid_stream_is_an_error() {
        initialize(log::Level::Error).unwrap();
        let mut input = input_buffer_with_format(crate::tests::encoded_ivf(2), "ivf").unwrap();
        assert!(matches!(
            Decoder::new_with_video(input.as_mut(), 2, FrameProcess::Decode),
            Err(FFmpegError::StreamNotFound(2))
        ));

        let mut input = input_buffer_with_format(crate::tests::encoded_ivf(2), "ivf").unwrap();
        let packets = Decoder::new_with_video(input.as_mut(), 0, FrameProcess::Passthrough)
            .unwrap()
            .map(|packet| match packet.unwrap() {
                Frame::Packet(packet) => packet.size(),
                Frame::Frame(_) => panic!("unexpected decoded frame"),
            })
            .collect::<Vec<_>>();
        assert!(!packets.is_empty());
        assert!(packets.iter().all(|size| *size > 0));
    }

    #[test]
    fn decoded_planes_expose_stride_and_truncated_input_is_an_error() {
        initialize(log::Level::Error).unwrap();
        let mut input = input_buffer_with_format(crate::tests::encoded_ivf(1), "ivf").unwrap();
        let frame = Decoder::new_with_video(input.as_mut(), 0, FrameProcess::Decode)
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let Frame::Frame(StreamFrame::Video(frame)) = frame else {
            panic!("unexpected frame type");
        };
        assert!(frame.stride(0) >= frame.width() as usize);
        assert!(frame.data(0).len() >= frame.stride(0) * frame.height() as usize);

        let mut bytes = crate::tests::encoded_ivf(4);
        bytes.truncate(bytes.len() - 8);
        let mut input = input_buffer_with_format(bytes, "ivf").unwrap();
        let result = Decoder::new_with_video(input.as_mut(), 0, FrameProcess::Decode)
            .unwrap()
            .collect::<FFmpegResult<Vec<_>>>();
        assert!(matches!(result, Err(FFmpegError::Decoder { .. })));
    }
}
