use std::{env, io, path::PathBuf};

use ffmpeg_sdk_builder::is_valid_sdk;

#[cfg(not(feature = "build-from-source"))]
use ffmpeg_sdk_builder::ensure_prebuilt_sdk;

#[cfg(feature = "build-from-source")]
use ffmpeg_sdk_builder::{AudioCodec, Component, FFmpegBuilder, MuxerFormat, VideoCodec};

#[cfg(not(feature = "build-from-source"))]
const DEFAULT_SDK_TAG: &str = "sdk-v7.1.5";
#[cfg(not(feature = "build-from-source"))]
const DEFAULT_RELEASE_BASE_URL: &str =
    "https://github.com/darkskygit/ffmpeg-pipeline/releases/download";

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    for variable in ["FFMPEG_DIR", "FFMPEG_SDK_TAG", "FFMPEG_SDK_BASE_URL"] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    if let Some(directory) = env::var_os("FFMPEG_DIR").map(PathBuf::from) {
        validate_and_emit_sdk(&directory)?;
        return Ok(());
    }

    #[cfg(feature = "build-from-source")]
    return build_from_source();

    #[cfg(not(feature = "build-from-source"))]
    {
        let output_dir = PathBuf::from(required_env("OUT_DIR")?);
        let target = required_env("TARGET")?;
        let tag = env::var("FFMPEG_SDK_TAG").unwrap_or_else(|_| DEFAULT_SDK_TAG.into());
        let base_url =
            env::var("FFMPEG_SDK_BASE_URL").unwrap_or_else(|_| DEFAULT_RELEASE_BASE_URL.into());
        let directory = ensure_prebuilt_sdk(&output_dir, &target, &tag, &base_url, true)?;
        validate_and_emit_sdk(&directory)
    }
}

#[cfg(feature = "build-from-source")]
fn build_from_source() -> io::Result<()> {
    let output_dir = PathBuf::from(required_env("OUT_DIR")?);
    let output = FFmpegBuilder::new()
        .source_dir(output_dir.join("sources"))
        .build_dir(&output_dir)
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
        .verbose(true)
        .build()
        .compile()?;

    for directive in output {
        println!("{directive}");
    }
    Ok(())
}

fn validate_and_emit_sdk(directory: &std::path::Path) -> io::Result<()> {
    if !is_valid_sdk(directory) {
        return Err(io::Error::other(format!(
            "FFmpeg SDK directory is invalid: {}",
            directory.display()
        )));
    }

    let library_dir = directory.join("lib");
    println!("cargo:rustc-link-search=native={}", library_dir.display());
    for library in ["aom", "opus", "z"] {
        if has_library(&library_dir, library) {
            println!("cargo:rustc-link-lib=static={library}");
        }
    }

    match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => {
            for library in ["z", "bz2", "iconv"] {
                println!("cargo:rustc-link-lib={library}");
            }
        }
        Ok("windows") => {
            for library in ["bcrypt", "ole32", "secur32", "user32", "ws2_32"] {
                println!("cargo:rustc-link-lib={library}");
            }
        }
        _ => {
            println!("cargo:rustc-link-lib=z");
            println!("cargo:rustc-link-lib=bz2");
        }
    }

    println!("cargo:library={}", directory.display());
    Ok(())
}

fn has_library(directory: &std::path::Path, name: &str) -> bool {
    [
        format!("lib{name}.a"),
        format!("{name}.lib"),
        format!("lib{name}.lib"),
    ]
    .iter()
    .any(|file| directory.join(file).is_file())
}

fn required_env(name: &str) -> io::Result<String> {
    env::var(name).map_err(|_| io::Error::other(format!("Cargo did not provide {name}")))
}
