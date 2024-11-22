use super::*;
use ffmpeg_next::{format::context, sys, Error};
use std::{
    any::Any,
    ffi::{c_void, CString},
    io::SeekFrom,
    ptr::null_mut,
};

pub struct BufferedOutput {
    _ctx: Box<AVIOContextData>,
    output: Output,
    io_ctx: Box<*mut sys::AVIOContext>,
}

pub trait Writable: Seek + Write + Any {}

impl<T: Seek + Write + Any> Writable for T {}

pub struct AVIOContextData {
    cursor: Box<dyn Writable>,
}

impl BufferedOutput {
    pub fn from_writer(writer: impl Writable + 'static, format: &str) -> FFmpegResult<Self> {
        let cursor = Box::new(writer) as Box<dyn Writable>;
        let ctx = Box::new(AVIOContextData { cursor });
        let (io_ctx, output) = Self::output_buffer(ctx.as_ref(), format)?;
        Ok(Self {
            _ctx: ctx,
            output,
            io_ctx,
        })
    }

    fn output_buffer(
        ctx: &AVIOContextData,
        format: &str,
    ) -> FFmpegResult<(Box<*mut sys::AVIOContext>, Output)> {
        unsafe {
            let avio_ctx = sys::avio_alloc_context(
                sys::av_malloc(4096) as *mut u8,
                4096,
                1,
                ctx as *const _ as *mut _,
                None,
                Some(Self::write),
                Some(Self::seek),
            );

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

    unsafe extern "C" fn write(
        opaque: *mut std::os::raw::c_void,
        buf: *mut u8,
        buf_size: i32,
    ) -> i32 {
        let ctx = &mut *(opaque as *mut AVIOContextData);
        let slice = std::slice::from_raw_parts(buf, buf_size as usize);
        println!("write: {}", slice.len());
        match ctx.cursor.write(slice) {
            Ok(size) => size as i32,
            Err(_) => 0, // You can handle specific errors if needed
        }
    }

    unsafe extern "C" fn seek(opaque: *mut std::os::raw::c_void, offset: i64, whence: i32) -> i64 {
        let ctx = &mut *(opaque as *mut AVIOContextData);
        match whence {
            _ => {
                let pos = match whence {
                    sys::SEEK_SET => SeekFrom::Start(offset as u64),
                    sys::SEEK_CUR => SeekFrom::Current(offset),
                    sys::SEEK_END => SeekFrom::End(offset),
                    _ => return -1,
                };
                println!("pos: {:?}", pos);
                match ctx.cursor.seek(pos) {
                    Ok(pos) => pos as i64,
                    Err(_e) => -1,
                }
            }
        }
    }

    pub fn as_ref(&self) -> &Output {
        &self.output
    }

    pub fn as_mut(&mut self) -> &mut Output {
        &mut self.output
    }

    pub fn into_inner<T: Writable>(mut self) -> Result<T, ()> {
        unsafe {
            sys::av_free((*self.io_ctx) as *mut c_void);
            (*self.output.as_mut_ptr()).pb = null_mut();
        }

        let cursor: Box<dyn Any> = self._ctx.cursor;
        if let Ok(cursor) = cursor.downcast::<T>() {
            Ok(*cursor)
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Read};

    #[test]
    fn test_buffered_input() {
        ffmpeg_init();

        let mut file = File::open("../../tests/assets/1.m4a").unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        let mut input = BufferedInput::from_reader(Cursor::new(data)).unwrap();
        let frames = Decoder::new_with_audio(input.as_mut(), 0, FrameProcess::Decode).unwrap();

        let mut output = BufferedOutput::from_writer(Cursor::new(vec![]), "opus").unwrap();

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
