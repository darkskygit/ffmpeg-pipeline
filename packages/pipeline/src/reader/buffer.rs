use super::*;
use ffmpeg_next::{format::context, sys, Error};
use std::{ffi::c_void, io::SeekFrom, ptr::null_mut};

pub struct BufferedInput {
    _ctx: Box<AVIOContextData>,
    input: Input,
    io_ctx: Box<*mut sys::AVIOContext>,
}

pub trait Readable: std::io::Read + std::io::Seek {}

impl<T: std::io::Read + std::io::Seek> Readable for T {}

pub struct AVIOContextData {
    cursor: Box<dyn Readable>,
    length: u64,
}

impl BufferedInput {
    pub fn from_reader(mut reader: impl Readable + 'static) -> FFmpegResult<Self> {
        let length = reader.stream_len()?;
        let cursor = Box::new(reader) as Box<dyn Readable>;
        let ctx = Box::new(AVIOContextData { cursor, length });
        let (io_ctx, input) = Self::input_buffer(ctx.as_ref())?;
        Ok(Self {
            _ctx: ctx,
            input,
            io_ctx,
        })
    }

    fn input_buffer(ctx: &AVIOContextData) -> FFmpegResult<(Box<*mut sys::AVIOContext>, Input)> {
        let mut options = Dictionary::new();
        options.set("max_streams", "8192");

        unsafe {
            let avio_ctx = sys::avio_alloc_context(
                sys::av_malloc(4096) as *mut u8,
                4096,
                0,
                ctx as *const _ as *mut _,
                Some(Self::read),
                None,
                Some(Self::seek),
            );

            let mut ps = sys::avformat_alloc_context();
            (*ps).pb = avio_ctx;

            let mut opts = options.disown();
            let res = sys::avformat_open_input(&mut ps, null_mut(), null_mut(), &mut opts);

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

    unsafe extern "C" fn read(
        opaque: *mut std::os::raw::c_void,
        buf: *mut u8,
        buf_size: i32,
    ) -> i32 {
        let ctx = &mut *(opaque as *mut AVIOContextData);
        let slice = std::slice::from_raw_parts_mut(buf, buf_size as usize);
        match ctx.cursor.read(slice) {
            Ok(size) => (size != 0)
                .then_some(size as i32)
                .unwrap_or(sys::AVERROR_EOF),
            Err(_) => -1,
        }
    }

    unsafe extern "C" fn seek(opaque: *mut std::os::raw::c_void, offset: i64, whence: i32) -> i64 {
        let ctx = &mut *(opaque as *mut AVIOContextData);
        match whence {
            sys::AVSEEK_SIZE => ctx.length as i64,
            _ => {
                let pos = match whence {
                    sys::SEEK_SET => SeekFrom::Start(offset as u64),
                    sys::SEEK_CUR => SeekFrom::Current(offset),
                    sys::SEEK_END => SeekFrom::End(offset),
                    _ => return -1,
                };

                match ctx.cursor.seek(pos) {
                    Ok(pos) => pos as i64,
                    Err(_e) => -1,
                }
            }
        }
    }

    pub fn as_ref(&self) -> &Input {
        &self.input
    }

    pub fn as_mut(&mut self) -> &mut Input {
        &mut self.input
    }
}

impl Drop for BufferedInput {
    fn drop(&mut self) {
        unsafe {
            sys::av_free((*self.io_ctx) as *mut c_void);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Read, path::Path};

    #[test]
    fn test_buffered_input() {
        let path = Path::new("../../tests/assets/中恵光城-Brightly horizon.m4a");
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
