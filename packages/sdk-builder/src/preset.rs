use std::path::Path;

use crate::{AudioCodec, Component, FFmpegBuilder, MuxerFormat, VideoCodec};

pub fn pipeline_sdk_builder(
    source_dir: impl AsRef<Path>,
    build_root: impl AsRef<Path>,
) -> FFmpegBuilder {
    FFmpegBuilder::new()
        .source_dir(source_dir)
        .build_dir(build_root)
        .with_component(Component::AOM)
        .with_component(Component::Opus)
        .with_component(Component::ZLib)
        .with_video_codecs([VideoCodec::H264, VideoCodec::HEVC, VideoCodec::AV1])
        .with_audio_codecs([
            AudioCodec::Opus,
            AudioCodec::AAC,
            AudioCodec::MP3,
            AudioCodec::FLAC,
        ])
        .with_muxer_formats([
            MuxerFormat::Matroska,
            MuxerFormat::MP4,
            MuxerFormat::Ogg,
            MuxerFormat::MOV,
        ])
        .enable_hwaccel(false)
        .use_cache(true)
}
