#!/bin/sh
#![allow(unused_attributes)] /*
                             OUT=/tmp/tmp && rustc "$0" -o ${OUT} && exec ${OUT} $@ || exit $? #*/

use std::fs;
use std::io::Result;
use std::path::PathBuf;
use std::process::Command;

fn mkdir(dir_name: &str) -> Result<()> {
    fs::create_dir(dir_name)
}

fn pwd() -> Result<PathBuf> {
    std::env::current_dir()
}

fn cd(dir_name: &str) -> Result<()> {
    std::env::set_current_dir(dir_name)
}

fn main() -> Result<()> {
    let _ = mkdir("tmp");

    cd("tmp")?;

    let num_job = std::thread::available_parallelism().unwrap().get();
    let tmp_path = pwd()?.to_string_lossy().to_string();
    let build_path = format!("{}/ffmpeg_build", tmp_path);

    {
        if fs::metadata("aom").is_err() {
            Command::new("git")
                .arg("clone")
                .arg("--single-branch")
                .arg("--depth")
                .arg("1")
                .arg("https://aomedia.googlesource.com/aom")
                .status()?;
        }

        cd("aom")?;

        // 412efe2 is the version of aom that is using in ffmpeg 4.2.2
        Command::new("git")
            .arg("fetch")
            .arg("origin")
            .arg("412efe2")
            .arg("--depth")
            .arg("1")
            .status()?;

        Command::new("git")
            .arg("checkout")
            .arg("412efe2")
            .status()?;

        cd("..")?;

        if fs::metadata("aom_build").is_err() {
            mkdir("aom_build")?;
        }
        cd("aom_build")?;

        Command::new("cmake")
            .arg("-G")
            .arg("Unix Makefiles")
            .arg(format!("-DCMAKE_INSTALL_PREFIX={}", build_path))
            .arg("-DBUILD_SHARED_LIBS=OFF")
            .arg("-DENABLE_DOCS=OFF")
            .arg("-DENABLE_EXAMPLES=OFF")
            .arg("-DENABLE_TESTS=OFF")
            .arg("-DENABLE_TESTDATA=OFF")
            .arg("-DENABLE_TOOLS=OFF")
            .arg("-DENABLE_NASM=on")
            .arg("../aom")
            .status()?;

        Command::new("make")
            .arg("-j")
            .arg(num_job.to_string())
            .status()?;

        Command::new("make").arg("install").status()?;

        cd("..")?;
    }

    {
        if fs::metadata("opus-1.5.2.tar.gz").is_err() {
            // should follow redirect
            Command::new("curl")
                .arg("-L")
                .arg("-o")
                .arg("opus-1.5.2.tar.gz")
                .arg("https://downloads.xiph.org/releases/opus/opus-1.5.2.tar.gz")
                .status()?;
        }

        if fs::metadata("opus-1.5.2").is_err() {
            Command::new("tar")
                .arg("-xf")
                .arg("opus-1.5.2.tar.gz")
                .status()?;
        }

        if fs::metadata("opus_build").is_err() {
            mkdir("opus_build")?;
        }
        cd("opus_build")?;

        Command::new("cmake")
            .arg("-G")
            .arg("Unix Makefiles")
            .arg(format!("-DCMAKE_INSTALL_PREFIX={}", build_path))
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg("-DBUILD_SHARED_LIBS=OFF")
            .arg("-DOPUS_OSCE=ON")
            .arg("-DOPUS_STATIC_RUNTIME=ON")
            .arg("../opus-1.5.2")
            .status()?;

        Command::new("make")
            .arg("-j")
            .arg(num_job.to_string())
            .status()?;

        Command::new("make").arg("install").status()?;

        cd("..")?;
    }

    {
        let branch = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "release/7.1".to_string());

        if fs::metadata("ffmpeg").is_err() {
            Command::new("git")
                .arg("clone")
                .arg("--single-branch")
                .arg("--branch")
                .arg(&branch)
                .arg("--depth")
                .arg("1")
                .arg("https://github.com/ffmpeg/ffmpeg")
                .status()?;
        }

        cd("ffmpeg")?;

        Command::new("git")
            .arg("fetch")
            .arg("origin")
            .arg(&branch)
            .arg("--depth")
            .arg("1")
            .status()?;

        Command::new("git")
            .arg("checkout")
            .arg("FETCH_HEAD")
            .status()?;

        Command::new("./configure")
            .arg(format!("--prefix={}", build_path))
            .arg("--disable-everything")
            .arg("--disable-programs")
            .arg("--disable-doc")
            .arg("--enable-gpl")
            // .arg("--enable-libass")
            // .arg("--enable-libfdk-aac")
            // .arg("--enable-libfreetype")
            // .arg("--enable-libmp3lame")
            .arg("--enable-libopus")
            // .arg("--enable-libvorbis")
            // .arg("--enable-libvpx")
            // .arg("--enable-libx264")
            // .arg("--enable-libx265")
            .arg("--enable-libaom")
            .arg("--enable-muxer=matroska,mp4,ogg,opus,mov")
            .arg("--enable-demuxer=matroska,mp4,ogg,opus,mov")
            // .arg("--enable-decoder=h264,hevc,av1,libaom_av1")
            .arg("--enable-decoder=h264,hevc,libaom_av1,png,aac")
            // .arg("--enable-encoder=h264_videotoolbox,hevc_videotoolbox,libx264,libx264rgb,libx265,libaom_av1")
            .arg("--enable-encoder=h264_videotoolbox,hevc_videotoolbox,libopus")
            .arg("--enable-parser=av1")
            .arg("--enable-zlib")
            .arg("--enable-protocol=file,data,pipe")
            .arg("--enable-hwaccel=h264_videotoolbox,hevc_videotoolbox")
            .arg("--enable-filter=anull,aresample")
            .arg("--enable-small")
            .arg("--extra-cflags=\"-stdlib=libc++\"")
            .arg("--extra-cxxflags=\"-stdlib=libc++\"")
            .status()?;

        Command::new("make")
            .arg("-j")
            .arg(num_job.to_string())
            .status()?;

        Command::new("make").arg("install").status()?;

        cd("..")?;
    }

    Ok(())
}
