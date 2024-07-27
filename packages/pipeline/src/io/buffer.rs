use super::*;
use ffmpeg_next::{format::context, sys, Error};
use std::{
    ffi::c_void,
    io::{Cursor, Read, Seek},
    ptr::null_mut,
};

pub struct BufferedInput {
    _cursor: Box<Cursor<Vec<u8>>>,
    input: Input,
    io_ctx: Box<*mut sys::AVIOContext>,
}

impl BufferedInput {
    pub fn new(data: Vec<u8>) -> FFmpegResult<Self> {
        let cursor = Box::new(Cursor::new(data));

        let (io_ctx, input) = Self::input_buffer(cursor.as_ref())?;
        Ok(Self {
            _cursor: cursor,
            input,
            io_ctx,
        })
    }

    pub fn input_buffer(
        cursor: &Cursor<Vec<u8>>,
    ) -> FFmpegResult<(Box<*mut sys::AVIOContext>, Input)> {
        let mut options = Dictionary::new();
        options.set("max_streams", "8192");

        unsafe {
            let avio_ctx = sys::avio_alloc_context(
                sys::av_malloc(4096) as *mut u8, // buffer size
                4096,                            // buffer size
                0,                               // write flag, 0 means read-only
                cursor as *const _ as *mut _,
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
        let cursor = &mut *(opaque as *mut Cursor<Vec<u8>>);
        let slice = std::slice::from_raw_parts_mut(buf, buf_size as usize);
        match cursor.read(slice) {
            Ok(size) => size as i32,
            Err(_) => -1, // return -1 indicates a read error
        }
    }

    unsafe extern "C" fn seek(opaque: *mut std::os::raw::c_void, offset: i64, whence: i32) -> i64 {
        let cursor = &mut *(opaque as *mut Cursor<Vec<u8>>);
        match whence {
            sys::AVSEEK_SIZE => cursor.get_ref().len() as i64,
            _ => {
                let pos = match whence {
                    sys::SEEK_SET => std::io::SeekFrom::Start(offset as u64),
                    sys::SEEK_CUR => std::io::SeekFrom::Current(offset),
                    sys::SEEK_END => std::io::SeekFrom::End(offset),
                    _ => return -1,
                };

                match cursor.seek(pos) {
                    Ok(pos) => pos as i64,
                    Err(_) => -1,
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
            sys::av_free(*self.io_ctx.as_ref() as *mut c_void);
        }
    }
}
