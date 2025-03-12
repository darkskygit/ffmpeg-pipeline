use super::*;
use ffmpeg_next::{format::context, sys, Error};
use std::{
    ffi::c_void,
    ptr::{null, null_mut},
};

pub struct BufferedInput {
    _ctx: Box<AVInputContextData>,
    input: Input,
    io_ctx: Box<*mut sys::AVIOContext>,
}

impl BufferedInput {
    pub fn from_reader(mut reader: impl Readable + 'static) -> FFmpegResult<Self> {
        let length = reader.stream_len()?;
        let cursor = Box::new(reader) as Box<dyn Readable>;
        let ctx = Box::new(AVInputContextData { cursor, length });
        let (io_ctx, input) = Self::input_buffer(ctx.as_ref())?;
        Ok(Self {
            _ctx: ctx,
            input,
            io_ctx,
        })
    }

    fn input_buffer(ctx: &AVInputContextData) -> FFmpegResult<(Box<*mut sys::AVIOContext>, Input)> {
        let mut options = Dictionary::new();
        options.set("max_streams", "8192");

        unsafe {
            let avio_ctx = get_avio_context(false, ctx as *const _ as *mut _);
            let mut ps = sys::avformat_alloc_context();
            (*ps).pb = avio_ctx;

            let mut opts = options.disown();
            let res = sys::avformat_open_input(&mut ps, null(), null(), &mut opts);

            Dictionary::own(opts);

            match res {
                0 => match sys::avformat_find_stream_info(ps, null_mut()) {
                    r if r >= 0 => Ok((Box::new(avio_ctx), context::Input::wrap(ps))),
                    e => {
                        sys::avformat_close_input(&mut ps);
                        Err(Error::from(e))
                    }
                },

                e => Err(Error::from(e)),
            }
        }
        .map_err(|e| e.into())
    }
}

impl Drop for BufferedInput {
    fn drop(&mut self) {
        unsafe {
            sys::av_free((*self.io_ctx) as *mut c_void);
        }
    }
}

impl AsRef<Input> for BufferedInput {
    fn as_ref(&self) -> &Input {
        &self.input
    }
}

impl AsMut<Input> for BufferedInput {
    fn as_mut(&mut self) -> &mut Input {
        &mut self.input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Read, path::Path};

    #[test]
    fn test_buffered_input() {
        let path = Path::new("../../tests/assets/test.m4a");
        let mut file = File::open(path).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        let mut input = BufferedInput::from_reader(Cursor::new(data)).unwrap();
        let frames = Decoder::new_with_audio(input.as_mut(), 0, FrameProcess::Decode).unwrap();

        for (idx, frame) in frames.enumerate() {
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
}
