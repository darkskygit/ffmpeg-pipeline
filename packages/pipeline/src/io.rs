use ffmpeg_next::{
    format::{context::Input, input_with_dictionary},
    media, Dictionary,
};
use std::{io::Result as IoResult, path::Path};

pub fn input<P: AsRef<Path>>(path: P) -> IoResult<Input> {
    let mut options = Dictionary::new();
    options.set("max_streams", "8192");
    input_with_dictionary(&path, options).map_err(|e| e.into())
}

pub fn read_attachment<P: AsRef<Path>>(path: P, index: usize) -> IoResult<Vec<u8>> {
    let mut input = input(&path)?;
    let mut ret = Vec::new();
    for (stream, packet) in input.packets() {
        if stream.index() != index {
            continue;
        }
        if stream.parameters().medium() == media::Type::Attachment {
            if let Some(data) = packet.data() {
                ret.extend_from_slice(data);
            }
        }
    }
    if ret.is_empty() {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("attachment not found: {}", index),
        ))
    } else {
        Ok(ret)
    }
}
