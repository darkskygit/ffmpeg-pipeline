use std::{collections::HashMap, path::PathBuf};

use ffmpeg_next::{codec, format, Rational};

use crate::FFmpegResult;

#[derive(Clone, Debug, Default)]
pub struct RemuxStream {
    pub input_index: usize,
    pub title: Option<String>,
    pub language: Option<String>,
    pub filename: Option<String>,
    pub mimetype: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RemuxRequest {
    pub input: PathBuf,
    pub output: PathBuf,
    pub streams: Vec<RemuxStream>,
}

pub fn remux(request: &RemuxRequest) -> FFmpegResult {
    let mut input = format::input(&request.input)?;
    let mut output = format::output(&request.output)?;
    let mut stream_mapping = HashMap::<usize, usize>::new();
    let mut input_time_bases = HashMap::<usize, Rational>::new();
    let mut stream_metadata = HashMap::<usize, RemuxStream>::new();

    for stream in &request.streams {
        if stream_mapping.contains_key(&stream.input_index) {
            continue;
        }
        let Some(input_stream) = input.stream(stream.input_index) else {
            continue;
        };
        let mut output_stream = output.add_stream(codec::encoder::find(codec::Id::None))?;
        output_stream.set_parameters(input_stream.parameters());
        unsafe {
            (*output_stream.parameters().as_mut_ptr()).codec_tag = 0;
        }
        let output_index = output_stream.index();
        stream_mapping.insert(stream.input_index, output_index);
        input_time_bases.insert(stream.input_index, input_stream.time_base());
        stream_metadata.insert(output_index, stream.clone());
    }

    if stream_mapping.is_empty() {
        return Ok(());
    }

    output.set_metadata(input.metadata().to_owned());
    for (output_index, stream) in stream_metadata {
        let Some(mut output_stream) = output.stream_mut(output_index) else {
            continue;
        };
        let mut metadata = output_stream.metadata().to_owned();
        if let Some(title) = stream.title {
            metadata.set("title", &title);
        }
        if let Some(language) = stream.language {
            metadata.set("language", &language);
        }
        if let Some(filename) = stream.filename {
            metadata.set("filename", &filename);
        }
        if let Some(mimetype) = stream.mimetype {
            metadata.set("mimetype", &mimetype);
        }
        output_stream.set_metadata(metadata);
    }

    output.write_header()?;
    for (stream, mut packet) in input.packets() {
        let input_index = stream.index();
        let Some(output_index) = stream_mapping.get(&input_index).copied() else {
            continue;
        };
        let Some(output_stream) = output.stream(output_index) else {
            continue;
        };
        let input_time_base = input_time_bases
            .get(&input_index)
            .copied()
            .unwrap_or(stream.time_base());
        packet.rescale_ts(input_time_base, output_stream.time_base());
        packet.set_position(-1);
        packet.set_stream(output_index);
        packet.write_interleaved(&mut output)?;
    }
    output.write_trailer()?;
    Ok(())
}
