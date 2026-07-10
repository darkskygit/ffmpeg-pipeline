use ffmpeg_next::ffi::{
    av_malloc, avio_alloc_context, AVIOContext, AVERROR_EOF, AVSEEK_SIZE, SEEK_CUR, SEEK_END,
    SEEK_SET,
};
use std::{
    any::Any,
    io::{Read, Seek, SeekFrom, Write},
    os::raw::c_void,
    slice::{from_raw_parts, from_raw_parts_mut},
};

pub trait Readable: Read + Seek {}

impl<T: Read + Seek> Readable for T {}

pub trait Writable: Seek + Write + Any {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Seek + Write + Any> Writable for T {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[repr(C)]
pub struct AVInputContextData {
    pub(super) cursor: Box<dyn Readable>,
    pub(super) length: u64,
}

#[repr(C)]
pub struct AVOutputContextData {
    pub(super) cursor: Box<dyn Writable>,
}

#[inline]
unsafe extern "C" fn read(opaque: *mut c_void, buf: *mut u8, buf_size: i32) -> i32 {
    let ctx = &mut *(opaque as *mut AVInputContextData);
    let slice = from_raw_parts_mut(buf, buf_size as usize);
    match ctx.cursor.read(slice) {
        Ok(size) => {
            if size != 0 {
                size as i32
            } else {
                AVERROR_EOF
            }
        }
        Err(_) => -1,
    }
}

#[inline]
unsafe extern "C" fn write(opaque: *mut c_void, buf: *const u8, buf_size: i32) -> i32 {
    let ctx = &mut *(opaque as *mut AVOutputContextData);
    let slice = from_raw_parts(buf, buf_size as usize);
    match ctx.cursor.write(slice) {
        Ok(size) => size as i32,
        Err(_) => 0,
    }
}

#[inline]
unsafe extern "C" fn seek(opaque: *mut c_void, offset: i64, whence: i32) -> i64 {
    let ctx = &mut *(opaque as *mut AVInputContextData);
    match whence {
        AVSEEK_SIZE => ctx.length as i64,
        _ => {
            let pos = match whence {
                SEEK_SET => SeekFrom::Start(offset as u64),
                SEEK_CUR => SeekFrom::Current(offset),
                SEEK_END => SeekFrom::End(offset),
                _ => return -1,
            };

            match ctx.cursor.seek(pos) {
                Ok(pos) => pos as i64,
                Err(_e) => -1,
            }
        }
    }
}

#[inline]
unsafe extern "C" fn seek_output(opaque: *mut c_void, offset: i64, whence: i32) -> i64 {
    let ctx = &mut *(opaque as *mut AVOutputContextData);
    if whence == AVSEEK_SIZE {
        let position = match ctx.cursor.stream_position() {
            Ok(position) => position,
            Err(_) => return -1,
        };
        let size = match ctx.cursor.seek(SeekFrom::End(0)) {
            Ok(size) => size,
            Err(_) => return -1,
        };
        let _ = ctx.cursor.seek(SeekFrom::Start(position));
        return size as i64;
    }
    let position = match whence {
        SEEK_SET => SeekFrom::Start(offset as u64),
        SEEK_CUR => SeekFrom::Current(offset),
        SEEK_END => SeekFrom::End(offset),
        _ => return -1,
    };
    ctx.cursor
        .seek(position)
        .map(|position| position as i64)
        .unwrap_or(-1)
}

pub unsafe fn get_avio_context(writable: bool, opaque: *mut c_void) -> *mut AVIOContext {
    avio_alloc_context(
        av_malloc(4096) as *mut u8,
        4096,
        if writable { 1 } else { 0 },
        opaque,
        if writable { None } else { Some(read) },
        if writable { Some(write) } else { None },
        if writable {
            Some(seek_output)
        } else {
            Some(seek)
        },
    )
}
