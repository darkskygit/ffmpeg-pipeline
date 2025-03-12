use super::*;
use ffmpeg_next::{format::context, sys, Error};
use std::{
    any::Any,
    ffi::{c_void, CString},
    ptr::null_mut,
};

pub struct BufferedOutput {
    _ctx: Box<AVOutputContextData>,
    output: Output,
    io_ctx: Box<*mut sys::AVIOContext>,
}

impl BufferedOutput {
    pub fn from_writer(writer: impl Writable + 'static, format: &str) -> FFmpegResult<Self> {
        let cursor = Box::new(writer) as Box<dyn Writable>;
        let ctx = Box::new(AVOutputContextData { cursor });
        let (io_ctx, output) = Self::output_buffer(ctx.as_ref(), format)?;
        Ok(Self {
            _ctx: ctx,
            output,
            io_ctx,
        })
    }

    fn output_buffer(
        ctx: &AVOutputContextData,
        format: &str,
    ) -> FFmpegResult<(Box<*mut sys::AVIOContext>, Output)> {
        unsafe {
            let avio_ctx = get_avio_context(true, ctx as *const _ as *mut _);

            let mut ps = sys::avformat_alloc_context();
            let format =
                CString::new(format).map_err(|_| FFmpegError::InvalidFormat(format.into()))?;
            let res = sys::avformat_alloc_output_context2(
                &mut ps,
                null_mut(),
                format.as_ptr(),
                null_mut(),
            );

            (*ps).pb = avio_ctx;

            match res {
                0 => Ok((Box::new(avio_ctx), context::Output::wrap(ps))),
                e => Err(Error::from(e)),
            }
        }
        .map_err(|e| e.into())
    }

    pub fn into_inner<T: Writable>(mut self) -> FFmpegResult<T> {
        use std::io::{Error, ErrorKind};

        unsafe {
            sys::av_free((*self.io_ctx) as *mut c_void);
            (*self.output.as_mut_ptr()).pb = null_mut();
        }

        let cursor: Box<dyn Any> = self._ctx.cursor;
        if let Ok(cursor) = cursor.downcast::<T>() {
            Ok(*cursor)
        } else {
            Err(FFmpegError::Io(Error::new(
                ErrorKind::InvalidData,
                "invalid cursor type",
            )))
        }
    }
}

impl AsRef<Output> for BufferedOutput {
    fn as_ref(&self) -> &Output {
        &self.output
    }
}

impl AsMut<Output> for BufferedOutput {
    fn as_mut(&mut self) -> &mut Output {
        &mut self.output
    }
}

#[cfg(test)]
mod tests {
    use ffmpeg_next::codec::Id;

    use super::*;
    use std::{fs::File, io::Read};

    fn init() {
        ffmpeg_init_with_level(log::Level::Debug);
    }

    fn get_buffer<P: AsRef<Path>>(path: P) -> Vec<u8> {
        let mut file = File::open(path).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        data
    }

    #[test]
    fn test_buffered_input() {
        init();
        let buffer = get_buffer("./tmp/1.m4a");
        let mut input = BufferedInput::from_reader(Cursor::new(buffer)).unwrap();
        let decoder = Decoder::new_with_audio(input.as_mut(), 0, FrameProcess::Decode).unwrap();

        for (idx, frame) in decoder.enumerate() {
            match frame {
                Frame::Frame(StreamFrame::Audio(audio)) => {
                    println!("frame: {:?}", audio.format());
                }
                Frame::Frame(StreamFrame::Video(video)) => {
                    println!("frame: {:?}", video.format());
                }
                Frame::Frame(StreamFrame::Eof) => {
                    println!("eof");
                }
                Frame::Packet(_) => {
                    println!("packet: {}", idx);
                }
            }
        }
    }

    #[test]
    fn test_buffered_output() {
        init();
        let buffer = get_buffer("./tmp/1.m4a");
        let mut input = BufferedInput::from_reader(Cursor::new(buffer)).unwrap();
        let mut output = BufferedOutput::from_writer(Cursor::new(vec![]), "opus").unwrap();

        let decoder = Decoder::new_with_audio(input.as_mut(), 0, FrameProcess::Decode).unwrap();
        let params = EncodeParams::from(&decoder)
            .with_bitrate(64 * 1024)
            .with_vbr(true);
        let mut encoder = Encoder::new(output.as_mut(), Id::OPUS, params).unwrap();
        let mut buffer = AutoAudioBuffer::new(&decoder, &encoder).unwrap();

        // for (idx, frame) in frames.enumerate() {
        //     match frame {
        //         Frame::Frame(StreamFrame::Audio(audio)) => {
        //             println!("frame: {:?}", audio.format());
        //         }
        //         Frame::Frame(StreamFrame::Video(video)) => {
        //             println!("frame: {:?}", video.format());
        //         }
        //         Frame::Frame(StreamFrame::Eof) => {
        //             println!("eof");
        //         }
        //         Frame::Packet(_) => {
        //             println!("packet: {}", idx);
        //         }
        //     }
        // }
    }
}
