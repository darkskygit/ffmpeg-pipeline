use std::{env, io, path::PathBuf};

use ffmpeg_sdk_builder::is_valid_sdk;

#[cfg(not(feature = "build-from-source"))]
use ffmpeg_sdk_builder::ensure_prebuilt_sdk;

#[cfg(feature = "build-from-source")]
use ffmpeg_sdk_builder::pipeline_sdk_builder;

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
    pipeline_sdk_builder(output_dir.join("sources"), &output_dir)
        .verbose(true)
        .build()
        .compile()?;
    validate_and_emit_sdk(&output_dir.join("ffmpeg_build"))
}

fn validate_and_emit_sdk(directory: &std::path::Path) -> io::Result<()> {
    if !is_valid_sdk(directory) {
        return Err(io::Error::other(format!(
            "FFmpeg SDK directory is invalid: {}",
            directory.display()
        )));
    }

    println!("cargo:library={}", directory.display());
    Ok(())
}

fn required_env(name: &str) -> io::Result<String> {
    env::var(name).map_err(|_| io::Error::other(format!("Cargo did not provide {name}")))
}
