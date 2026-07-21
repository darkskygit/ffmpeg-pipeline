mod input;
mod output;
mod utils;

use super::*;
use ffmpeg_next::{
    format::{input_with_dictionary, output_with},
    media, Dictionary,
};
use input::BufferedInput;
use output::BufferedOutput;
use std::{io::Cursor, path::Path};
use utils::{get_avio_context, AVInputContextData, AVOutputContextData, Readable, Writable};

#[inline(always)]
pub fn input_buffer(data: Vec<u8>) -> FFmpegResult<BufferedInput> {
    input_reader(Cursor::new(data))
}

#[inline(always)]
pub fn input_buffer_with_format(data: Vec<u8>, format: &str) -> FFmpegResult<BufferedInput> {
    BufferedInput::from_reader_with_format(Cursor::new(data), Some(format))
}

#[inline(always)]
pub fn input_buffer_with_format_options(
    data: Vec<u8>,
    format: &str,
    options: &[(&str, &str)],
) -> FFmpegResult<BufferedInput> {
    BufferedInput::from_reader_with_format_and_options(Cursor::new(data), Some(format), options)
}

pub fn input_reader<R: Readable + 'static>(reader: R) -> FFmpegResult<BufferedInput> {
    BufferedInput::from_reader(reader)
}

#[inline(always)]
pub fn output_buffer(format: &str) -> FFmpegResult<BufferedOutput> {
    output_writer(Cursor::new(vec![]), format)
}

pub fn output_writer<W: Writable + 'static>(
    writer: W,
    format: &str,
) -> FFmpegResult<BufferedOutput> {
    BufferedOutput::from_writer(writer, format)
}

pub fn input_file<P: AsRef<Path>>(path: P) -> FFmpegResult<Input> {
    let mut options = Dictionary::new();
    options.set("max_streams", "8192");
    input_with_dictionary(&path, options).map_err(|e| e.into())
}

/// guess the output format from the file extension
pub fn output_file<P: AsRef<Path>>(path: P) -> FFmpegResult<Output> {
    let mut options = Dictionary::new();
    options.set("max_streams", "8192");
    output_with(&path, options).map_err(|e| e.into())
}

pub fn read_attachment<P: AsRef<Path>>(path: P, index: usize) -> FFmpegResult<Vec<u8>> {
    let input = input_file(&path)?;
    let mut ret = Vec::new();
    if let Some(stream) = input.stream(index) {
        if stream.parameters().medium() == media::Type::Attachment {
            let params = stream.parameters();
            unsafe {
                let data = (*params.as_ptr()).extradata as *const u8;
                let size = (*params.as_ptr()).extradata_size as usize;

                if !data.is_null() && size > 0 {
                    ret.extend_from_slice(std::slice::from_raw_parts(data, size));
                }
            }
        }
    }
    if ret.is_empty() {
        Err(FFmpegError::AttachmentNotFound(index))
    } else {
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_attachment() {
        initialize(log::Level::Error).unwrap();
        let path = std::env::temp_dir().join(format!(
            "ffmpeg-pipeline-attachment-{}.ivf",
            std::process::id()
        ));
        std::fs::write(&path, crate::tests::encoded_ivf(1)).unwrap();
        let result = read_attachment(&path, 0);
        std::fs::remove_file(path).unwrap();
        assert!(matches!(result, Err(FFmpegError::AttachmentNotFound(0))));
    }
}
