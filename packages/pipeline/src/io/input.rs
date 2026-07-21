use super::*;
use ffmpeg_next::{format::context, sys, Error};
use std::{
    ffi::c_void,
    ffi::CString,
    io::SeekFrom,
    ptr::{null, null_mut},
};

pub struct BufferedInput {
    _ctx: Box<AVInputContextData>,
    input: Input,
    io_ctx: Box<*mut sys::AVIOContext>,
}

impl BufferedInput {
    pub fn from_reader(reader: impl Readable + 'static) -> FFmpegResult<Self> {
        Self::from_reader_with_format(reader, None)
    }

    pub fn from_reader_with_format(
        reader: impl Readable + 'static,
        format: Option<&str>,
    ) -> FFmpegResult<Self> {
        Self::from_reader_with_format_and_options(reader, format, &[])
    }

    pub fn from_reader_with_format_and_options(
        mut reader: impl Readable + 'static,
        format: Option<&str>,
        input_options: &[(&str, &str)],
    ) -> FFmpegResult<Self> {
        let position = reader.stream_position()?;
        let length = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(position))?;
        let cursor = Box::new(reader) as Box<dyn Readable>;
        let ctx = Box::new(AVInputContextData { cursor, length });
        let (io_ctx, input) = Self::input_buffer(ctx.as_ref(), format, input_options)?;
        Ok(Self {
            _ctx: ctx,
            input,
            io_ctx,
        })
    }

    fn input_buffer(
        ctx: &AVInputContextData,
        format: Option<&str>,
        input_options: &[(&str, &str)],
    ) -> FFmpegResult<(Box<*mut sys::AVIOContext>, Input)> {
        let mut options = Dictionary::new();
        options.set("max_streams", "8192");
        for (key, value) in input_options {
            options.set(key, value);
        }

        unsafe {
            let avio_ctx = get_avio_context(false, ctx as *const _ as *mut _);
            let mut ps = sys::avformat_alloc_context();
            (*ps).pb = avio_ctx;

            let format_name = format
                .map(CString::new)
                .transpose()
                .map_err(|_| Error::InvalidData)?;
            let input_format = format_name
                .as_ref()
                .map(|name| sys::av_find_input_format(name.as_ptr()))
                .unwrap_or(null());
            if format.is_some() && input_format.is_null() {
                sys::avformat_free_context(ps);
                return Err(Error::InvalidData.into());
            }
            let mut opts = options.disown();
            let res = sys::avformat_open_input(&mut ps, null(), input_format, &mut opts);

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

    #[test]
    fn test_buffered_input() {
        let mut input = BufferedInput::from_reader_with_format(
            Cursor::new(crate::tests::encoded_ivf(2)),
            Some("ivf"),
        )
        .unwrap();
        let frames = Decoder::new_with_video(input.as_mut(), 0, FrameProcess::Decode).unwrap();

        for (idx, frame) in frames.enumerate() {
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
}
