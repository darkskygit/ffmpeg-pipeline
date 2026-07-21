use super::*;
use ffmpeg_next::{format::context, sys, Error};
use std::{ffi::CString, mem, ptr::null_mut};

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

    pub fn into_inner<T: Writable>(self) -> FFmpegResult<T> {
        use std::io::{Error, ErrorKind};

        let Self {
            mut _ctx,
            mut output,
            mut io_ctx,
        } = self;
        unsafe {
            sys::avio_context_free(io_ctx.as_mut());
            (*output.as_mut_ptr()).pb = null_mut();
            sys::avformat_free_context(output.as_mut_ptr());
        }
        // ffmpeg-next 7.1 always closes Output.pb as URL IO. The custom AVIO and
        // format context are already released above, so its destructor must not run.
        mem::forget(output);

        let cursor = _ctx.cursor.into_any();
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
    use super::*;

    fn init() {
        initialize(log::Level::Debug).unwrap();
    }

    #[test]
    fn test_buffered_input() {
        init();
        let mut input = BufferedInput::from_reader_with_format(
            Cursor::new(crate::tests::encoded_ivf(2)),
            Some("ivf"),
        )
        .unwrap();
        let decoder = Decoder::new_with_video(input.as_mut(), 0, FrameProcess::Decode).unwrap();

        for (idx, frame) in decoder.enumerate() {
            let frame = frame.unwrap();
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
        let output = crate::tests::encoded_ogg();
        assert!(output.starts_with(b"OggS"));

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
