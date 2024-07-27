mod buffer;

use super::*;
use buffer::BufferedInput;
use ffmpeg_next::{
    format::{input_with_dictionary, output_with},
    media, Dictionary,
};
use std::{
    io::{Cursor, Read, Seek},
    path::Path,
};

pub fn input_buffer(data: Vec<u8>) -> FFmpegResult<BufferedInput<Cursor<Vec<u8>>>> {
    input_reader(Cursor::new(data))
}

pub fn input_reader<R: Read + Seek>(reader: R) -> FFmpegResult<BufferedInput<R>> {
    BufferedInput::from_reader(reader)
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
    use rayon::prelude::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn test_read_attachment() {
        ffmpeg_init().unwrap();

        let paths = std::fs::read_dir("/Users/ds/Resilio Sync/CG")
            .unwrap()
            .filter_map(|p| p.ok().map(|p| p.path()))
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_lowercase())
                        .unwrap_or_default()
                        .ends_with("mkv")
            })
            .collect::<Vec<_>>();

        paths.par_iter().for_each(|file| {
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| {
                match parse_video_group(&file, FrameCalculation::Skip) {
                    Ok(groups) => {
                        for group in groups.values() {
                            if group.stream_type != "Attachment" {
                                continue;
                            }
                            assert!(read_attachment(&file, group.stream as usize).is_ok());
                        }
                    }
                    Err(e) => debug!("file {}: error: {:?}", file.display(), e),
                }
            })) {
                debug!("file {}: crash: {:?}", file.display(), e);
            }
        });
    }
}
